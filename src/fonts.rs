//! A Thai interface font for the Start Center.
//!
//! KrubOffice ships no font of its own in this repository (fonts live in storytold/craft-fonts).
//! The packaging step may put Sarabun, the apps' interface font, in the app's `fonts` folder;
//! otherwise the platform's own Thai face is used. egui's built-in fonts cover Latin text either
//! way, so with no Thai face at all the window still works, only Thai letters would be missing.

use std::path::{Path, PathBuf};

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

/// The largest font file loaded (a font collection can be big, a hostile file bigger).
const MAX_FONT_BYTES: u64 = 64 * 1024 * 1024;

/// Regular and bold Thai faces to try, in order: (path, face index in a collection).
/// Font files to try: (path, face index in a collection).
type Faces = Vec<(PathBuf, u32)>;

fn candidates(resources: Option<&Path>) -> (Faces, Faces) {
    let mut regular = Vec::new();
    let mut bold = Vec::new();
    if let Some(r) = resources {
        regular.push((r.join("fonts/Sarabun-Regular.ttf"), 0));
        bold.push((r.join("fonts/Sarabun-SemiBold.ttf"), 0));
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        for dir in [Some(PathBuf::from("/Library/Fonts")), home.as_ref().map(|h| h.join("Library/Fonts"))].into_iter().flatten() {
            regular.push((dir.join("Sarabun-Regular.ttf"), 0));
            bold.push((dir.join("Sarabun-SemiBold.ttf"), 0));
        }
        // Thonburi, the macOS interface face for Thai: regular is the first face, bold the second.
        regular.push((PathBuf::from("/System/Library/Fonts/ThonburiUI.ttc"), 0));
        bold.push((PathBuf::from("/System/Library/Fonts/ThonburiUI.ttc"), 1));
        regular.push((PathBuf::from("/System/Library/Fonts/Supplemental/Tahoma.ttf"), 0));
        bold.push((PathBuf::from("/System/Library/Fonts/Supplemental/Tahoma Bold.ttf"), 0));
    } else if cfg!(windows) {
        let fonts = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Windows")).join("Fonts");
        if let Some(l) = std::env::var_os("LOCALAPPDATA") {
            let user = PathBuf::from(l).join("Microsoft\\Windows\\Fonts");
            regular.push((user.join("Sarabun-Regular.ttf"), 0));
            bold.push((user.join("Sarabun-SemiBold.ttf"), 0));
        }
        regular.push((fonts.join("Sarabun-Regular.ttf"), 0));
        bold.push((fonts.join("Sarabun-SemiBold.ttf"), 0));
        // Leelawadee UI, the Windows interface face for Thai, then Tahoma.
        regular.push((fonts.join("LeelawUI.ttf"), 0));
        bold.push((fonts.join("LeelaUIb.ttf"), 0));
        regular.push((fonts.join("tahoma.ttf"), 0));
        bold.push((fonts.join("tahomabd.ttf"), 0));
    } else {
        for dir in ["/usr/share/fonts/truetype/noto", "/usr/share/fonts/noto", "/usr/share/fonts/google-noto"] {
            regular.push((Path::new(dir).join("NotoSansThai-Regular.ttf"), 0));
            bold.push((Path::new(dir).join("NotoSansThai-Bold.ttf"), 0));
        }
    }
    (regular, bold)
}

fn load(list: &[(PathBuf, u32)]) -> Option<FontData> {
    list.iter().find_map(|(path, index)| {
        let meta = std::fs::metadata(path).ok()?;
        if !meta.is_file() || meta.len() > MAX_FONT_BYTES {
            return None;
        }
        let bytes = std::fs::read(path).ok()?;
        let mut data = FontData::from_owned(bytes);
        data.index = *index;
        Some(data)
    })
}

/// The bold family name used by [`install`].
pub const BOLD: &str = "kruboffice-bold";

/// Put the Thai faces first in the proportional family, and a bold family for headings.
/// `resources` is the app's resources folder, where packaging may have placed Sarabun.
pub fn install(ctx: &egui::Context, resources: Option<&Path>) {
    let (regular, bold) = candidates(resources);
    let mut defs = FontDefinitions::default();
    let fallback: Vec<String> = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let regular = load(&regular);
    let bold = load(&bold);
    let mut proportional = Vec::new();
    if let Some(r) = regular {
        defs.font_data.insert("thai".into(), r.into());
        proportional.push("thai".to_string());
    }
    proportional.extend(fallback.iter().cloned());
    let mut heading = Vec::new();
    if let Some(b) = bold {
        defs.font_data.insert("thai-bold".into(), b.into());
        heading.push("thai-bold".to_string());
    }
    heading.extend(proportional.iter().cloned());
    defs.families.insert(FontFamily::Proportional, proportional);
    defs.families.insert(FontFamily::Name(BOLD.into()), heading);
    ctx.set_fonts(defs);
}
