#[cfg(not(target_arch = "wasm32"))]
use super::Blob;
use super::font_fallback::load_fallback;
use super::{Arc, Color32, FontData, FontDefinitions, FontFamily, FontId};

/// A font from the optional craft-fonts build input (`CRAFT_FONTS_DIR` at build time; see
/// <https://github.com/storytold/craft-fonts> and its `docs/integration.md`). The slice is empty
/// when the app is built without the checkout; nothing is downloaded and no font is in this repo.
#[derive(Debug)]
pub struct CraftFont {
    pub family: &'static str,
    pub style: &'static str,
    /// ISO 15924 scripts the family is intended for, from the craft-fonts manifest.
    pub scripts: &'static [&'static str],
    pub bytes: &'static [u8],
}

mod craft_fonts {
    use super::CraftFont;
    include!(concat!(env!("OUT_DIR"), "/craft_fonts.rs"));
}

/// The light or dark value of a colour, by mode.
pub(super) fn pick<T>(dark: bool, light: T, dark_value: T) -> T {
    if dark { dark_value } else { light }
}

pub const UI: &str = "ui";

pub const UI_BOLD: &str = "ui-bold";

pub const CELL: &str = "cell";

pub const CELL_BOLD: &str = "cell-bold";

pub const CELL_ITALIC: &str = "cell-italic";

pub const CELL_BOLD_ITALIC: &str = "cell-bold-italic";

pub const SERIF: &str = "serif";

pub const MONO: &str = "mono";

/// Equation faces: a maths font (upright letters, operators, Greek, double-struck and script
/// letters), its italic for variables, and a bold. System fonts only; nothing is bundled.
pub const MATH: &str = "math";

pub const MATH_ITALIC: &str = "math-italic";

pub const MATH_BOLD: &str = "math-bold";

pub fn ui_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(UI.into()))
}

pub fn ui_bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(UI_BOLD.into()))
}

/// The font family to use for a cell font name and weight.
///
/// When the name is an installed font whose faces were bound by [`font_definitions`], that family
/// is used; otherwise the name is bucketed to one of the built-in roles (mono/serif/cell). egui
/// panics on a family that was never bound, so the installed branch is gated on
/// [`crate::fonts::is_bound`] and never names a family egui hasn't been given.
pub fn cell_family(name: &str, bold: bool, italic: bool) -> FontFamily {
    let role = crate::fonts::role_key(name, bold, italic);
    if crate::fonts::is_bound(&role) {
        return FontFamily::Name(role.into());
    }
    let lower = name.to_ascii_lowercase();
    if lower.contains("courier") || lower.contains("mono") || lower.contains("consolas") {
        return FontFamily::Name(MONO.into());
    }
    if lower.contains("times")
        || lower.contains("georgia")
        || lower.contains("cambria")
        || lower.contains("serif") && !lower.contains("sans")
        || lower.contains("garamond")
    {
        return FontFamily::Name(SERIF.into());
    }
    FontFamily::Name(
        match (bold, italic) {
            (true, true) => CELL_BOLD_ITALIC,
            (true, false) => CELL_BOLD,
            (false, true) => CELL_ITALIC,
            (false, false) => CELL,
        }
        .into(),
    )
}

/// Width in points, at 100% zoom, of one line of cell text drawn as the grid draws it: in the
/// face [`cell_family`] picks, which on a machine without the named font is a wider or narrower
/// substitute (#1316). A family the active font set doesn't have yet (bound fonts apply from the
/// next frame) falls back to the engine's estimate rather than letting egui panic on it.
///
/// `exact` lays the text out (a galley kept until the end of the frame); otherwise the glyph
/// advances are summed without kerning, which is cheap and never narrower. This takes the
/// Context lock (`fonts_mut`): never call it, or run a command, while an egui lock is held.
pub fn cell_text_width(ctx: &egui::Context, text: &str, font: &gridcraft_engine::model::Font, exact: bool) -> f32 {
    let family = cell_family(&font.name, font.bold, font.italic);
    let id = FontId::new(font.size * 96.0 / 72.0, family);
    ctx.fonts_mut(|f| {
        f.families().contains(&id.family).then(|| {
            if exact {
                f.layout_no_wrap(text.to_owned(), id, Color32::PLACEHOLDER).size().x
            } else {
                // Half a point covers the galley's rounding of its width up to the pixel grid.
                text.chars().map(|c| f.glyph_width(&id, c)).sum::<f32>() + 0.5
            }
        })
    })
    .unwrap_or_else(|| gridcraft_engine::cmd::format::approx_text_width(text, font.size, font.bold))
}

/// Whether epaint can open face `index` of this font file and it maps characters to glyphs.
/// epaint panics (in `Context::set_fonts`, i.e. on startup) on a face it can't open, so every font
/// file we did not ship is checked first with the same parser and the same call epaint makes
/// (`skrifa::FontRef::from_index`): a corrupt file, something that is not an sfnt at all, or a
/// collection index past the end is rejected instead of crashing the app (#226). A face whose
/// character map is unreadable (a truncated file) would open but draw nothing, so it is refused
/// too and the next candidate gets its turn.
pub(crate) fn font_parses(bytes: &[u8], index: u32) -> bool {
    use skrifa::MetadataProvider;
    skrifa::FontRef::from_index(bytes, index).is_ok_and(|font| font.charmap().has_map())
}

/// Loads the first candidate that exists and parses. A file that exists but can't be parsed is
/// skipped with a warning and the next candidate is tried.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn try_load(paths: &[(&str, u32)]) -> Option<FontData> {
    for (p, index) in paths {
        if let Some(blob) = font_file(p, *index) {
            return Some(FontData::from_blob(blob, *index));
        }
    }
    None
}

/// The interface and script fallback font files, each read once per run and kept as a shared
/// `Blob` for the rest of it. egui shares the blob instead of copying the file into its text
/// engine, because every one of these files is loaded by every font rebuild for as long as the
/// app runs. Only a file that parses is kept: a missing or broken one is looked for again next
/// time.
#[cfg(not(target_arch = "wasm32"))]
fn font_file(path: &str, index: u32) -> Option<Blob> {
    static FILES: std::sync::Mutex<std::collections::BTreeMap<String, Blob>> = std::sync::Mutex::new(std::collections::BTreeMap::new());
    let mut files = FILES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(blob) = files.get(path) {
        let bytes: &[u8] = (**blob).as_ref();
        return font_parses(bytes, index).then(|| std::sync::Arc::clone(blob));
    }
    let bytes = std::fs::read(path).ok()?;
    if !font_parses(&bytes, index) {
        log::warn!("skipping font {path} (face {index}): not a font file egui can read");
        return None;
    }
    let blob: Blob = std::sync::Arc::new(bytes);
    files.insert(path.to_string(), blob.clone());
    Some(blob)
}

