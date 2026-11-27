//! Portable mode (Windows, the portable zip): with a file named `portable` next to the program,
//! every setting, log, dictionary word and recovery copy lives in `data\PhutKrub` beside it
//! instead of `%APPDATA%\PhutKrub`, so the app runs from a USB stick and leaves nothing on the
//! computer. Each KrubOffice program has its own folder under `data`, since the KrubOffice
//! portable zip puts all three in one folder (the Start Center reads them there). The installed
//! version has no marker and is unchanged.

use std::path::{Path, PathBuf};

/// The file whose presence next to the program turns on portable mode.
pub const MARKER: &str = "portable";

/// The `data\PhutKrub` folder beside the program in portable mode; `None` otherwise (always `None` off
/// Windows).
pub fn data_dir() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    data_dir_beside(exe.parent()?)
}

fn data_dir_beside(program_dir: &Path) -> Option<PathBuf> {
    program_dir.join(MARKER).is_file().then(|| program_dir.join("data").join("PhutKrub"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_marker_file_beside_the_program() {
        let root = std::env::temp_dir().join(format!("phutkrub-portable-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(MARKER)).unwrap();
        // A folder named `portable` is not the marker.
        assert_eq!(data_dir_beside(&root), None);
        std::fs::remove_dir_all(root.join(MARKER)).unwrap();
        std::fs::write(root.join(MARKER), "").unwrap();
        assert_eq!(data_dir_beside(&root), Some(root.join("data").join("PhutKrub")));
        let _ = std::fs::remove_dir_all(&root);
        if !cfg!(windows) {
            assert_eq!(data_dir(), None);
        }
    }
}
