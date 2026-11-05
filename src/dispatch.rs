//! Which app opens a file, where that app is, and how to hand it the file.
//!
//! KrubOffice is one app to the person using it, with three programs inside: KhianKrub (documents),
//! KhitKrub (spreadsheets) and PhutKrub (presentations). Each is built from its own web repository
//! and installed next to this launcher:
//!
//! - macOS: `KrubOffice.app/Contents/Helpers/{KhianKrub,KhitKrub,PhutKrub}.app`. Files go through
//!   LaunchServices (`open -a`), so a program that is already running gets the file in its open
//!   window instead of a second copy starting.
//! - Windows and others: `khiankrub.exe`, `khitkrub.exe`, `phutkrub.exe` in the launcher's folder,
//!   started with the file's path.
//!
//! `KRUBOFFICE_APPS_DIR` points at another folder of programs (development builds).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The three programs inside KrubOffice.
#[allow(clippy::enum_variant_names)] // they are the programs' names
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum App {
    KhianKrub,
    KhitKrub,
    PhutKrub,
}

impl App {
    pub const ALL: [App; 3] = [App::KhianKrub, App::KhitKrub, App::PhutKrub];

    /// The name people see, in Latin letters (also the bundle and settings folder name).
    pub fn name(self) -> &'static str {
        match self {
            App::KhianKrub => "KhianKrub",
            App::KhitKrub => "KhitKrub",
            App::PhutKrub => "PhutKrub",
        }
    }

    /// The Thai name, as on kruboffice.app.
    pub fn thai_name(self) -> &'static str {
        match self {
            App::KhianKrub => "เขียนครับ",
            App::KhitKrub => "คิดครับ",
            App::PhutKrub => "พูดครับ",
        }
    }

    /// What the program is for, in Thai.
    pub fn thai_kind(self) -> &'static str {
        match self {
            App::KhianKrub => "เอกสาร",
            App::KhitKrub => "ตารางคำนวณ",
            App::PhutKrub => "งานนำเสนอ",
        }
    }

    /// The file extensions this program opens and KrubOffice registers for it. An extension
    /// appears under one program only; ambiguous ones go to the program that handles them best
    /// (`.txt`, `.rtf`, `.html` to KhianKrub; `.csv` and `.tsv` to KhitKrub).
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            App::KhianKrub => {
                &["docx", "docm", "dotx", "dotm", "doc", "dot", "rtf", "odt", "txt", "md", "markdown", "html", "htm", "tex", "latex", "ltx"]
            }
            App::KhitKrub => &["xlsx", "xlsm", "xltx", "xltm", "xls", "xlt", "xlsb", "ods", "csv", "tsv", "slk"],
            App::PhutKrub => &["pptx", "potx", "ppsx", "deckcraft"],
        }
    }

    /// The folder name of this program's settings, which hold its recent files (`ui.json`).
    /// KhianKrub keeps WordCraft's folder so settings from KhianKrub 0.9 carry over.
    pub fn settings_folder(self) -> &'static str {
        match self {
            App::KhianKrub => "WordCraft",
            App::KhitKrub => "KhitKrub",
            App::PhutKrub => "PhutKrub",
        }
    }

    /// The executable's file name on Windows and Linux.
    fn exe_name(self) -> String {
        let base = self.name().to_ascii_lowercase();
        if cfg!(windows) { format!("{base}.exe") } else { base }
    }
}

/// The program that opens `path`, from its extension (case-insensitive). `None` for a file
/// KrubOffice doesn't handle.
pub fn app_for(path: &Path) -> Option<App> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    App::ALL.into_iter().find(|a| a.extensions().contains(&ext.as_str()))
}

/// Where the three programs are installed.
#[derive(Clone, Debug)]
pub struct Installation {
    dir: PathBuf,
    /// macOS app bundles (`Name.app`) rather than plain executables.
    bundles: bool,
}

impl Installation {
    /// The programs next to this launcher, or in `KRUBOFFICE_APPS_DIR`.
    pub fn locate() -> Option<Installation> {
        if let Some(dir) = std::env::var_os("KRUBOFFICE_APPS_DIR").filter(|d| !d.is_empty()) {
            let dir = PathBuf::from(dir);
            let bundles = App::ALL.iter().any(|a| dir.join(format!("{}.app", a.name())).is_dir());
            return Some(Installation { dir, bundles });
        }
        let exe = std::env::current_exe().ok()?;
        let here = exe.parent()?.to_path_buf();
        // KrubOffice.app/Contents/MacOS/kruboffice → KrubOffice.app/Contents/Helpers
        if cfg!(target_os = "macos") && here.ends_with("Contents/MacOS") {
            let helpers = here.parent()?.join("Helpers");
            return Some(Installation { dir: helpers, bundles: true });
        }
        Some(Installation { dir: here, bundles: false })
    }

    /// An installation in `dir` (tests).
    #[cfg(test)]
    pub fn at(dir: impl Into<PathBuf>, bundles: bool) -> Installation {
        Installation { dir: dir.into(), bundles }
    }

    /// The program's bundle or executable.
    pub fn path_of(&self, app: App) -> PathBuf {
        if self.bundles { self.dir.join(format!("{}.app", app.name())) } else { self.dir.join(app.exe_name()) }
    }

    /// Whether the program is there.
    pub fn has(&self, app: App) -> bool {
        let p = self.path_of(app);
        if self.bundles { p.is_dir() } else { p.is_file() }
    }