/// Last line of defence before [`FontDefinitions`] reach egui: drops every face epaint could not
/// parse (and its key from every family), so a bad font costs a fallback, never a crash.
pub(crate) fn drop_unparseable_fonts(fonts: &mut FontDefinitions) {
    let bad: Vec<String> = fonts.font_data.iter().filter(|(_, fd)| !font_parses(fd.bytes(), fd.index)).map(|(k, _)| k.clone()).collect();
    for key in &bad {
        log::warn!("dropping font {key:?}: egui can't parse it");
        fonts.font_data.remove(key);
    }
    if !bad.is_empty() {
        for chain in fonts.families.values_mut() {
            chain.retain(|k| !bad.contains(k));
        }
    }
}

/// egui's bundled monospace face, used as the last-resort symbol fallback (see `Hack-Regular.ttf`
/// in `epaint_default_fonts`). It is present whenever egui is built with `default_fonts`.
const HACK: &str = "Hack";

/// The permitted craft-fonts faces with their `FontDefinitions` keys, in manifest order. Empty
/// without a `CRAFT_FONTS_DIR` checkout at build time. The filter is repeated here so a stale
/// generated table can never smuggle in a barred face or a web-build face outside the allowlist.
pub(super) fn craft_font_faces() -> Vec<(String, &'static CraftFont)> {
    let wasm = cfg!(target_arch = "wasm32");
    craft_fonts::CRAFT_FONTS
        .iter()
        .enumerate()
        .filter(|(_, font)| !crate::craft_fonts_policy::barred(font.family))
        .filter(|(_, font)| !wasm || crate::craft_fonts_policy::web_allowed(font.family, font.style))
        .map(|(i, font)| (format!("craft-{i}-{}-{}", font.family, font.style), font))
        .collect()
}

/// Adds the optional craft-fonts faces to `fonts` and returns their keys.
fn add_craft_fonts(fonts: &mut FontDefinitions) -> Vec<String> {
    let mut keys = Vec::new();
    for (key, font) in craft_font_faces() {
        fonts.font_data.insert(key.clone(), Arc::new(FontData::from_static(font.bytes)));
        keys.push(key);
    }
    keys
}

/// Whether the optional craft-fonts input was embedded in this build.
pub fn craft_fonts_loaded() -> bool {
    !craft_fonts::CRAFT_FONTS.is_empty()
}

/// Whether the native-only Simplified Chinese craft face (LXGW WenKai Lite) was embedded. A web
/// build or an older craft-fonts checkout without `fonts/simplified-chinese-manifest.txt` has none.
pub fn chinese_font_loaded() -> bool {
    craft_font_faces().iter().any(|(_, f)| f.family == "LXGW WenKai Lite" && f.style == "Regular")
}

/// Whether this build was made with `CRAFT_FONTS_REQUIRED=1`. Acceptance tests must fail rather
/// than skip when this is true and the faces are absent.
pub fn craft_fonts_required() -> bool {
    craft_fonts::CRAFT_FONTS_REQUIRED
}

/// A bundled-face character with the same meaning as `c`, for interface chrome whose exact symbol
/// no bundled face has (`✕`, `✓`, `⌃`). Document text still needs a real glyph; this is only a
/// display fallback and never changes the characters a command inserts or stores.
pub fn symbol_fallback(c: char) -> Option<char> {
    match c {
        '✕' => Some('×'),
        '✓' => Some('✔'),
        '⌃' => Some('^'),
        _ => None,
    }
}

/// `text` with every character the current font set cannot draw replaced by its semantic fallback
/// (see [`symbol_fallback`]). Used by chrome tooltips so a host without symbol faces never paints
/// a tofu box; it is not applied to workbook text.
pub fn available_text(ui: &egui::Ui, text: &str) -> String {
    let font = ui_font(13.0);
    text.chars().map(|c| if ui.ctx().fonts_mut(|f| f.has_glyph(&font, c)) { c } else { symbol_fallback(c).unwrap_or(c) }).collect()
}

/// The family, style and scripts of every embedded craft-fonts face (empty without the input).
pub fn craft_font_scripts() -> impl Iterator<Item = (&'static str, &'static str, &'static [&'static str])> {
    craft_font_faces().into_iter().map(|(_, f)| (f.family, f.style, f.scripts))
}

/// Whether one embedded craft-fonts face maps every non-whitespace character in `text`. Tests use
/// this to tell "the pinned fixture covers this script" from "this host has no face for it",
/// without trusting a manifest script tag alone.
pub fn craft_fonts_cover(text: &str) -> bool {
    craft_font_faces().iter().any(|(_, f)| {
        use skrifa::MetadataProvider;
        let Ok(font) = skrifa::FontRef::from_index(f.bytes, 0) else {
            return false;
        };
        let map = font.charmap();
        text.chars().filter(|c| !c.is_whitespace()).all(|c| map.map(c).is_some())
    })
}

#[cfg(target_arch = "wasm32")]
pub(super) fn try_load(_paths: &[(&str, u32)]) -> Option<FontData> {
    None
}

fn home_fonts(file: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    format!("{home}/Library/Fonts/{file}")
}

/// Every file in `files` under every directory in `dirs`, directory first, in the same order a
/// nested loop over the directories would add them.
pub(super) fn in_dirs(dirs: &[&str], files: &[&str]) -> Vec<String> {
    dirs.iter().flat_map(|d| files.iter().map(move |f| format!("{d}/{f}"))).collect()
}

