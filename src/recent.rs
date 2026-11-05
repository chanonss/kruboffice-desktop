//! Recent files of all three programs, for the Start Center.
//!
//! Each program keeps its own list in `ui.json` (`"recent": [paths…]`, newest first) in its
//! settings folder. KrubOffice reads the three lists (never writes them), keeps the files that
//! still exist, and orders them by when each was last changed.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::dispatch::{App, app_for};

/// One recent file.
#[derive(Clone, Debug, PartialEq)]
pub struct Recent {
    pub path: PathBuf,
    pub app: App,
    pub modified: Option<SystemTime>,
}

/// The folder that holds every program's settings folder.
pub fn settings_root() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    }
}

/// A program's settings folder on this platform (lower-case on Linux, as the programs do).
fn settings_dir(root: &Path, app: App) -> PathBuf {
    if cfg!(any(target_os = "macos", windows)) { root.join(app.settings_folder()) } else { root.join(app.settings_folder().to_ascii_lowercase()) }
}

/// The largest `ui.json` read; a bigger one is ignored rather than loaded.
const MAX_SETTINGS_BYTES: u64 = 8 * 1024 * 1024;
/// Entries taken from each program's list.
const PER_APP: usize = 50;

/// The paths in one program's recent list, newest first. Anything unreadable gives an empty list.
pub fn read_list(ui_json: &Path) -> Vec<PathBuf> {
    let Ok(meta) = std::fs::metadata(ui_json) else { return Vec::new() };
    if meta.len() > MAX_SETTINGS_BYTES {
        return Vec::new();
    }
    let Ok(bytes) = std::fs::read(ui_json) else { return Vec::new() };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) else { return Vec::new() };
    v.get("recent")
        .and_then(|r| r.as_array())
        .map(|a| a.iter().filter_map(|p| p.as_str()).filter(|p| !p.is_empty()).take(PER_APP).map(PathBuf::from).collect())
        .unwrap_or_default()
}

/// Recent files of all three programs under `root`, newest first, at most `limit`. A file that
/// no longer exists, or that the program listing it isn't the one KrubOffice would open it with,
/// is left out.
pub fn collect(root: &Path, limit: usize) -> Vec<Recent> {
    let mut out: Vec<Recent> = Vec::new();
    for app in App::ALL {
        for path in read_list(&settings_dir(root, app).join("ui.json")) {
            if app_for(&path) != Some(app) || out.iter().any(|r| r.path == path) {
                continue;
            }
            let Ok(meta) = std::fs::metadata(&path) else { continue };
            if !meta.is_file() {
                continue;
            }
            out.push(Recent { path, app, modified: meta.modified().ok() });
        }
    }
    out.sort_by_key(|r| std::cmp::Reverse(r.modified));
    out.truncate(limit);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_list(root: &Path, app: App, paths: &[&Path]) {
        let dir = settings_dir(root, app);
        std::fs::create_dir_all(&dir).unwrap();
        let list: Vec<String> = paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
        std::fs::write(dir.join("ui.json"), serde_json::json!({"recent": list, "zoom": 1.0}).to_string()).unwrap();
    }

    #[test]
    fn lists_of_all_three_are_merged_newest_first() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("settings");
        let files = tmp.path().join("files");
        std::fs::create_dir_all(&files).unwrap();
        let doc = files.join("รายงาน.docx");
        let sheet = files.join("งบ.xlsx");
        let deck = files.join("talk.pptx");
        for (i, f) in [&doc, &sheet, &deck].into_iter().enumerate() {
            std::fs::write(f, b"x").unwrap();
            let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000 + i as u64 * 60);
            std::fs::File::options().write(true).open(f).unwrap().set_modified(t).unwrap();
        }
        let gone = files.join("gone.docx");
        write_list(&root, App::KhianKrub, &[&doc, &gone]);
        write_list(&root, App::KhitKrub, &[&sheet, &doc]);
        write_list(&root, App::PhutKrub, &[&deck]);
        let r = collect(&root, 10);
        let names: Vec<_> = r.iter().map(|r| r.path.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, ["talk.pptx", "งบ.xlsx", "รายงาน.docx"], "newest first; missing and misplaced files dropped");
        assert_eq!(r.first().map(|r| r.app), Some(App::PhutKrub));
        assert_eq!(collect(&root, 1).len(), 1);
    }

    #[test]
    fn broken_settings_give_no_recent_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = settings_dir(tmp.path(), App::KhitKrub);
        std::fs::create_dir_all(&dir).unwrap();
        for junk in [&b"not json"[..], b"{\"recent\": 5}", b"{\"recent\": [1, null, \"\"]}", b"[]"] {
            std::fs::write(dir.join("ui.json"), junk).unwrap();
            assert!(read_list(&dir.join("ui.json")).is_empty());
        }
        assert!(collect(tmp.path(), 10).is_empty());
        assert!(read_list(&tmp.path().join("missing.json")).is_empty());
    }
}
