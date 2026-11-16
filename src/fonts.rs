//! The Start Center's interface font: the operating system's own.
//!
//! Each platform's interface face is read from the system at runtime (nothing is bundled or
//! compiled in): on macOS the system font (SF) with Thonburi for Thai, on Windows Segoe UI with
//! Leelawadee UI for Thai, on Linux the desktop's interface font as fontconfig resolves it, with
//! the Thai face fontconfig picks for it. A face that has no Thai is always followed by one that
//! does; if the system has none, Sarabun from the app's `fonts` folder (put there by the
//! packaging) keeps Thai from showing as boxes. egui's built-in fonts come last, for the rest.

use std::path::{Path, PathBuf};

use eframe::egui::epaint::text::VariationCoords;
use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontTweak};

/// The largest font file loaded (a font collection can be big, a hostile file bigger).
const MAX_FONT_BYTES: u64 = 64 * 1024 * 1024;

/// Weights asked of variable faces (SF, many Linux faces); static faces ignore them.
const REGULAR: f32 = 400.0;
const SEMIBOLD: f32 = 600.0;

/// One font file to try: its path and the face index in a collection.
#[derive(Clone, Debug, PartialEq)]
struct Face {
    path: PathBuf,
    index: u32,
}

fn face(path: impl Into<PathBuf>, index: u32) -> Face {
    Face { path: path.into(), index }
}

/// Regular and bold candidates for one slot of the font stack, best first.
#[derive(Default)]
struct Slot {
    regular: Vec<Face>,
    bold: Vec<Face>,
}

/// The interface face (`ui`) and the Thai face behind it (`thai`), as this system names them.
fn system_slots() -> (Slot, Slot) {
    let mut ui = Slot::default();
    let mut thai = Slot::default();
    if cfg!(target_os = "macos") {
        // SF, the macOS system font: one variable file (weight set by `wght`). It has no Thai;
        // macOS shows Thai in its interface with Thonburi (regular is face 0, bold face 1).
        ui.regular.push(face("/System/Library/Fonts/SFNS.ttf", 0));
        ui.bold.push(face("/System/Library/Fonts/SFNS.ttf", 0));
        thai.regular.push(face("/System/Library/Fonts/ThonburiUI.ttc", 0));
        thai.bold.push(face("/System/Library/Fonts/ThonburiUI.ttc", 1));
        thai.regular.push(face("/System/Library/Fonts/Supplemental/Tahoma.ttf", 0));
        thai.bold.push(face("/System/Library/Fonts/Supplemental/Tahoma Bold.ttf", 0));
    } else if cfg!(windows) {
        // Segoe UI, the Windows interface font (read from the system, never shipped); Windows
        // shows Thai with Leelawadee UI, then Tahoma on older systems.
        let fonts = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Windows")).join("Fonts");
        ui.regular.push(face(fonts.join("segoeui.ttf"), 0));
        ui.bold.push(face(fonts.join("seguisb.ttf"), 0));
        ui.bold.push(face(fonts.join("segoeuib.ttf"), 0));
        thai.regular.push(face(fonts.join("LeelawUI.ttf"), 0));
        thai.bold.push(face(fonts.join("LeelaUIb.ttf"), 0));
        thai.regular.push(face(fonts.join("tahoma.ttf"), 0));
        thai.bold.push(face(fonts.join("tahomabd.ttf"), 0));
    } else {
        let family = linux::desktop_font_family();
        let base = family.as_deref().unwrap_or("sans-serif");
        ui.regular.extend(linux::fc_match(base, false, false));
        ui.bold.extend(linux::fc_match(base, true, false));
        thai.regular.extend(linux::fc_match(base, false, true));
        thai.bold.extend(linux::fc_match(base, true, true));
        if family.is_some() {
            thai.regular.extend(linux::fc_match("sans-serif", false, true));
            thai.bold.extend(linux::fc_match("sans-serif", true, true));
        }
        for dir in ["/usr/share/fonts/truetype/noto", "/usr/share/fonts/noto", "/usr/share/fonts/google-noto"] {
            thai.regular.push(face(Path::new(dir).join("NotoSansThai-Regular.ttf"), 0));
            thai.bold.push(face(Path::new(dir).join("NotoSansThai-Bold.ttf"), 0));
        }
        for dir in ["/usr/share/fonts/truetype/tlwg", "/usr/share/fonts/tlwg"] {
            thai.regular.push(face(Path::new(dir).join("Loma.ttf"), 0));
            thai.bold.push(face(Path::new(dir).join("Loma-Bold.ttf"), 0));
        }
    }
    (ui, thai)
}

