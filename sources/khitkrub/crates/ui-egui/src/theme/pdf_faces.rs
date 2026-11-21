//! Thai faces for the PDF writer on the desktop, found on this computer.
//!
//! Release builds carry no craft-fonts faces, so the desktop has no Thai font of its own to embed:
//! its screen draws Thai with the system's faces. Before a PDF is made, [`prepare`] looks for
//! TrueType faces the writer can embed: Sarabun in the KrubOffice bundle's `fonts` folder, the
//! installed `TH Sarabun New` and `Sarabun`, and any other installed Thai face. Without one, Thai
//! in an exported PDF showed as Chinese characters (the writer's predefined CID font).

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// Four styles: regular, bold, italic, bold italic.
pub(super) type Styles = [Option<Arc<[u8]>>; 4];

/// A Thai fallback face: regular and bold.
pub(super) type Pair = (Option<Arc<[u8]>>, Option<Arc<[u8]>>);

/// The faces found, read once.
#[derive(Clone, Default)]
pub(super) struct NativeFaces {
    pub th_sarabun_new: Styles,
    pub sarabun: Styles,
    /// Another installed Thai face (regular, bold), for when neither Sarabun is there.
    pub system_thai: Pair,
}

static FOUND: OnceLock<NativeFaces> = OnceLock::new();

/// The faces [`prepare`] found, if it has run.
pub(super) fn found() -> Option<NativeFaces> {
    FOUND.get().cloned()
}

/// Finds the faces (once; it waits for the installed-font scan) and hands them to the PDF writer.
/// A no-op on the web, whose page loads its own faces.
pub fn prepare() {
    if cfg!(target_arch = "wasm32") {
        return;
    }
    if FOUND.get().is_none() {
        let _ = FOUND.set(discover());
        super::font_setup::sync_pdf_faces();
    }
}

/// Families tried, in order, for the Thai fallback after Sarabun.
const THAI_FAMILIES: &[&str] =
    &["Noto Sans Thai", "Noto Sans Thai UI", "Leelawadee UI", "Leelawadee", "Ayuthaya", "Silom", "Garuda", "Loma", "Tahoma", "TH Sarabun New"];

/// The largest font file read.
const MAX_FONT_BYTES: u64 = 32 << 20;

fn discover() -> NativeFaces {
    let catalog = crate::fonts::catalog();
    let mut sarabun = styles_of(catalog, "Sarabun");
    // KrubOffice ships Sarabun beside its programs, for computers that have no Thai font.
    if sarabun[0].is_none() {
        sarabun[0] = bundled("Sarabun-Regular.ttf");
    }
    if sarabun[1].is_none() {
        sarabun[1] = bundled("Sarabun-SemiBold.ttf");
    }
    let mut system_thai = (None, None);
    if sarabun[0].is_none() {
        for family in THAI_FAMILIES {
            let styles = styles_of(catalog, family);
            if styles[0].as_deref().is_some_and(has_thai) {
                system_thai = (styles[0].clone(), styles[1].clone());
                break;
            }
        }
    }
    if system_thai.0.is_none() && sarabun[0].is_none() {
        // Any installed face with Thai letters.
        system_thai.0 = catalog.iter().filter(|f| f.index == 0).find_map(|f| read(&f.path).filter(|b| has_thai(b)));
    }
    NativeFaces { th_sarabun_new: styles_of(catalog, "TH Sarabun New"), sarabun, system_thai }
}

/// The installed family's TrueType files for each style, nearest weight first. Faces inside a
/// collection are skipped: the writer embeds whole single-face files.
fn styles_of(catalog: &[crate::fonts::Face], family: &str) -> Styles {
    let faces: Vec<&crate::fonts::Face> =
        catalog.iter().filter(|f| f.index == 0 && (f.family.eq_ignore_ascii_case(family) || f.answers_to(family))).collect();
    let pick = |bold: bool, italic: bool| -> Option<Arc<[u8]>> {
        let target = if bold { 700.0 } else { 400.0 };
        let mut matching: Vec<&&crate::fonts::Face> = faces.iter().filter(|f| f.italic == italic && (f.weight >= 600.0) == bold).collect();
        matching.sort_by(|a, b| (a.weight - target).abs().total_cmp(&(b.weight - target).abs()));
        matching.iter().find_map(|f| read(&f.path))
    };
    [pick(false, false), pick(true, false), pick(false, true), pick(true, true)]
}

fn read(path: &Path) -> Option<Arc<[u8]>> {
    if std::fs::metadata(path).ok()?.len() > MAX_FONT_BYTES {
        return None;
    }
    std::fs::read(path).ok().map(Arc::from)
}

/// Whether the font maps the Thai letter ก.
fn has_thai(bytes: &[u8]) -> bool {
    use skrifa::MetadataProvider as _;
    skrifa::FontRef::new(bytes).is_ok_and(|f| f.charmap().map('ก').is_some())
}

/// The folders where packaging puts Sarabun: `fonts` beside the program (Windows, Linux) or the
/// KrubOffice bundle's `Contents/Resources/fonts` (macOS, where KhitKrub is a helper app inside).
pub(super) fn bundled_dirs(exe: &Path) -> Vec<PathBuf> {
    exe.ancestors().skip(1).take(5).flat_map(|dir| [dir.join("fonts"), dir.join("Resources").join("fonts")]).collect()
}

fn bundled(file: &str) -> Option<Arc<[u8]>> {
    let exe = std::env::current_exe().ok()?;
    bundled_dirs(&exe).iter().map(|d| d.join(file)).filter(|p| p.is_file()).find_map(|p| read(&p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_krub_office_bundle_folder_is_searched_from_a_helper_app() {
        let exe = Path::new("/Applications/KrubOffice.app/Contents/Helpers/KhitKrub.app/Contents/MacOS/khitkrub");
        assert!(bundled_dirs(exe).contains(&PathBuf::from("/Applications/KrubOffice.app/Contents/Resources/fonts")));
        let exe = Path::new("/usr/lib/kruboffice/khitkrub");
        assert!(bundled_dirs(exe).contains(&PathBuf::from("/usr/lib/kruboffice/fonts")));
    }

    #[test]
    fn garbage_is_not_a_thai_font() {
        assert!(!has_thai(b"not a font"));
        assert!(!has_thai(&[]));
    }
}