/// Candidate font files per role, in preference order.
pub(super) fn candidates(role: &str) -> Vec<(String, u32)> {
    let mac_sys = "/System/Library/Fonts";
    let mac_sup = "/System/Library/Fonts/Supplemental";
    let win = std::env::var("WINDIR").map(|w| format!("{w}\\Fonts")).unwrap_or_else(|_| "C:\\Windows\\Fonts".into());
    // ChromeOS's Linux container (Crostini) mounts the host's fonts under `chromeos/` (#1762).
    let lin = ["/usr/share/fonts/truetype", "/usr/share/fonts", "/usr/local/share/fonts", "/usr/share/fonts/TTF", "/usr/share/fonts/chromeos"];
    let mut v: Vec<(String, u32)> = Vec::new();
    let mut add = |s: String, i: u32| v.push((s, i));
    match role {
        "ui" => {
            add(format!("{mac_sys}/SFNS.ttf"), 0);
            add(format!("{mac_sys}/HelveticaNeue.ttc"), 0);
            add(format!("{win}\\segoeui.ttf"), 0);
            for p in in_dirs(
                &lin,
                &["cantarell/Cantarell-Regular.otf", "noto/NotoSans-Regular.ttf", "dejavu/DejaVuSans.ttf", "liberation/LiberationSans-Regular.ttf"],
            ) {
                add(p, 0);
            }
            // Fedora keeps each family in its own package folder (#54).
            add("/usr/share/fonts/abattis-cantarell-fonts/Cantarell-Regular.otf".into(), 0);
            add("/usr/share/fonts/google-noto/NotoSans-Regular.ttf".into(), 0);
            add("/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf".into(), 0);
            add("/usr/share/fonts/liberation-sans-fonts/LiberationSans-Regular.ttf".into(), 0);
        }
        "ui-bold" => {
            add(format!("{mac_sys}/HelveticaNeue.ttc"), 1);
            add(format!("{win}\\segoeuib.ttf"), 0);
            for p in in_dirs(&lin, &["noto/NotoSans-Bold.ttf", "dejavu/DejaVuSans-Bold.ttf", "liberation/LiberationSans-Bold.ttf"]) {
                add(p, 0);
            }
            add("/usr/share/fonts/google-noto/NotoSans-Bold.ttf".into(), 0);
            add("/usr/share/fonts/dejavu-sans-fonts/DejaVuSans-Bold.ttf".into(), 0);
            add("/usr/share/fonts/liberation-sans-fonts/LiberationSans-Bold.ttf".into(), 0);
        }
        "cell" | "cell-bold" | "cell-italic" | "cell-bold-italic" => {
            let (carlito, calibri_win, calibri_mac, arial_mac, arial_win, lib, dejavu) = match role {
                "cell" => {
                    ("Carlito-Regular.ttf", "calibri.ttf", "Calibri.ttf", "Arial.ttf", "arial.ttf", "LiberationSans-Regular.ttf", "DejaVuSans.ttf")
                }
                "cell-bold" => (
                    "Carlito-Bold.ttf",
                    "calibrib.ttf",
                    "Calibri Bold.ttf",
                    "Arial Bold.ttf",
                    "arialbd.ttf",
                    "LiberationSans-Bold.ttf",
                    "DejaVuSans-Bold.ttf",
                ),
                "cell-italic" => (
                    "Carlito-Italic.ttf",
                    "calibrii.ttf",
                    "Calibri Italic.ttf",
                    "Arial Italic.ttf",
                    "ariali.ttf",
                    "LiberationSans-Italic.ttf",
                    "DejaVuSans-Oblique.ttf",
                ),
                _ => (
                    "Carlito-BoldItalic.ttf",
                    "calibriz.ttf",
                    "Calibri Bold Italic.ttf",
                    "Arial Bold Italic.ttf",
                    "arialbi.ttf",
                    "LiberationSans-BoldItalic.ttf",
                    "DejaVuSans-BoldOblique.ttf",
                ),
            };
            // Calibri if the user has it installed system-wide (never from an Office bundle).
            add(format!("/Library/Fonts/{calibri_mac}"), 0);
            add(home_fonts(calibri_mac), 0);
            add(format!("{win}\\{calibri_win}"), 0);
            for p in in_dirs(&lin, &[&format!("crosextra/{carlito}"), &format!("carlito/{carlito}")]) {
                add(p, 0);
            }
            // Fedora keeps each family in its own package folder, as for the UI fonts (#54).
            add(format!("/usr/share/fonts/google-carlito-fonts/{carlito}"), 0);
            add(home_fonts(carlito), 0);
            add(format!("{mac_sup}/{arial_mac}"), 0);
            add(format!("{win}\\{arial_win}"), 0);
            for p in in_dirs(&lin, &[&format!("liberation/{lib}"), &format!("dejavu/{dejavu}")]) {
                add(p, 0);
            }
            add(format!("/usr/share/fonts/liberation-sans-fonts/{lib}"), 0);
            add(format!("/usr/share/fonts/dejavu-sans-fonts/{dejavu}"), 0);
        }
        "serif" => {
            add(format!("{mac_sup}/Times New Roman.ttf"), 0);
            add(format!("{win}\\times.ttf"), 0);
            for p in in_dirs(&lin, &["liberation/LiberationSerif-Regular.ttf", "dejavu/DejaVuSerif.ttf"]) {
                add(p, 0);
            }
            add("/usr/share/fonts/liberation-serif-fonts/LiberationSerif-Regular.ttf".into(), 0);
            add("/usr/share/fonts/dejavu-serif-fonts/DejaVuSerif.ttf".into(), 0);
        }
        "math" => {
            add(format!("{mac_sup}/STIXTwoMath.otf"), 0);
            add(format!("{win}\\cambria.ttc"), 1);
            for p in in_dirs(
                &lin,
                &[
                    "opentype/stix/STIXTwoMath-Regular.otf",
                    "stix/STIXTwoMath-Regular.otf",
                    "opentype/lmodern/latinmodern-math.otf",
                    "dejavu/DejaVuSerif.ttf",
                    "liberation/LiberationSerif-Regular.ttf",
                ],
            ) {
                add(p, 0);
            }
            add(format!("{mac_sup}/Times New Roman.ttf"), 0);
            add(format!("{win}\\times.ttf"), 0);
        }
        "math-italic" => {
            add(format!("{mac_sup}/STIXTwoText-Italic.ttf"), 0);
            add(format!("{mac_sup}/Times New Roman Italic.ttf"), 0);
            add(format!("{win}\\cambriai.ttf"), 0);
            add(format!("{win}\\timesi.ttf"), 0);
            for p in
                in_dirs(&lin, &["opentype/stix/STIXTwoText-Italic.otf", "liberation/LiberationSerif-Italic.ttf", "dejavu/DejaVuSerif-Italic.ttf"])
            {
                add(p, 0);
            }
        }
        "math-bold" => {
            add(format!("{mac_sup}/Times New Roman Bold.ttf"), 0);
            add(format!("{win}\\cambriab.ttf"), 0);
            add(format!("{win}\\timesbd.ttf"), 0);
            for p in in_dirs(&lin, &["opentype/stix/STIXTwoText-Bold.otf", "liberation/LiberationSerif-Bold.ttf", "dejavu/DejaVuSerif-Bold.ttf"]) {
                add(p, 0);
            }
        }
        "mono" => {
            add(format!("{mac_sys}/SFNSMono.ttf"), 0);
            add(format!("{mac_sys}/Menlo.ttc"), 0);
            add(format!("{win}\\consola.ttf"), 0);
            for p in in_dirs(&lin, &["dejavu/DejaVuSansMono.ttf"]) {
                add(p, 0);
            }
            add("/usr/share/fonts/dejavu-sans-mono-fonts/DejaVuSansMono.ttf".into(), 0);
            // Fedora installs DejaVu Sans Mono only on request. Liberation Mono comes with LibreOffice
            // (in Workstation and KDE by default), Noto Sans Mono (variable, Regular by default) always.
            for p in in_dirs(&lin, &["liberation/LiberationMono-Regular.ttf"]) {
                add(p, 0);
            }
            add("/usr/share/fonts/liberation-mono-fonts/LiberationMono-Regular.ttf".into(), 0);
            add("/usr/share/fonts/google-noto-vf/NotoSansMono[wght].ttf".into(), 0);
        }
        _ => {}
    }
    v
}

