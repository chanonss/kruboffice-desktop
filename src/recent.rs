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

/// The folder that holds every program's settings folder (and KrubOffice's own).
pub fn settings_root() -> Option<PathBuf> {
    if let Some(dir) = portable_root() {
        return Some(dir);
    }
    if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    }
}

/// Portable mode (Windows, the portable zip): with a file named `portable` next to
/// `kruboffice.exe`, the three programs beside it keep their settings in `data\KhianKrub`,
/// `data\KhitKrub` and `data\PhutKrub`, and KrubOffice its own in `data\KrubOffice`, instead of
/// `%APPDATA%`. The installed version has no marker and is unchanged.
pub fn portable_root() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    portable_root_beside(exe.parent()?)
}

fn portable_root_beside(program_dir: &Path) -> Option<PathBuf> {
    program_dir.join("portable").is_file().then(|| program_dir.join("data"))
}

/// A program's settings folder on this platform (lower-case on Linux, as the programs do): the
/// first of its folders that holds a `ui.json`, else the first.
pub fn settings_dir(root: &Path, app: App) -> PathBuf {
    let named = |f: &str| if cfg!(any(target_os = "macos", windows)) { root.join(f) } else { root.join(f.to_ascii_lowercase()) };
    let folders = app.settings_folders();
    folders.iter().map(|f| named(f)).find(|d| d.join("ui.json").is_file()).unwrap_or_else(|| named(folders.first().copied().unwrap_or_default()))
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

/// The most pinned files kept.
pub const MAX_PINS: usize = 200;

/// The files pinned on the Start Center, from KrubOffice's own `settings.json` (`"pinned":
/// [paths…]`). Anything unreadable gives none.
pub fn load_pins(settings: &Path) -> Vec<PathBuf> {
    let Ok(meta) = std::fs::metadata(settings) else { return Vec::new() };
    if meta.len() > MAX_SETTINGS_BYTES {
        return Vec::new();
    }
    let Ok(bytes) = std::fs::read(settings) else { return Vec::new() };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) else { return Vec::new() };
    let mut out: Vec<PathBuf> = Vec::new();
    for p in v.get("pinned").and_then(|r| r.as_array()).into_iter().flatten().filter_map(|p| p.as_str()) {
        let p = PathBuf::from(p);
        if !p.as_os_str().is_empty() && !out.contains(&p) && out.len() < MAX_PINS {
            out.push(p);
        }
    }
    out
}

/// Write the pinned files into `settings.json`, keeping its other keys (through a temporary
/// file, so a crash never leaves half a file).
pub fn save_pins(settings: &Path, pins: &[PathBuf]) -> Result<(), String> {
    let old = std::fs::metadata(settings)
        .ok()
        .filter(|m| m.len() <= MAX_SETTINGS_BYTES)
        .and_then(|_| std::fs::read(settings).ok())
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    let mut v = old;
    let list: Vec<serde_json::Value> = pins.iter().take(MAX_PINS).map(|p| serde_json::Value::String(p.to_string_lossy().into_owned())).collect();
    if let Some(map) = v.as_object_mut() {
        map.insert("pinned".into(), serde_json::Value::Array(list));
    }
    if let Some(dir) = settings.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let tmp = settings.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, settings).map_err(|e| format!("{}: {e}", settings.display()))
}

/// `recent` with the pinned files that still exist and that KrubOffice opens added (pinned
/// files stay on the Start Center however old they get), newest first.
pub fn with_pins(mut recent: Vec<Recent>, pins: &[PathBuf]) -> Vec<Recent> {
    for path in pins {
        if recent.iter().any(|r| &r.path == path) {
            continue;
        }
        let Some(app) = app_for(path) else { continue };
        let Ok(meta) = std::fs::metadata(path) else { continue };
        if meta.is_file() {
            recent.push(Recent { path: path.clone(), app, modified: meta.modified().ok() });
        }
    }
    recent.sort_by_key(|r| std::cmp::Reverse(r.modified));
    recent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_round_trip_and_keep_other_settings() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("KrubOffice").join("settings.json");
        assert!(load_pins(&settings).is_empty(), "no file, no pins");
        std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
        std::fs::write(&settings, r#"{"checkUpdates": true, "pinned": ["/a.docx", "/a.docx", 5, ""]}"#).unwrap();
        assert_eq!(load_pins(&settings), [PathBuf::from("/a.docx")], "duplicates, non-strings and blanks dropped");
        save_pins(&settings, &[PathBuf::from("/b.xlsx"), PathBuf::from("/c.pptx")]).unwrap();
        assert_eq!(load_pins(&settings), [PathBuf::from("/b.xlsx"), PathBuf::from("/c.pptx")]);
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&settings).unwrap()).unwrap();
        assert_eq!(v["checkUpdates"], true, "the update setting is kept");
        std::fs::write(&settings, b"not json").unwrap();
        assert!(load_pins(&settings).is_empty());
        save_pins(&settings, &[]).unwrap();
        assert!(load_pins(&settings).is_empty());
    }

    #[test]
    fn pinned_files_stay_listed_when_they_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let doc = tmp.path().join("old.docx");
        std::fs::write(&doc, b"x").unwrap();
        let other = tmp.path().join("notes.zzz");
        std::fs::write(&other, b"x").unwrap();
        let pins = [doc.clone(), tmp.path().join("gone.xlsx"), other];
        let list = with_pins(Vec::new(), &pins);
        assert_eq!(list.iter().map(|r| r.path.clone()).collect::<Vec<_>>(), std::slice::from_ref(&doc), "missing and unknown files left out");
        assert_eq!(with_pins(list.clone(), &pins).len(), 1, "a pinned file already listed isn't added twice");
    }

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
    fn khiankrub_is_read_from_its_own_folder_before_wordcraft_s() {
        let tmp = tempfile::tempdir().unwrap();
        let named = |f: &str| if cfg!(any(target_os = "macos", windows)) { tmp.path().join(f) } else { tmp.path().join(f.to_ascii_lowercase()) };
        assert_eq!(settings_dir(tmp.path(), App::KhianKrub), named("KhianKrub"), "neither exists yet");
        std::fs::create_dir_all(named("WordCraft")).unwrap();
        std::fs::write(named("WordCraft").join("ui.json"), "{}").unwrap();
        assert_eq!(settings_dir(tmp.path(), App::KhianKrub), named("WordCraft"), "only the old folder");
        std::fs::create_dir_all(named("KhianKrub")).unwrap();
        std::fs::write(named("KhianKrub").join("ui.json"), "{}").unwrap();
        assert_eq!(settings_dir(tmp.path(), App::KhianKrub), named("KhianKrub"), "KhianKrub 0.19 and later");
    }

    #[test]
    fn portable_marker_keeps_settings_beside_the_program() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(portable_root_beside(tmp.path()), None);
        // A folder named `portable` is not the marker.
        std::fs::create_dir_all(tmp.path().join("portable")).unwrap();
        assert_eq!(portable_root_beside(tmp.path()), None);
        std::fs::remove_dir_all(tmp.path().join("portable")).unwrap();
        std::fs::write(tmp.path().join("portable"), "").unwrap();
        assert_eq!(portable_root_beside(tmp.path()), Some(tmp.path().join("data")));
        if !cfg!(windows) {
            assert_eq!(portable_root(), None);
        }
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