/// Sarabun in the app's resources, used only when the system has no Thai face.
fn bundled_slot(resources: Option<&Path>) -> Slot {
    let mut s = Slot::default();
    if let Some(r) = resources {
        s.regular.push(face(r.join("fonts/Sarabun-Regular.ttf"), 0));
        s.bold.push(face(r.join("fonts/Sarabun-SemiBold.ttf"), 0));
    }
    s
}

#[cfg(not(any(target_os = "macos", windows)))]
mod linux {
    use super::Face;

    /// Run a desktop tool briefly and return its trimmed output, if it worked.
    fn output(program: &str, args: &[&str]) -> Option<String> {
        let out = std::process::Command::new(program).args(args).stderr(std::process::Stdio::null()).output().ok()?;
        if !out.status.success() {
            return None;
        }
        let s = String::from_utf8(out.stdout).ok()?;
        let s = s.trim();
        (!s.is_empty()).then(|| s.to_string())
    }

    /// The interface font family the desktop is set to (Cinnamon, GNOME, MATE), without its size.
    pub(super) fn desktop_font_family() -> Option<String> {
        let keys =
            [("org.cinnamon.desktop.interface", "font-name"), ("org.gnome.desktop.interface", "font-name"), ("org.mate.interface", "font-name")];
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();
        let mut order: Vec<&(&str, &str)> = keys.iter().collect();
        // The running desktop's own setting first; others may hold stale values.
        order.sort_by_key(|(schema, _)| {
            let wanted = if desktop.contains("cinnamon") || desktop.contains("x-cinnamon") {
                "cinnamon"
            } else if desktop.contains("mate") {
                "mate"
            } else {
                "gnome"
            };
            !schema.contains(wanted)
        });
        order.into_iter().find_map(|(schema, key)| output("gsettings", &["get", schema, key]).and_then(|v| family_of(&v)))
    }

    /// `'Ubuntu 11'` → `Ubuntu`; `'Noto Sans Bold 10'` keeps the style words for fontconfig to ignore.
    pub(super) fn family_of(setting: &str) -> Option<String> {
        let s = setting.trim().trim_matches('\'').trim_matches('"').trim();
        let s = match s.rsplit_once(' ') {
            Some((name, size)) if size.parse::<f32>().is_ok() => name.trim(),
            _ => s,
        };
        // fontconfig pattern syntax: these characters would start options.
        let s: String = s.chars().filter(|c| !matches!(c, ':' | ',' | '-' | '\\' | '=')).collect();
        (!s.trim().is_empty()).then(|| s.trim().to_string())
    }

    /// The file fontconfig picks for `family` (semibold if `bold`), only if it covers Thai when `thai`.
    pub(super) fn fc_match(family: &str, bold: bool, thai: bool) -> Option<Face> {
        let mut pattern = family.to_string();
        if bold {
            pattern.push_str(":weight=demibold");
        }
        if thai {
            pattern.push_str(":lang=th");
        }
        let text = output("fc-match", &["--format=%{file}\n%{index}\n%{lang}", &pattern])?;
        parse_match(&text, thai)
    }