/// Builds the font set from system fonts (with egui's defaults as fallback), with the default
/// (non-Japanese) Han order. Never reads the host locale, so tests and snapshots are stable.
pub fn font_definitions() -> FontDefinitions {
    font_definitions_for_language(crate::i18n::Language::En)
}

/// The faces a host supplies at startup. A browser has no system fonts to scan, so the web shell
/// fetches its Thai faces and hands them over with [`set_host_faces`] before the first frame; every
/// font rebuild after that includes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostFace {
    /// Sarabun Regular: the interface font in Thai.
    Sarabun,
    /// Sarabun SemiBold: the bold interface font in Thai.
    SarabunBold,
    /// TH Sarabun New in its four styles: what draws Thai letters in cells.
    ThSarabunNew,
    ThSarabunNewBold,
    ThSarabunNewItalic,
    ThSarabunNewBoldItalic,
    /// The system's interface font for Latin text and its bold, read from the computer with the
    /// browser's Local Font Access (see [`WEB_SYSTEM_UI_LATIN`]): they lead the interface.
    SystemUi,
    SystemUiBold,
    /// The same for Thai letters ([`WEB_SYSTEM_UI_THAI`]).
    SystemUiThai,
    SystemUiThaiBold,
}

/// Platform interface families the web asks the computer for, best first.
pub const WEB_SYSTEM_UI_LATIN: &[&str] = &[
    "SF Pro Text",
    "SF Pro",
    ".AppleSystemUIFont",
    ".SF NS",
    ".SFNS",
    "System Font",
    "Segoe UI Variable Text",
    "Segoe UI Variable",
    "Segoe UI",
    "Helvetica Neue",
    "Cantarell",
    "Noto Sans",
];
/// The Thai interface families to ask for, best first.
pub const WEB_SYSTEM_UI_THAI: &[&str] = &["Thonburi UI", "Thonburi", "Leelawadee UI", "Leelawadee", "Noto Sans Thai UI", "Noto Sans Thai"];

impl HostFace {
    /// The faces read from the computer, not shipped with the page.
    fn is_system(self) -> bool {
        matches!(self, Self::SystemUi | Self::SystemUiBold | Self::SystemUiThai | Self::SystemUiThaiBold)
    }

    fn key(self) -> &'static str {
        match self {
            Self::Sarabun => "host-sarabun",
            Self::SarabunBold => "host-sarabun-bold",
            Self::ThSarabunNew => "host-th-sarabun-new",
            Self::ThSarabunNewBold => "host-th-sarabun-new-bold",
            Self::ThSarabunNewItalic => "host-th-sarabun-new-italic",
            Self::ThSarabunNewBoldItalic => "host-th-sarabun-new-bold-italic",
            Self::SystemUi => "host-system-ui",
            Self::SystemUiBold => "host-system-ui-bold",
            Self::SystemUiThai => "host-system-ui-thai",
            Self::SystemUiThaiBold => "host-system-ui-thai-bold",
        }
    }
}

/// How much larger the Thai fallback faces draw than their nominal size (see [`apply_host_faces`]).
/// Measured in the font files: TH Sarabun New's Thai consonants are 0.40 em tall, the x-height of the
/// Latin stand-in (Ubuntu) is 0.52 em, and Thai consonants sit at about the x-height of the Latin
/// beside them, so 0.52 / 0.40 = 1.3.
const THAI_FALLBACK_SCALE: f32 = 1.3;

/// The font-data key of the enlarged copy of a host face used as a fallback.
fn fallback_key(face: HostFace) -> String {
    format!("{}-fallback", face.key())
}

static HOST_FACES: std::sync::Mutex<Vec<(HostFace, Arc<FontData>)>> = std::sync::Mutex::new(Vec::new());
/// Bumped whenever the registered faces change, so a font set built earlier is rebuilt ([`FontKey`]).
static HOST_REVISION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The faces whose bytes egui can read; the rest are skipped with a warning, so a corrupt download
/// costs a fallback, never a crash.
fn readable(faces: Vec<(HostFace, Vec<u8>)>) -> Vec<(HostFace, Arc<FontData>)> {
    faces
        .into_iter()
        .filter_map(|(face, bytes)| {
            if font_parses(&bytes, 0) {
                Some((face, Arc::new(FontData::from_owned(bytes))))
            } else {
                log::warn!("skipping host font {}: not a font file egui can read", face.key());
                None
            }
        })
        .collect()
}