    /// The command that starts `app`, opening `files` (none: its start screen).
    pub fn command(&self, app: App, files: &[PathBuf]) -> Command {
        let target = self.path_of(app);
        if self.bundles {
            // `open -a` goes through LaunchServices: a running program gets the files as an
            // open-documents event in the window it already has.
            let mut c = Command::new("/usr/bin/open");
            c.arg("-a").arg(&target);
            c.args(files);
            c
        } else {
            let mut c = Command::new(&target);
            c.args(files);
            c
        }
    }

    /// Start `app` with `files`. The launcher doesn't wait for it.
    pub fn launch(&self, app: App, files: &[PathBuf]) -> Result<(), String> {
        if !self.has(app) {
            return Err(format!("{} is missing from this installation ({})", app.name(), self.path_of(app).display()));
        }
        self.command(app, files).spawn().map(|_| ()).map_err(|e| format!("couldn't start {}: {e}", app.name()))
    }
}

/// Files grouped by the program that opens them, in the order given; the files no program opens
/// are returned separately.
pub fn group(files: impl IntoIterator<Item = PathBuf>) -> (Vec<(App, Vec<PathBuf>)>, Vec<PathBuf>) {
    let mut groups: Vec<(App, Vec<PathBuf>)> = Vec::new();
    let mut unknown = Vec::new();
    for f in files {
        match app_for(&f) {
            Some(app) => match groups.iter_mut().find(|(a, _)| *a == app) {
                Some((_, list)) => list.push(f),
                None => groups.push((app, vec![f])),
            },
            None => unknown.push(f),
        }
    }
    (groups, unknown)
}

/// The file arguments on a command line: everything that isn't a flag. (Windows and Linux pass
/// the files to open this way; macOS sends an open-documents event instead.) At most 256, so a
/// hostile command line can't make the launcher start thousands of programs.
pub fn file_args(args: impl IntoIterator<Item = OsString>) -> Vec<PathBuf> {
    args.into_iter().filter(|a| !a.to_string_lossy().starts_with("--")).take(256).map(PathBuf::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The installers read packaging/types.txt (`kruboffice --list-types`); it must match this table.
    #[test]
    fn packaging_types_match_the_table() {
        let expected: String = App::ALL.iter().map(|a| format!("{}\t{}\n", a.name(), a.extensions().join(","))).collect();
        assert_eq!(include_str!("../packaging/types.txt"), expected, "run `kruboffice --list-types > packaging/types.txt`");
    }

    #[test]
    fn each_extension_belongs_to_one_program() {
        let mut seen = std::collections::HashMap::new();
        for app in App::ALL {
            for ext in app.extensions() {
                assert_eq!(seen.insert(*ext, app), None, "{ext} is listed twice");
                assert_eq!(*ext, ext.to_ascii_lowercase());
            }
        }
    }

    #[test]
    fn files_go_to_the_right_program() {
        assert_eq!(app_for(Path::new("/x/รายงาน.DOCX")), Some(App::KhianKrub));
        assert_eq!(app_for(Path::new("C:\\งบ\\budget.xlsx")), Some(App::KhitKrub));
        assert_eq!(app_for(Path::new("data.csv")), Some(App::KhitKrub));
        assert_eq!(app_for(Path::new("talk.pptx")), Some(App::PhutKrub));
        assert_eq!(app_for(Path::new("notes.txt")), Some(App::KhianKrub));
        assert_eq!(app_for(Path::new("photo.jpg")), None);
        assert_eq!(app_for(Path::new("no-extension")), None);
        assert_eq!(app_for(Path::new(".docx")), None, "a hidden file named .docx has no extension");
    }

    #[test]
    fn grouping_keeps_order_and_reports_the_rest() {
        let files = ["a.docx", "b.xlsx", "c.docx", "d.zip", "e.pptx"].map(PathBuf::from);
        let (groups, unknown) = group(files);
        let apps: Vec<App> = groups.iter().map(|(a, _)| *a).collect();
        assert_eq!(apps, [App::KhianKrub, App::KhitKrub, App::PhutKrub]);
        assert_eq!(groups.first().map(|(_, f)| f.len()), Some(2));
        assert_eq!(unknown, [PathBuf::from("d.zip")]);
    }

    #[test]
    fn flags_are_not_files_and_the_count_is_capped() {
        let args = ["--start", "a.docx", "b.xlsx"].map(OsString::from);
        assert_eq!(file_args(args), [PathBuf::from("a.docx"), PathBuf::from("b.xlsx")]);
        let many = (0..10_000).map(|i| OsString::from(format!("{i}.docx")));
        assert_eq!(file_args(many).len(), 256);
    }

    #[test]
    fn programs_are_found_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let exe = Installation::at(dir.path(), false);
        assert!(!exe.has(App::KhitKrub));
        std::fs::write(exe.path_of(App::KhitKrub), b"").unwrap();
        assert!(exe.has(App::KhitKrub));
        assert!(exe.launch(App::PhutKrub, &[]).is_err(), "a missing program is an error, not a crash");
        let bundles = Installation::at(dir.path(), true);
        std::fs::create_dir(bundles.path_of(App::KhianKrub)).unwrap();
        assert!(bundles.has(App::KhianKrub));
        assert!(bundles.path_of(App::KhianKrub).ends_with("KhianKrub.app"));
        let c = bundles.command(App::KhianKrub, &[PathBuf::from("/tmp/a.docx")]);
        assert_eq!(c.get_program(), "/usr/bin/open");
        let args: Vec<_> = c.get_args().collect();
        assert_eq!(args.first().map(|a| a.to_string_lossy().into_owned()), Some("-a".into()));
        assert_eq!(args.last().map(|a| a.to_string_lossy().into_owned()), Some("/tmp/a.docx".into()));
    }
}
