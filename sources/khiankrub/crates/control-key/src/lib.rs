//! The control channel's key file, one rule for the app (which writes it) and the MCP bridge
//! (which reads it): `<settings>/control-key.<instance>`. See `docs/control-protocol.md`, Keys.

use std::path::{Path, PathBuf};

/// The app's settings folder (preferences, logs, key files): macOS
/// `~/Library/Application Support/KhianKrub`, Windows `%APPDATA%\KhianKrub`, otherwise
/// `$XDG_CONFIG_HOME/khiankrub` or `~/.config/khiankrub`.
/// `WORDCRAFT_PROFILE_DIR` overrides the entire folder for isolated runs, and portable mode
/// ([`portable_dir`]) keeps it next to the program.
///
/// KhianKrub: up to 0.19 this was WordCraft's own folder, so with WordCraft installed too both
/// programs read and wrote the same preferences (recent files, custom ribbons, options).
/// [`migrate_from_wordcraft`] copies that folder once.
pub fn settings_dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("WORDCRAFT_PROFILE_DIR").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(path));
    }
    if let Some(dir) = portable_dir() {
        return Some(dir);
    }
    named_dir("KhianKrub", "khiankrub")
}

/// The file whose presence next to the program turns on portable mode.
pub const PORTABLE_MARKER: &str = "portable";

/// Portable mode (Windows, the portable zip): with a file named `portable` next to
/// `KhianKrub.exe`, every setting, log, dictionary word and recovery copy lives in
/// `data\KhianKrub` beside the program instead of `%APPDATA%`, so the app runs from a USB stick
/// and leaves nothing on the computer. Each KrubOffice program has its own folder under `data`,
/// since the KrubOffice portable zip puts all three in one folder (the Start Center reads them
/// there). The installed version has no marker and is unchanged.
pub fn portable_dir() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    portable_dir_beside(exe.parent()?)
}

fn portable_dir_beside(program_dir: &Path) -> Option<PathBuf> {
    program_dir.join(PORTABLE_MARKER).is_file().then(|| program_dir.join("data").join("KhianKrub"))
}

/// WordCraft's settings folder, which KhianKrub shared until 0.19.
pub fn wordcraft_dir() -> Option<PathBuf> {
    named_dir("WordCraft", "wordcraft")
}

fn named_dir(name: &str, unix_name: &str) -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support").join(name))
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join(name))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|c| c.join(unix_name))
    }
}

/// First start after the move: copy WordCraft's settings folder to KhianKrub's so the user keeps
/// their recent files, ribbons, options, dictionary words and recovery copies. Only when
/// KhianKrub's folder doesn't exist yet and no profile override is set; WordCraft's folder is left
/// as it is. Returns the source folder when it copied.
pub fn migrate_from_wordcraft() -> std::io::Result<Option<PathBuf>> {
    // Portable mode never copies the computer's settings onto the stick.
    if std::env::var_os("WORDCRAFT_PROFILE_DIR").is_some_and(|p| !p.is_empty()) || portable_dir().is_some() {
        return Ok(None);
    }
    let (Some(to), Some(from)) = (settings_dir(), wordcraft_dir()) else { return Ok(None) };
    migrate(&from, &to)
}

fn migrate(from: &Path, to: &Path) -> std::io::Result<Option<PathBuf>> {
    if to.exists() || !from.is_dir() {
        return Ok(None);
    }
    // Copied next to the target and renamed, so a failed copy leaves no half folder that would
    // stop the next try.
    let tmp = to.with_file_name(format!(".{}-migrating", to.file_name().and_then(|n| n.to_str()).unwrap_or("settings")));
    let _ = std::fs::remove_dir_all(&tmp);
    let result = copy_dir(from, &tmp, true).and_then(|()| std::fs::rename(&tmp, to));
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&tmp);
    }
    result.map(|()| Some(from.to_path_buf()))
}

/// Copies `from` into `to`. At the top level it skips WordCraft's logs and control-port keys,
/// which belong to the running WordCraft.
fn copy_dir(from: &Path, to: &Path, top: bool) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        let n = name.to_string_lossy();
        if top && (n == "logs" || n.starts_with("control-key.")) {
            continue;
        }
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir(&entry.path(), &to.join(&name), false)?;
        } else if ty.is_file() {
            std::fs::copy(entry.path(), to.join(&name))?;
        }
    }
    Ok(())
}