/// Registers the faces the host loaded. Replaces any earlier registration.
pub fn set_host_faces(faces: Vec<(HostFace, Vec<u8>)>) {
    *HOST_FACES.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = readable(faces);
    HOST_REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// Adds faces to the registered ones (a face already there is replaced). The interface fonts are
/// rebuilt with them on the next frame.
pub fn add_host_faces(faces: Vec<(HostFace, Vec<u8>)>) {
    let added = readable(faces);
    if added.is_empty() {
        return;
    }
    let mut all = HOST_FACES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    all.retain(|(face, _)| !added.iter().any(|(a, _)| a == face));
    all.extend(added);
    HOST_REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// Whether the computer's interface font is in use (a system face is registered).
pub fn system_ui_fonts_loaded() -> bool {
    host_faces().iter().any(|(face, _)| face.is_system())
}

fn host_faces() -> Vec<(HostFace, Arc<FontData>)> {
    HOST_FACES.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
}

/// Adds the host faces to `fonts`. In Thai, Sarabun leads the interface families. Everywhere else the
/// faces are fallbacks at the end of each chain, so Thai letters (the app's name, Thai text in a cell)
/// draw without changing how Latin text looks: **the cell families never lead with them**, so cells
/// keep the workbook's own font (and, on the web, the same stand-in as in every other language).
/// Each family tries the face that matches its style first (TH Sarabun New Bold in bold cells).
///
/// Returns the role families it bound for the font names `Sarabun` and `TH Sarabun New` (the
/// workbook can ask for them by name, and the Font list offers them), as
/// [`crate::fonts::role_key`] builds them, so [`cell_family`] draws a cell set in one of them in it.
fn apply_host_faces(fonts: &mut FontDefinitions, language: crate::i18n::Language, host: &[(HostFace, Arc<FontData>)]) -> Vec<String> {
    for (face, data) in host {
        fonts.font_data.insert(face.key().to_string(), Arc::clone(data));
        if face.is_system() {
            continue; // leads the interface below; never a fallback in cells
        }
        // The copy that catches Thai letters in a Latin cell or interface font, drawn larger: TH
        // Sarabun New and Sarabun are small-looking faces, so beside Latin letters at the same point
        // size Thai looks about a fifth smaller ("ไทย English"). The face that leads a family is
        // not scaled: a cell set in TH Sarabun New keeps its own size.
        let mut scaled = (**data).clone();
        scaled.tweak.scale = THAI_FALLBACK_SCALE;
        fonts.font_data.insert(fallback_key(*face), Arc::new(scaled));
    }
    let has = |face: HostFace| host.iter().any(|(h, _)| *h == face);
    if language == crate::i18n::Language::Th {
        for (role, face) in [(UI, HostFace::Sarabun), (UI_BOLD, HostFace::SarabunBold)] {
            if has(face)
                && let Some(chain) = fonts.families.get_mut(&FontFamily::Name(role.into()))
            {
                chain.retain(|k| k != face.key());
                chain.insert(0, face.key().to_string());
            }
        }
    }
    // The computer's own interface font leads the menus and ribbon: its Latin face, then its Thai
    // one, then whatever the chain had (Sarabun in Thai).
    for (role, faces) in [(UI, [HostFace::SystemUi, HostFace::SystemUiThai]), (UI_BOLD, [HostFace::SystemUiBold, HostFace::SystemUiThaiBold])] {
        let Some(chain) = fonts.families.get_mut(&FontFamily::Name(role.into())) else { continue };
        // A bold face the computer did not give falls back to its regular one.
        let regular = if role == UI { faces } else { [HostFace::SystemUi, HostFace::SystemUiThai] };
        for (index, face) in faces.into_iter().enumerate().rev() {
            let use_face = if has(face) { Some(face) } else { Some(regular[index]).filter(|f| has(*f)) };
            if let Some(face) = use_face {
                chain.retain(|k| k != face.key());
                chain.insert(0, face.key().to_string());
            }
        }
    }
    // Cells are drawn in Sarabun (Latin, digits and Thai from one family), so a mixed line such as
    // "ไทย 500 English" shares one baseline and one letter size; the stand-in (Ubuntu) stays behind it
    // for symbols and other scripts. Serif and monospace cells keep their own faces.
    for (role, face) in
        [(CELL, HostFace::Sarabun), (CELL_ITALIC, HostFace::Sarabun), (CELL_BOLD, HostFace::SarabunBold), (CELL_BOLD_ITALIC, HostFace::SarabunBold)]
    {
        if has(face)
            && let Some(chain) = fonts.families.get_mut(&FontFamily::Name(role.into()))
        {
            chain.retain(|k| k != face.key());
            chain.insert(0, face.key().to_string());
        }
    }
    // The face that matches a role's style, tried before the others when a glyph is missing.
    let preferred = |role: &str| match role {
        UI => Some(HostFace::Sarabun),
        UI_BOLD => Some(HostFace::SarabunBold),
        CELL | SERIF | MONO => Some(HostFace::ThSarabunNew),
        CELL_BOLD => Some(HostFace::ThSarabunNewBold),
        CELL_ITALIC => Some(HostFace::ThSarabunNewItalic),
        CELL_BOLD_ITALIC => Some(HostFace::ThSarabunNewBoldItalic),
        _ => None,
    };
    for (family, chain) in &mut fonts.families {
        let first = match family {
            FontFamily::Name(name) => preferred(name),
            _ => None,
        };
        let order = first.filter(|f| has(*f)).into_iter().chain(host.iter().map(|(face, _)| *face).filter(|f| Some(*f) != first && !f.is_system()));
        for face in order {
            if !chain.iter().any(|k| k == face.key()) {
                chain.push(fallback_key(face));
            }
        }
    }
    bind_host_font_names(fonts, host)
}

/// Binds `Sarabun` and `TH Sarabun New` as font names a cell can ask for: one role family per
/// weight and slant, led by the matching face (the regular one where a style was not supplied) and
/// followed by the cell family's own chain for symbols and other scripts.
fn bind_host_font_names(fonts: &mut FontDefinitions, host: &[(HostFace, Arc<FontData>)]) -> Vec<String> {
    use HostFace::{Sarabun, SarabunBold, ThSarabunNew, ThSarabunNewBold, ThSarabunNewBoldItalic, ThSarabunNewItalic};
    let has = |face: HostFace| host.iter().any(|(h, _)| *h == face);
    let tail = fonts.families.get(&FontFamily::Name(CELL.into())).cloned().unwrap_or_default();
    let mut bound = Vec::new();
    /// The faces to try, best first, for a style of `TH Sarabun New`.
    fn th_sarabun_new(bold: bool, italic: bool) -> [HostFace; 4] {
        match (bold, italic) {
            (true, true) => [ThSarabunNewBoldItalic, ThSarabunNewBold, ThSarabunNewItalic, ThSarabunNew],
            (true, false) => [ThSarabunNewBold, ThSarabunNew, ThSarabunNew, ThSarabunNew],
            (false, true) => [ThSarabunNewItalic, ThSarabunNew, ThSarabunNew, ThSarabunNew],
            (false, false) => [ThSarabunNew; 4],
        }
    }
    /// The same for `Sarabun` (regular and semibold, no italic).
    fn sarabun(bold: bool, _italic: bool) -> [HostFace; 4] {
        if bold { [SarabunBold, Sarabun, Sarabun, Sarabun] } else { [Sarabun; 4] }
    }
    type Faces = fn(bool, bool) -> [HostFace; 4];
    let names: [(&str, Faces); 2] = [("TH Sarabun New", th_sarabun_new), ("Sarabun", sarabun)];
    for (name, wanted) in names {
        for bold in [false, true] {
            for italic in [false, true] {
                let Some(face) = wanted(bold, italic).into_iter().find(|f| has(*f)) else { continue };
                let role = crate::fonts::role_key(name, bold, italic);
                let mut chain = vec![face.key().to_string()];
                chain.extend(tail.iter().filter(|k| k.as_str() != face.key()).cloned());
                fonts.families.insert(FontFamily::Name(role.clone().into()), chain);
                bound.push(role);
            }
        }
    }
    bound
}

/// What a font rebuild depends on besides the installed workbook fonts: a language switch
/// rebuilds the fonts only when this changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontKey {
    han: HanOrder,
    thai: bool,
    /// Which set of host faces the fonts were built with.
    host: u64,
}

/// The [`FontKey`] of an interface language.
pub fn font_key(language: crate::i18n::Language) -> FontKey {
    FontKey {
        han: HanOrder::of(language),
        thai: language == crate::i18n::Language::Th,
        host: HOST_REVISION.load(std::sync::atomic::Ordering::Relaxed),
    }
}

/// Which Han face leads the fallback chain. Both Han faces cover kana and most kanji, so this only
/// decides glyph shapes (and which face catches a glyph the other lacks); switching between two
/// languages with the same order needs no font rebuild.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HanOrder {
    /// Japanese glyph shapes first (the interface is Japanese).
    Japanese,
    /// Simplified Chinese first: the widest Han coverage, for every other interface language.
    Chinese,
}