    pub(super) fn parse_match(text: &str, thai: bool) -> Option<Face> {
        let mut lines = text.lines();
        let path = lines.next()?.trim();
        // fontconfig puts a variable font's named instance in the high 16 bits.
        let index = lines.next()?.trim().parse::<u32>().ok()? & 0xFFFF;
        let langs = lines.next().unwrap_or("");
        if path.is_empty() || (thai && !langs.split('|').any(|l| l == "th")) {
            return None;
        }
        Some(super::face(path, index))
    }
}

#[cfg(any(target_os = "macos", windows))]
mod linux {
    pub(super) fn desktop_font_family() -> Option<String> {
        None
    }
    pub(super) fn fc_match(_: &str, _: bool, _: bool) -> Option<super::Face> {
        None
    }
}

/// The first candidate that loads, at `weight` if it is a variable face.
fn load(list: &[Face], weight: f32) -> Option<FontData> {
    list.iter().find_map(|f| {
        let meta = std::fs::metadata(&f.path).ok()?;
        if !meta.is_file() || meta.len() > MAX_FONT_BYTES {
            return None;
        }
        let bytes = std::fs::read(&f.path).ok()?;
        let mut data = FontData::from_owned(bytes);
        data.index = f.index;
        data.tweak = FontTweak { coords: VariationCoords::new([(b"wght", weight)]), ..FontTweak::default() };
        Some(data)
    })
}

/// The bold family name used by [`install`].
pub const BOLD: &str = "kruboffice-bold";

/// Put the system's interface faces first in the proportional family, and a bold family for
/// headings. `resources` is the app's resources folder, where packaging may have placed Sarabun.
pub fn install(ctx: &egui::Context, resources: Option<&Path>) {
    let (ui, thai) = system_slots();
    let mut defs = FontDefinitions::default();
    let fallback: Vec<String> = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let mut proportional = Vec::new();
    let mut heading = Vec::new();
    let mut add = |name: &str, regular: Option<FontData>, bold: Option<FontData>| {
        let has_regular = regular.is_some();
        if let Some(r) = regular {
            defs.font_data.insert(name.into(), r.into());
            proportional.push(name.to_string());
        }
        let bold_name = format!("{name}-bold");
        if let Some(b) = bold {
            defs.font_data.insert(bold_name.clone(), b.into());
            heading.push(bold_name);
        } else if has_regular {
            heading.push(name.to_string());
        }
        has_regular
    };
    add("system-ui", load(&ui.regular, REGULAR), load(&ui.bold, SEMIBOLD));
    let thai_found = add("system-thai", load(&thai.regular, REGULAR), load(&thai.bold, SEMIBOLD));
    if !thai_found {
        let bundled = bundled_slot(resources);
        if !add("sarabun", load(&bundled.regular, REGULAR), load(&bundled.bold, SEMIBOLD)) {
            log::warn!("no Thai font found on this system; Thai text may not show");
        }
    }
    proportional.extend(fallback.iter().cloned());
    heading.extend(proportional.iter().cloned());
    defs.families.insert(FontFamily::Proportional, proportional);
    defs.families.insert(FontFamily::Name(BOLD.into()), heading);
    ctx.set_fonts(defs);
}

#[cfg(all(test, not(any(target_os = "macos", windows))))]
mod tests {
    use super::linux::{family_of, parse_match};

    #[test]
    fn desktop_font_setting() {
        assert_eq!(family_of("'Ubuntu 11'").as_deref(), Some("Ubuntu"));
        assert_eq!(family_of("'Noto Sans 10.5'").as_deref(), Some("Noto Sans"));
        assert_eq!(family_of("'Cantarell'").as_deref(), Some("Cantarell"));
        assert_eq!(family_of("''"), None);
    }

    #[test]
    fn fc_match_output() {
        let f = parse_match("/usr/share/fonts/NotoSansThai.ttf\n196608\naa|th|en", true);
        assert_eq!(f.map(|f| f.index), Some(0));
        assert!(parse_match("/usr/share/fonts/Ubuntu.ttf\n0\naa|en", true).is_none());
        assert!(parse_match("/usr/share/fonts/Ubuntu.ttf\n0\naa|en", false).is_some());
    }
}