/// The `instance-id=` value of a `/.flatpak-info` file, only when it is `[A-Za-z0-9_-]+` (it
/// becomes part of a file name).
pub fn parse_instance_id(info: &str) -> Option<String> {
    let v = info.lines().find_map(|l| l.strip_prefix("instance-id="))?.trim();
    (!v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')).then(|| v.to_string())
}

/// The Flatpak instance id, or `None` outside Flatpak.
pub fn instance_id() -> Option<String> {
    parse_instance_id(&std::fs::read_to_string("/.flatpak-info").ok()?)
}

/// `<settings>/control-key.<instance>`, with `<instance>` the Flatpak instance id or
/// `local-<port>`.
pub fn key_path(settings: &Path, port: u16) -> PathBuf {
    key_path_for(settings, instance_id().as_deref(), port)
}

fn key_path_for(settings: &Path, instance: Option<&str>, port: u16) -> PathBuf {
    match instance {
        Some(id) => settings.join(format!("control-key.{id}")),
        None => settings.join(format!("control-key.local-{port}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn instance_id_only_safe_values() {
        assert_eq!(parse_instance_id("[Instance]\ninstance-id=abc_123-X\n").as_deref(), Some("abc_123-X"));
        assert_eq!(parse_instance_id("instance-id= 42 \n").as_deref(), Some("42"));
        // It ends up in a file name.
        assert_eq!(parse_instance_id("instance-id=../etc\n"), None);
        assert_eq!(parse_instance_id("instance-id=a/b\n"), None);
        assert_eq!(parse_instance_id("instance-id=a b\n"), None);
        assert_eq!(parse_instance_id("instance-id=..\n"), None);
        assert_eq!(parse_instance_id("instance-id=\n"), None);
        assert_eq!(parse_instance_id("app=ai.storyteller.wordcraft\n"), None);
    }

    #[test]
    fn key_file_per_instance() {
        let dir = Path::new("/settings/wordcraft");
        assert_eq!(key_path_for(dir, Some("1234"), 7981), dir.join("control-key.1234"));
        assert_eq!(key_path_for(dir, None, 7981), dir.join("control-key.local-7981"));
        assert_eq!(key_path_for(dir, None, 7982), dir.join("control-key.local-7982"));
        if instance_id().is_none() {
            assert_eq!(key_path(dir, 7981), dir.join("control-key.local-7981"));
        }
    }

    #[test]
    fn settings_dir_is_the_apps_folder() {
        if let Some(d) = settings_dir() {
            let name = d.file_name().and_then(|n| n.to_str()).unwrap_or("");
            assert!(name == "khiankrub" || name == "KhianKrub", "{}", d.display());
        }
    }

    #[test]
    fn wordcraft_folder_is_not_ours() {
        if let (Some(ours), Some(theirs)) = (named_dir("KhianKrub", "khiankrub"), wordcraft_dir()) {
            assert_ne!(ours, theirs);
        }
    }

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("khiankrub-migrate-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn migration_copies_once_and_leaves_wordcraft_alone() {
        let root = scratch("copy");
        let (from, to) = (root.join("WordCraft"), root.join("KhianKrub"));
        std::fs::create_dir_all(from.join("recovery")).unwrap();
        std::fs::create_dir_all(from.join("logs")).unwrap();
        std::fs::write(from.join("ui.json"), "{}").unwrap();
        std::fs::write(from.join("recovery/a.json"), "x").unwrap();
        std::fs::write(from.join("logs/wordcraft.log"), "log").unwrap();
        std::fs::write(from.join("control-key.local-7981"), "k").unwrap();

        assert_eq!(migrate(&from, &to).unwrap(), Some(from.clone()));
        assert_eq!(std::fs::read_to_string(to.join("ui.json")).unwrap(), "{}");
        assert_eq!(std::fs::read_to_string(to.join("recovery/a.json")).unwrap(), "x");
        assert!(!to.join("logs").exists());
        assert!(!to.join("control-key.local-7981").exists());
        assert!(from.join("ui.json").exists() && from.join("logs/wordcraft.log").exists());
        assert!(!root.join(".KhianKrub-migrating").exists());

        // Our folder exists now: WordCraft's later changes stay WordCraft's.
        std::fs::write(from.join("ui.json"), "{\"changed\":1}").unwrap();
        assert_eq!(migrate(&from, &to).unwrap(), None);
        assert_eq!(std::fs::read_to_string(to.join("ui.json")).unwrap(), "{}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn portable_marker_keeps_settings_beside_the_program() {
        let root = scratch("portable");
        assert_eq!(portable_dir_beside(&root), None);
        // A folder named `portable` is not the marker.
        std::fs::create_dir_all(root.join(PORTABLE_MARKER)).unwrap();
        assert_eq!(portable_dir_beside(&root), None);
        std::fs::remove_dir_all(root.join(PORTABLE_MARKER)).unwrap();
        std::fs::write(root.join(PORTABLE_MARKER), "").unwrap();
        assert_eq!(portable_dir_beside(&root), Some(root.join("data").join("KhianKrub")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn portable_mode_is_windows_only() {
        if !cfg!(windows) {
            assert_eq!(portable_dir(), None);
        }
    }

    #[test]
    fn no_wordcraft_folder_no_migration() {
        let root = scratch("none");
        assert_eq!(migrate(&root.join("WordCraft"), &root.join("KhianKrub")).unwrap(), None);
        assert!(!root.join("KhianKrub").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