impl HanOrder {
    pub fn of(language: crate::i18n::Language) -> Self {
        if language == crate::i18n::Language::Ja { Self::Japanese } else { Self::Chinese }
    }
}

/// Builds the font set with the Han fallback faces ordered for the interface language.
pub fn font_definitions_for_language(language: crate::i18n::Language) -> FontDefinitions {
    font_definitions_with(language, &[])
}

/// The bundled faces plus the pinned craft-fonts faces, with no system-font probing or installed
/// workbook-font binding.
///
/// Integration tests use this through their shared harness, so a minimal CI image and a developer
/// machine see the same font set. Craft-fonts remains a pinned build input rather than a repository
/// asset; without it only egui's bundled faces are available, which is the documented no-font mode.
pub fn pinned_font_definitions() -> FontDefinitions {
    font_definitions_impl(crate::i18n::Language::En, &[], false)
}

/// [`font_definitions_for_language`] that also binds the workbook fonts in `used` that are
/// installed, so cells set in them render with them; the rest fall back to the roles. The bound
/// families are recorded in [`crate::fonts`] so [`cell_family`] only names a family egui has.
pub fn font_definitions_with(language: crate::i18n::Language, used: &[String]) -> FontDefinitions {
    font_definitions_impl(language, used, true)
}

/// Builds the role font chains, script fallbacks, installed workbook fonts and optional craft-font faces, drops unparseable data and records which families were bound.
fn font_definitions_impl(language: crate::i18n::Language, used: &[String], system: bool) -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let base_prop: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let base_mono: Vec<String> = fonts.families.get(&FontFamily::Monospace).cloned().unwrap_or_default();
    let mut bound: Vec<String> = Vec::new();
    // The non-Latin fallback faces have to sit at the end of each chain: a Latin font owns Latin,
    // and epaint picks the first face that has the glyph, so they only catch their own scripts.
    let fallback = if system { load_fallback(&mut fonts, HanOrder::of(language)) } else { Vec::new() };
    let hack = fonts.font_data.contains_key(HACK).then_some(HACK);
    for role in [UI, UI_BOLD, CELL, CELL_BOLD, CELL_ITALIC, CELL_BOLD_ITALIC, SERIF, MONO, MATH, MATH_ITALIC, MATH_BOLD] {
        let chain = build_role_chain(&mut fonts, role, &base_prop, &base_mono, &fallback, hack, system);
        fonts.families.insert(FontFamily::Name(role.into()), chain);
        bound.push(role.to_string());
    }
    // Installed workbook fonts, so a cell's chosen font actually renders in it.
    if system {
        for family in crate::fonts::installed_among(used) {
            bound.extend(crate::fonts::bind_family(&mut fonts, &family));
        }
    }
    // Interface text (formula bar, name box, dialogs) falls back to the cell font before the
    // script faces: whatever a cell can draw, the formula bar can draw too, even where no script
    // fallback face was found (#349: macOS UI font without Hebrew, cells in Arial with it).
    bind_cell_fallback(&mut fonts, &fallback);
    if system {
        bound.extend(apply_host_faces(&mut fonts, language, &host_faces()));
    }
    // Default proportional text in widgets uses the UI font.
    if let Some(ui) = fonts.families.get(&FontFamily::Name(UI.into())).cloned() {
        fonts.families.insert(FontFamily::Proportional, ui);
    }
    // The optional craft-fonts faces go at the very end of every family: a system or bundled face
    // that has the script keeps its glyph shapes, while the permitted OFL faces cover scripts and
    // symbols the host image lacks (Japanese, Korean, Thai, Arabic, Georgian and the UI symbols).
    // Chinese comes from the native-only OFL LXGW face when embedded; a system Han face still
    // leads, and the barred Noto CJK collection never enters.
    append_craft_faces(&mut fonts);
    drop_unparseable_fonts(&mut fonts);
    // Only families egui still has data for count as bound (a dropped face can empty a chain).
    bound.retain(|f| fonts.families.get(&FontFamily::Name(f.as_str().into())).is_some_and(|c| !c.is_empty()));
    crate::fonts::set_bound(bound);
    fonts
}

/// Builds one role family's chain: the loaded system face (or its role fallback), egui's bundled
/// proportional/monospace base, Hack's symbols, then the script fallbacks. Keys without font data
/// are dropped, as before.
fn build_role_chain(
    fonts: &mut FontDefinitions,
    role: &str,
    base_prop: &[String],
    base_mono: &[String],
    fallback: &[String],
    hack: Option<&str>,
    system: bool,
) -> Vec<String> {
    let paths = candidates(role);
    let refs: Vec<(&str, u32)> = paths.iter().map(|(p, i)| (p.as_str(), *i)).collect();
    let mut chain: Vec<String> = Vec::new();
    let loaded = if system { try_load(&refs) } else { None };
    if let Some(mut fd) = loaded {
        // Helvetica Neue Bold's own TrueType hints close Cyrillic е into ө at 13 pt on a Retina
        // screen (#1184). Core Text never runs them, so this draws it the way macOS does.
        if cfg!(target_os = "macos") && role == UI_BOLD {
            fd.tweak.hinting = Some(false);
        }
        let key = format!("sys-{role}");
        fonts.font_data.insert(key.clone(), Arc::new(fd));
        chain.push(key);
    } else if role == UI_BOLD {
        chain.push(format!("sys-{UI}"));
    } else if role.starts_with("cell-") {
        chain.push(format!("sys-{CELL}"));
    }
    // Equation faces fall back to the maths font (for the symbols a text face lacks), then to the
    // serif face.
    if role == MATH_ITALIC || role == MATH_BOLD {
        chain.push(format!("sys-{MATH}"));
    }
    if role.starts_with("math") {
        chain.push(format!("sys-{SERIF}"));
    }
    chain.extend(if role == MONO { base_mono.to_vec() } else { base_prop.to_vec() });
    // egui bundles Hack, a monospace face rich in arrows, triangles and keyboard symbols, but
    // 0.37 only puts it in the Monospace family. The interface draws ↑, ▲, ▼, ▸, ▾ and ↳ as
    // text (filter menus, ribbon, keytips), so add it behind the UI/cell and emoji faces; it
    // only catches characters those do not have.
    if let Some(hack) = hack
        && !chain.iter().any(|k| k.as_str() == hack)
    {
        chain.push(hack.to_string());
    }
    chain.extend(fallback.iter().cloned());
    chain.retain(|k| fonts.font_data.contains_key(k));
    chain
}

/// Inserts the loaded cell face before the script fallbacks of the interface families, so the
/// formula bar and dialogs can draw whatever a cell can (#349).
fn bind_cell_fallback(fonts: &mut FontDefinitions, fallback: &[String]) {
    let cell_key = format!("sys-{CELL}");
    if !fonts.font_data.contains_key(&cell_key) {
        return;
    }
    for role in [UI, UI_BOLD, SERIF, MONO] {
        if let Some(chain) = fonts.families.get_mut(&FontFamily::Name(role.into()))
            && !chain.contains(&cell_key)
        {
            let at = chain.iter().position(|k| fallback.contains(k)).unwrap_or(chain.len());
            chain.insert(at, cell_key.clone());
        }
    }
}

/// Appends the permitted craft-fonts faces to every family, preserving the leading system or
/// bundled face's glyph shapes.
fn append_craft_faces(fonts: &mut FontDefinitions) {
    let craft = add_craft_fonts(fonts);
    if craft.is_empty() {
        return;
    }
    for chain in fonts.families.values_mut() {
        for key in &craft {
            if !chain.contains(key) {
                chain.push(key.clone());
            }
        }
    }
}

#[cfg(test)]
mod font_binding_tests {
    use super::*;

    /// The families [`font_definitions`] always binds, whatever is installed.
    fn always_bound() -> Vec<String> {
        [UI, UI_BOLD, CELL, CELL_BOLD, CELL_ITALIC, CELL_BOLD_ITALIC, SERIF, MONO].iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_name_that_is_not_an_installed_font_gets_a_built_in_role() {
        let _g = crate::fonts::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Nothing installed: every name buckets to a role that font_definitions bound.
        crate::fonts::set_bound(always_bound());
        for name in ["", "Calibri", "Courier New", "Times New Roman", "Grossly Missing Font", "\u{1F600}"] {
            for (b, i) in [(false, false), (true, false), (false, true), (true, true)] {
                let FontFamily::Name(fam) = cell_family(name, b, i) else { panic!("{name:?} should name a family") };
                assert!(crate::fonts::is_bound(&fam), "{name:?} {b} {i} → unbound {fam:?}");
            }
        }
    }

    #[test]
    fn a_bound_installed_font_is_used_by_name() {
        let _g = crate::fonts::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let role = crate::fonts::role_key("Ubuntu", true, false);
        crate::fonts::set_bound(always_bound().into_iter().chain([role.clone()]));
        assert!(matches!(cell_family("Ubuntu", true, false), FontFamily::Name(f) if *f == *role));
        // The same family in a style that wasn't bound falls back to a built-in role that is.
        let FontFamily::Name(fallback) = cell_family("Ubuntu", false, true) else { panic!("should name a family") };
        assert!(crate::fonts::is_bound(&fallback), "the fallback is a bound role");
        crate::fonts::set_bound(always_bound());
    }

    #[test]
    fn host_faces_lead_the_interface_in_thai_and_trail_everywhere_else() {
        use crate::i18n::Language;
        let defaults = FontDefinitions::default();
        let Some(stand_in) = defaults.font_data.values().next().cloned() else { panic!("egui bundles a font") };
        let host: Vec<(HostFace, Arc<FontData>)> =
            [HostFace::Sarabun, HostFace::SarabunBold, HostFace::ThSarabunNew, HostFace::ThSarabunNewBold, HostFace::ThSarabunNewItalic]
                .into_iter()
                .map(|face| (face, Arc::clone(&stand_in)))
                .collect();
        let build = |language| {
            let mut fonts = FontDefinitions::default();
            for role in [UI, UI_BOLD, CELL, CELL_BOLD, CELL_ITALIC, CELL_BOLD_ITALIC, SERIF, MONO] {
                fonts.families.insert(FontFamily::Name(role.into()), vec!["Ubuntu-Light".to_string()]);
            }
            let _ = apply_host_faces(&mut fonts, language, &host);
            fonts
        };
        let chain = |fonts: &FontDefinitions, role: &str| fonts.families.get(&FontFamily::Name(role.into())).cloned().unwrap_or_default();
        let position = |chain: &[String], key: &str| chain.iter().position(|k| k == key);

        let thai = build(Language::Th);
        // The faces that only catch Thai letters draw larger than their size, so Thai matches the
        // Latin letters beside it; the same face leading a family keeps its own size.
        let scale = |key: &str| thai.font_data.get(key).map(|fd| fd.tweak.scale);
        assert_eq!(scale("host-th-sarabun-new-fallback"), Some(THAI_FALLBACK_SCALE));
        assert_eq!(scale("host-sarabun-fallback"), Some(THAI_FALLBACK_SCALE));
        assert_eq!(scale("host-th-sarabun-new"), Some(1.0));
        assert_eq!(chain(&thai, UI).first().map(String::as_str), Some("host-sarabun"));
        assert_eq!(chain(&thai, UI_BOLD).first().map(String::as_str), Some("host-sarabun-bold"));
        // Cells lead with Sarabun (bold ones with SemiBold) in every language, so Latin, digits and
        // Thai share one family; serif and monospace cells keep their own face first.
        for (role, lead) in
            [(CELL, "host-sarabun"), (CELL_ITALIC, "host-sarabun"), (CELL_BOLD, "host-sarabun-bold"), (CELL_BOLD_ITALIC, "host-sarabun-bold")]
        {
            assert_eq!(chain(&thai, role).first().map(String::as_str), Some(lead), "{role} leads with Sarabun");
        }
        for role in [SERIF, MONO] {
            assert_eq!(chain(&thai, role).first().map(String::as_str), Some("Ubuntu-Light"), "{role} keeps its own face first");
        }
        // The matching style is tried first: bold cells draw Thai in TH Sarabun New Bold, not Sarabun.
        let bold = chain(&thai, CELL_BOLD);
        assert!(position(&bold, "host-th-sarabun-new-bold-fallback") < position(&bold, "host-sarabun-fallback"));
        let regular = chain(&thai, CELL);
        assert!(position(&regular, "host-th-sarabun-new-fallback").is_some(), "TH Sarabun New still catches what Sarabun lacks");
        // No bold-italic face was supplied: that role still gets the others as fallbacks.
        assert!(chain(&thai, CELL_BOLD_ITALIC).iter().any(|k| k == "host-th-sarabun-new-fallback"));
        assert!(chain(&thai, MONO).iter().any(|k| k == "host-sarabun-fallback"), "every family can draw Thai");

        // Every other language: the same fallbacks, and the interface does not lead with Sarabun either.
        let english = build(Language::En);
        assert_eq!(chain(&english, UI).first().map(String::as_str), Some("Ubuntu-Light"));
        assert!(chain(&english, UI).iter().any(|k| k == "host-sarabun-fallback"));
        assert_eq!(chain(&english, CELL).first().map(String::as_str), Some("host-sarabun"));
        assert_eq!(chain(&thai, CELL), chain(&english, CELL), "cells are the same in Thai and English");
        let once = chain(&thai, UI).iter().filter(|k| *k == "host-sarabun").count();
        assert_eq!(once, 1, "a face is listed once per chain");
    }

    #[test]
    fn the_computers_interface_font_leads_the_menus_and_never_the_cells() {
        use crate::i18n::Language;
        let defaults = FontDefinitions::default();
        let Some(stand_in) = defaults.font_data.values().next().cloned() else { panic!("egui bundles a font") };
        // No bold Thai face was supplied: the regular one stands in for it.
        let host: Vec<(HostFace, Arc<FontData>)> =
            [HostFace::Sarabun, HostFace::SarabunBold, HostFace::SystemUi, HostFace::SystemUiBold, HostFace::SystemUiThai]
                .into_iter()
                .map(|face| (face, Arc::clone(&stand_in)))
                .collect();
        let mut fonts = FontDefinitions::default();
        for role in [UI, UI_BOLD, CELL, CELL_BOLD, CELL_ITALIC, CELL_BOLD_ITALIC, SERIF, MONO] {
            fonts.families.insert(FontFamily::Name(role.into()), vec!["Ubuntu-Light".to_string()]);
        }
        let _ = apply_host_faces(&mut fonts, Language::En, &host);
        let chain = |role: &str| fonts.families.get(&FontFamily::Name(role.into())).cloned().unwrap_or_default();
        assert_eq!(chain(UI)[..2], ["host-system-ui", "host-system-ui-thai"]);
        assert_eq!(chain(UI_BOLD)[..2], ["host-system-ui-bold", "host-system-ui-thai"], "bold Thai falls back to the regular face");
        for role in [CELL, CELL_BOLD, SERIF, MONO] {
            assert!(!chain(role).iter().any(|k| k.starts_with("host-system-ui")), "{role} never uses the interface font");
        }
        assert!(!fonts.font_data.contains_key("host-system-ui-fallback"), "no scaled fallback copy of a system face");
        // Registering faces changes the key the fonts were built under, so they are rebuilt.
        let before = font_key(Language::En);
        add_host_faces(vec![(HostFace::SystemUi, b"not a font".to_vec())]);
        assert_eq!(before, font_key(Language::En), "faces egui cannot read change nothing");
    }

    #[test]
    fn a_cell_can_ask_for_the_thai_faces_by_name() {
        use crate::i18n::Language;
        let defaults = FontDefinitions::default();
        let Some(stand_in) = defaults.font_data.values().next().cloned() else { panic!("egui bundles a font") };
        let host: Vec<(HostFace, Arc<FontData>)> = [HostFace::Sarabun, HostFace::SarabunBold, HostFace::ThSarabunNew, HostFace::ThSarabunNewBold]
            .into_iter()
            .map(|face| (face, Arc::clone(&stand_in)))
            .collect();
        let mut fonts = FontDefinitions::default();
        for role in [UI, UI_BOLD, CELL, CELL_BOLD, CELL_ITALIC, CELL_BOLD_ITALIC, SERIF, MONO] {
            fonts.families.insert(FontFamily::Name(role.into()), vec!["Ubuntu-Light".to_string()]);
        }
        let bound = apply_host_faces(&mut fonts, Language::En, &host);
        let chain = |name: &str, bold, italic| fonts.families.get(&FontFamily::Name(crate::fonts::role_key(name, bold, italic).into())).cloned();
        assert_eq!(bound.len(), 8, "four styles of each of the two names");
        assert_eq!(chain("TH Sarabun New", false, false).and_then(|c| c.first().cloned()).as_deref(), Some("host-th-sarabun-new"));
        assert_eq!(chain("TH Sarabun New", true, false).and_then(|c| c.first().cloned()).as_deref(), Some("host-th-sarabun-new-bold"));
        // No italic face was supplied: the regular one stands in for it.
        assert_eq!(chain("TH Sarabun New", false, true).and_then(|c| c.first().cloned()).as_deref(), Some("host-th-sarabun-new"));
        assert_eq!(chain("Sarabun", true, true).and_then(|c| c.first().cloned()).as_deref(), Some("host-sarabun-bold"));
        assert!(chain("Sarabun", false, false).is_some_and(|c| c.iter().any(|k| k == "Ubuntu-Light")), "symbols still come from the cell chain");
        // With no faces loaded (a desktop build) nothing is bound.
        let mut none = FontDefinitions::default();
        assert!(apply_host_faces(&mut none, Language::En, &[]).is_empty());
    }

    #[test]
    fn a_host_face_egui_cannot_read_is_skipped() {
        // Garbage leaves the registry as it was at startup (empty), so no other test sees a change.
        set_host_faces(vec![(HostFace::Sarabun, b"not a font".to_vec()), (HostFace::ThSarabunNew, Vec::new())]);
        assert!(host_faces().is_empty());
    }

    #[test]
    fn font_keys_change_only_with_han_order_or_thai() {
        use crate::i18n::Language;
        assert_eq!(font_key(Language::En), font_key(Language::De));
        assert_ne!(font_key(Language::En), font_key(Language::Th));
        assert_ne!(font_key(Language::En), font_key(Language::Ja));
        assert_ne!(font_key(Language::Th), font_key(Language::Ja));
    }

    #[test]
    fn arbitrary_names_never_yield_an_unbound_family() {
        let _g = crate::fonts::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // A workbook can hold any font name; none may name a family egui has no data for.
        crate::fonts::set_bound(always_bound());
        for seed in 0..500u32 {
            let name: String =
                (0..(seed % 40)).map(|k| char::from_u32(0x20 + (seed.wrapping_mul(31).wrapping_add(k) % 0x5F)).unwrap_or('x')).collect();
            let FontFamily::Name(fam) = cell_family(&name, seed % 2 == 0, seed % 3 == 0) else { continue };
            assert!(crate::fonts::is_bound(&fam), "{name:?} → unbound {fam:?}");
        }
    }
}
