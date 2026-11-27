//! KrubOffice's accent colours on Tabler drawings (style A, approved by the owner 2026-11-24).
//!
//! The Tabler outlines stay as they are; the part of an icon that carries its meaning (the clip of
//! Paste, the plus of Grow Font, the red X of Clear Formatting) is drawn in a colour, sometimes with
//! a soft tint inside, and everything else keeps the caller's ink. This is a KrubOffice modification
//! of Tabler Icons (MIT, see `LICENSE-tabler.txt`). It is hand-written, kept apart from the generated
//! `tabler.rs` so regenerating that file keeps the colours.
//!
//! Colour is off unless the caller turns it on for a stretch of drawing with [`with_colour`]; the
//! other apps and every surface that doesn't ask keep monochrome icons. Disabled icons never colour.

use egui::{Color32, Context, Id, Painter, Pos2, Shape};

/// One accent hue, with a lighter shade for dark themes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hue {
    Blue,
    Red,
    Green,
    Orange,
    Purple,
    Teal,
    Gold,
}

impl Hue {
    /// The hue on a light background, and on a dark one.
    fn rgb(self) -> ([u8; 3], [u8; 3]) {
        match self {
            Hue::Blue => ([0x25, 0x63, 0xEB], [0x60, 0xA5, 0xFA]),
            Hue::Red => ([0xDC, 0x26, 0x26], [0xF8, 0x71, 0x71]),
            Hue::Green => ([0x16, 0xA3, 0x4A], [0x4A, 0xDE, 0x80]),
            Hue::Orange => ([0xEA, 0x58, 0x0C], [0xFB, 0x92, 0x3C]),
            Hue::Purple => ([0x7C, 0x3A, 0xED], [0xA7, 0x8B, 0xFA]),
            Hue::Teal => ([0x0D, 0x94, 0x88], [0x2D, 0xD4, 0xBF]),
            Hue::Gold => ([0xD9, 0x9A, 0x00], [0xF2, 0xB7, 0x05]),
        }
    }
}

/// How one outline of a drawing is coloured: its stroke in `hue` (unless `stroke` is off, when it
/// keeps the ink), and with `tint` a soft fill of the same hue inside it. Outlines are counted from 0 in drawing order; each `M` or circle starts one.
#[derive(Clone, Copy, Debug)]
pub struct Part {
    pub index: u8,
    pub hue: Hue,
    pub stroke: bool,
    pub tint: bool,
}

/// The index that names a drawing's solid (pre-filled) shapes rather than an outline.
pub const SOLID: u8 = u8::MAX;

/// Stroke in colour.
const fn s(index: u8, hue: Hue) -> Part {
    Part { index, hue, stroke: true, tint: false }
}

/// Stroke in colour, tinted inside.
const fn t(index: u8, hue: Hue) -> Part {
    Part { index, hue, stroke: true, tint: true }
}

/// Ink stroke, tinted inside.
const fn f(index: u8, hue: Hue) -> Part {
    Part { index, hue, stroke: false, tint: true }
}

use Hue::{Blue, Gold, Green, Orange, Purple, Red, Teal};

/// The coloured parts of each Tabler drawing (by its Tabler name), KhianKrub's Home ribbon first.
const TABLE: &[(&str, &[Part])] = &[
    ("clipboard", &[t(0, Blue), s(1, Orange)]),
    ("scissors", &[t(0, Red), t(1, Red)]),
    ("copy", &[t(0, Blue)]),
    ("brush", &[t(0, Gold)]),
    ("underline", &[s(1, Blue)]),
    ("strikethrough", &[s(0, Red)]),
    ("subscript", &[s(2, Purple)]),
    ("superscript", &[s(2, Purple)]),
    ("text-increase", &[s(2, Green), s(3, Green)]),
    ("text-decrease", &[s(2, Red)]),
    ("letter-case", &[t(0, Purple), s(3, Purple)]),
    ("clear-formatting", &[s(0, Red), s(1, Red)]),
    ("sparkles", &[t(0, Orange), t(1, Blue), t(2, Purple)]),
    ("highlight", &[t(0, Gold), s(3, Gold)]),
    ("text-color", &[s(2, Red)]),
    ("list", &[s(3, Blue), s(4, Blue), s(5, Blue)]),
    ("list-numbers", &[s(3, Blue), s(4, Blue)]),
    ("list-tree", &[s(3, Blue), s(4, Teal), s(5, Purple)]),
    ("indent-decrease", &[s(3, Orange)]),
    ("indent-increase", &[s(3, Green)]),
    ("arrows-sort", &[s(0, Blue), s(1, Blue), s(2, Orange), s(3, Orange)]),
    ("pilcrow", &[t(2, Blue)]),
    ("line-height", &[s(0, Teal), s(1, Teal), s(2, Teal)]),
    ("paint", &[t(0, Orange)]),
    ("search", &[t(0, Blue)]),
    ("replace", &[t(0, Blue), t(1, Orange)]),
    ("pointer", &[t(0, Blue)]),
    ("microphone", &[t(0, Red)]),
    ("writing-sign", &[s(0, Purple)]),
    // Insert (2026-11-24)
    ("file-description", &[f(1, Blue), s(2, Blue), s(3, Blue)]),
    ("file", &[f(1, Blue)]),
    ("page-break", &[s(2, Orange), s(3, Orange), s(4, Orange)]),
    ("table", &[f(0, Blue)]),
    ("pencil", &[t(0, Gold)]),
    ("table-plus", &[f(0, Blue), s(3, Green), s(4, Green)]),
    ("photo", &[s(0, Gold), s(2, Green), s(3, Green)]),
    ("shape", &[t(0, Blue), t(1, Blue), t(2, Blue), t(3, Blue)]),
    ("icons", &[t(0, Orange), t(1, Green), s(2, Red), s(3, Red), t(4, Blue)]),
    ("sitemap", &[t(0, Teal), t(1, Teal), t(2, Blue)]),
    ("chart-bar", &[t(0, Blue), t(1, Teal), t(2, Orange)]),
    ("chalkboard", &[t(0, Teal)]),
    ("link", &[s(1, Blue), s(2, Blue)]),
    ("bookmark", &[t(0, Red)]),
    ("message-plus", &[s(3, Green), s(4, Green)]),
    ("layout-navbar", &[s(1, Blue)]),
    ("layout-bottombar", &[s(1, Blue)]),
    ("file-digit", &[s(1, Blue), s(3, Blue)]),
    ("text-resize", &[t(0, Blue), t(1, Blue), t(2, Blue), t(3, Blue)]),
    ("puzzle", &[t(0, Purple)]),
    ("letter-a", &[s(0, Purple)]),
    ("signature", &[s(0, Purple)]),
    ("calendar", &[s(1, Red), s(2, Red), s(4, Red), s(5, Red)]),
    ("box", &[t(0, Orange)]),
    ("math-function", &[s(2, Blue), s(3, Blue)]),
    ("omega", &[s(0, Purple)]),
    // Design (2026-11-24; line-height, sparkles and paint are coloured above)
    ("palette", &[t(1, Red), t(2, Green), t(3, Blue)]),
    ("typography", &[s(2, Purple), s(3, Purple)]),
    ("star", &[t(0, Gold)]),
    ("folder-open", &[t(0, Gold)]),
    ("droplet", &[t(0, Blue)]),
    ("border-outer", &[s(0, Blue)]),
    // Layout (2026-11-24; list-numbers is coloured above)
    ("text-direction-ltr", &[s(0, Blue), s(1, Blue)]),
    ("box-margin", &[t(0, Blue)]),
    ("rotate-rectangle", &[f(0, Blue)]),
    ("dimensions", &[s(0, Blue), s(1, Blue), s(2, Blue), s(3, Blue), s(4, Blue), s(5, Blue)]),
    ("columns", &[s(0, Blue), s(1, Blue), s(2, Blue), s(3, Blue), s(4, Teal), s(5, Teal), s(6, Teal), s(7, Teal)]),
    ("separator", &[s(1, Orange)]),
    ("minus", &[s(0, Orange)]),
    ("text-wrap", &[s(2, Blue), s(3, Blue)]),
    ("stack-front", &[s(0, Blue), s(SOLID, Blue)]),
    ("stack-back", &[s(1, Blue), s(SOLID, Blue)]),
    ("layout-sidebar", &[s(1, Blue)]),
    ("layout-align-left", &[t(1, Blue)]),
    // References (2026-11-24; list-numbers, superscript, brush and link are coloured above)
    ("refresh", &[s(0, Blue), s(1, Blue), s(2, Blue), s(3, Blue)]),
    ("text-plus", &[s(4, Green), s(5, Green)]),
    ("file-text", &[s(4, Purple)]),
    ("arrow-down", &[s(0, Blue), s(1, Blue), s(2, Blue)]),
    ("notes", &[s(1, Purple), s(2, Purple), s(3, Purple)]),
    ("info-square", &[s(0, Blue), s(2, Blue)]),
    ("zoom-question", &[t(0, Blue), s(2, Orange), s(3, Orange)]),
    ("quote", &[s(0, Purple), s(1, Purple)]),
    ("address-book", &[s(1, Purple), s(2, Purple)]),
    ("books", &[t(4, Purple), s(5, Purple), s(6, Purple)]),
    ("text-caption", &[s(0, Blue), s(2, Blue)]),
    ("tag", &[t(1, Red)]),
    ("list-letters", &[s(3, Blue), s(4, Blue), s(5, Blue)]),
    ("bookmark-plus", &[t(0, Red), s(1, Green), s(2, Green)]),
    ("gavel", &[t(4, Orange)]),
    // Review (2026-11-24; message-plus and arrow-down are coloured above)
    ("abc", &[s(0, Purple), s(1, Purple), s(2, Purple), s(3, Purple)]),
    ("book", &[s(0, Purple), s(1, Purple)]),
    ("numbers", &[s(0, Blue), s(1, Blue), s(2, Blue), s(3, Blue)]),
    ("volume", &[s(0, Blue), s(1, Blue)]),
    ("accessible", &[s(1, Blue), s(2, Blue), s(3, Blue), s(4, Blue), s(SOLID, Blue)]),
    ("language", &[s(3, Blue), s(4, Blue)]),
    ("message-x", &[s(3, Red), s(4, Red)]),
    ("message-reply", &[s(1, Blue), s(2, Blue)]),
    ("message-forward", &[s(1, Blue), s(2, Blue)]),
    ("messages", &[t(0, Gold)]),
    ("git-compare", &[s(2, Blue), s(3, Blue), s(4, Blue), s(5, Blue)]),
    ("message-dots", &[s(0, Blue), s(1, Blue), s(2, Blue)]),
    ("file-check", &[s(2, Green)]),
    ("file-x", &[s(2, Red), s(3, Red)]),
    ("arrow-up", &[s(0, Blue), s(1, Blue), s(2, Blue)]),
    ("shield-lock", &[t(0, Gold)]),
    ("scribble-off", &[s(4, Red)]),
    // View (2026-11-24; book, file-text, list-tree, file, layout-sidebar and palette are coloured above)
    ("file-pencil", &[t(2, Gold)]),
    ("world", &[t(0, Blue)]),
    ("focus-centered", &[t(0, Blue)]),
    ("eyeglass", &[t(3, Blue), t(4, Blue)]),
    ("ruler", &[t(0, Gold)]),
    ("grid-4x4", &[s(1, Blue), s(4, Blue)]),
    ("border-corners", &[s(0, Teal), s(1, Teal), s(2, Teal), s(3, Teal)]),
    ("fold", &[s(0, Blue), s(1, Blue), s(2, Blue), s(3, Blue), s(4, Orange), s(5, Orange), s(6, Orange), s(7, Orange)]),
    ("file-search", &[t(2, Blue)]),
    ("zoom", &[t(0, Blue)]),
    ("zoom-out", &[t(0, Blue)]),
    ("zoom-in", &[t(0, Blue)]),
    ("files", &[f(1, Blue), f(2, Blue)]),
    ("arrows-horizontal", &[s(0, Blue), s(1, Blue), s(2, Blue)]),
    ("app-window", &[f(0, Blue)]),
    ("layout-grid", &[t(0, Blue)]),
    ("arrows-split", &[s(0, Blue), s(2, Blue), s(1, Teal), s(3, Teal)]),
    ("maximize", &[s(0, Blue), s(1, Blue), s(2, Blue), s(3, Blue)]),
    ("template", &[t(0, Blue), f(1, Blue)]),
    // Side panels, batch 1 (2026-11-24): Text, Styles and Navigation
    ("square-letter-a", &[s(0, Blue)]),
    ("circle-letter-a", &[s(0, Blue)]),
    ("letter-spacing", &[s(3, Teal), s(4, Teal), s(5, Teal)]),
    ("layout-sidebar-right", &[s(1, Blue)]),
    ("heading", &[s(0, Purple), s(1, Purple), s(2, Purple), s(3, Purple), s(4, Purple), s(5, Purple), s(6, Purple)]), // Side panels, batch 2 (2026-11-24): the object panes
    ("eraser", &[t(0, Red)]),
    (
        "table-dashed",
        &[
            s(0, Blue),
            s(1, Blue),
            s(2, Blue),
            s(3, Blue),
            s(4, Blue),
            s(5, Blue),
            s(6, Blue),
            s(7, Blue),
            s(8, Blue),
            s(9, Blue),
            s(10, Blue),
            s(11, Blue),
        ],
    ),
    ("row-insert-top", &[s(1, Green), s(2, Green)]),
    ("row-insert-bottom", &[s(1, Green), s(2, Green)]),
    ("column-insert-left", &[s(1, Green), s(2, Green)]),
    ("column-insert-right", &[s(1, Green), s(2, Green)]),
    ("row-remove", &[s(1, Red), s(2, Red)]),
    ("column-remove", &[s(1, Red), s(2, Red)]),
    ("square-x", &[s(1, Red), s(2, Red)]),
    ("table-minus", &[s(3, Red)]),
    ("arrows-join", &[s(0, Blue), s(1, Blue), s(2, Blue)]),
    ("separator-vertical", &[s(0, Orange), s(1, Blue), s(2, Blue)]),
    ("separator-horizontal", &[s(0, Orange), s(1, Blue), s(2, Blue)]),
    ("box-padding", &[s(1, Teal), s(2, Teal), s(3, Teal), s(4, Teal), s(5, Teal), s(6, Teal), s(7, Teal), s(8, Teal)]),
    ("table-row", &[s(1, Blue), s(2, Blue), s(3, Blue), s(4, Blue)]),
    ("table-export", &[s(3, Blue), s(4, Blue)]),
    ("adjustments-alt", &[t(0, Blue), t(3, Blue), t(6, Blue)]),
    ("color-filter", &[s(0, Blue), s(1, Green), s(2, Red)]),
    ("droplet-half-2", &[t(0, Blue)]),
    ("photo-edit", &[t(4, Gold)]),
    ("restore", &[s(0, Blue), s(1, Blue)]),
    ("photo-star", &[t(3, Gold)]),
    ("rotate", &[s(0, Blue), s(1, Blue)]),
    ("box-multiple", &[f(0, Blue)]),
    ("crop", &[s(0, Blue)]),
    ("shadow", &[s(1, Blue), s(2, Blue), s(3, Blue), s(4, Blue), s(5, Blue)]),
    ("flip-horizontal", &[t(1, Blue)]),
    ("flip-vertical", &[t(1, Blue)]),
    ("transform", &[t(0, Blue), t(5, Blue)]),
    ("chart-arrows", &[s(1, Blue), s(2, Blue), s(5, Blue)]),
    ("square-plus", &[s(0, Green), s(1, Green)]),
    ("text-size", &[s(0, Purple), s(1, Purple), s(2, Purple)]),
    ("arrow-bar-left", &[s(0, Orange), s(1, Orange), s(2, Orange)]),
    ("arrow-bar-right", &[s(0, Green), s(1, Green), s(2, Green)]),
    // Side panels, batch 3 (2026-11-25): the tool panes
    ("list-details", &[t(4, Gold), t(5, Gold)]),
    ("trash", &[s(0, Red), s(4, Red)]), // The less common and contextual tabs (2026-11-25)
    ("lasso", &[s(0, Blue)]),
    ("ballpen", &[t(1, Blue)]),
    ("pencil-plus", &[t(0, Gold), s(2, Green), s(3, Green)]),
    ("color-picker", &[t(1, Red)]),
    ("math-symbols", &[s(2, Blue), s(3, Blue), s(4, Blue), s(5, Blue), s(6, Blue), s(7, Blue), s(8, Blue)]),
    ("rotate-2", &[s(0, Blue), s(1, Blue)]),
    ("mail", &[s(1, Blue)]),
    ("tags", &[t(0, Red)]),
    ("braces", &[s(0, Purple), s(1, Purple)]),
    ("eye", &[t(0, Blue)]),
    ("player-track-prev", &[t(0, Blue), t(1, Blue)]),
    ("player-track-next", &[t(0, Blue), t(1, Blue)]),
    ("alert-triangle", &[t(1, Orange)]),
    ("flag", &[t(0, Green)]),
    ("ruler-2", &[t(0, Gold)]),
    ("cursor-text", &[s(0, Purple), s(1, Purple), s(2, Purple)]),
    ("photo-square-rounded", &[s(1, Gold), s(2, Green), s(3, Green)]),
    ("square-check", &[s(1, Green)]),
    ("forms", &[s(0, Blue), s(1, Blue)]),
    ("select", &[s(1, Blue)]),
    ("calendar-event", &[s(1, Red), s(2, Red), s(4, Red)]),
    ("repeat", &[s(0, Blue), s(1, Blue), s(2, Blue), s(3, Blue)]),
    ("help-circle", &[s(1, Blue), s(2, Blue)]),
    ("code", &[s(0, Blue), s(1, Blue)]),
    ("note", &[t(1, Gold)]),
    ("file-settings", &[f(8, Blue)]),
    ("unlink", &[s(0, Red), s(2, Blue), s(3, Blue), s(4, Red), s(5, Red), s(6, Red)]),
    ("arrow-bar-to-up", &[s(0, Orange), s(1, Orange), s(2, Orange)]),
    ("plus", &[s(0, Green), s(1, Green)]),
    // Right-click menu, mini toolbar and status bar (2026-11-25)
    ("number-123", &[s(0, Blue), s(1, Blue), s(2, Blue)]),
    ("check", &[s(0, Green)]),
    ("device-floppy", &[t(2, Blue)]),
    ("arrow-forward-up", &[s(0, Blue), s(1, Blue)]),
    // KhitKrub and PhutKrub ribbons (2026-11-25)
    ("adjustments", &[t(0, Blue), t(3, Blue), t(6, Blue)]),
    ("adjustments-horizontal", &[t(0, Blue), t(3, Blue), t(6, Blue)]),
    ("arrow-bar-to-down", &[s(1, Blue), s(2, Blue), s(3, Blue)]),
    ("arrows-diagonal", &[s(0, Blue), s(1, Blue), s(2, Blue), s(3, Blue)]),
    ("arrows-maximize", &[s(0, Blue), s(1, Blue), s(2, Blue), s(3, Blue), s(4, Blue), s(5, Blue), s(6, Blue), s(7, Blue)]),
    ("arrows-right", &[s(0, Teal), s(1, Blue), s(2, Teal), s(3, Blue)]),
    ("arrows-split-2", &[s(0, Blue), s(1, Teal), s(2, Teal), s(3, Blue)]),
    ("arrows-up-down", &[s(0, Blue), s(1, Blue), s(2, Teal), s(3, Teal)]),
    ("aspect-ratio", &[f(0, Blue), s(1, Blue), s(2, Blue)]),
    ("background", &[s(0, Teal), s(1, Teal), s(2, Teal), s(3, Teal), s(4, Teal)]),
    ("bolt", &[t(0, Gold)]),
    ("bulb", &[s(0, Gold), s(1, Gold), s(2, Gold), s(3, Gold), s(4, Gold), t(5, Gold)]),
    ("calculator", &[f(0, Blue), s(1, Blue)]),
    ("chart-dots", &[t(1, Blue), t(2, Orange), t(3, Teal)]),
    ("chart-infographic", &[t(0, Blue), s(1, Blue), s(2, Teal), s(3, Orange), s(4, Teal), s(5, Orange)]),
    ("chart-line", &[s(1, Blue)]),
    ("chart-pie", &[t(0, Blue), t(1, Orange)]),
    ("checkbox", &[s(0, Green)]),
    ("checklist", &[s(1, Green)]),
    ("chevron-left", &[s(0, Blue)]),
    ("chevron-right", &[s(0, Blue)]),
    ("click", &[s(0, Orange), s(1, Orange), s(2, Orange), s(3, Orange), s(4, Orange), t(5, Blue)]),
    ("clock-play", &[s(0, Blue), t(1, Green)]),
    ("color-swatch", &[t(0, Blue), t(1, Green), t(2, Red)]),
    ("columns-2", &[s(1, Orange)]),
    ("copy-check", &[t(0, Blue), s(2, Green)]),
    ("copy-x", &[t(0, Blue), s(2, Red), s(3, Red)]),
    ("currency-dollar", &[s(0, Green), s(1, Green), s(2, Green)]),
    ("cut", &[t(0, Red), t(1, Red)]),
    ("filter", &[t(0, Blue)]),
    ("history", &[s(1, Blue), s(2, Blue)]),
    ("id", &[f(0, Blue), t(1, Purple)]),
    ("layout", &[f(0, Blue), f(1, Blue), f(2, Blue)]),
    ("layout-align-bottom", &[t(1, Blue)]),
    ("layout-align-middle", &[t(2, Blue)]),
    ("layout-align-top", &[t(1, Blue)]),
    ("list-check", &[s(0, Green), s(1, Green), s(2, Green)]),
    ("movie", &[t(0, Blue)]),
    ("number", &[s(0, Blue), s(1, Blue), s(2, Blue)]),
    ("percentage", &[s(0, Blue), s(1, Blue), s(2, Blue)]),
    ("player-pause", &[t(0, Orange), t(1, Orange)]),
    ("player-play", &[t(0, Green)]),
    ("player-record", &[t(0, Red)]),
    ("presentation", &[f(1, Blue), s(4, Orange)]),
    ("presentation-analytics", &[s(0, Orange), s(1, Orange), s(2, Orange), f(4, Blue)]),
    ("presentation-off", &[s(8, Red)]),
    ("printer", &[t(2, Blue)]),
    ("robot", &[t(0, Teal), s(1, Teal)]),
    ("route", &[t(0, Green), t(1, Red), s(2, Blue)]),
    ("ruler-measure", &[t(0, Gold)]),
    ("screenshot", &[s(0, Blue), s(1, Blue), s(2, Blue), s(3, Blue), s(4, Blue), s(5, Blue), s(6, Green), s(7, Green), s(8, Blue)]),
    ("script", &[t(0, Purple)]),
    ("section", &[t(10, Blue)]),
    (
        "snowflake",
        &[
            s(0, Blue),
            s(1, Blue),
            s(2, Blue),
            s(3, Blue),
            s(4, Blue),
            s(5, Blue),
            s(6, Blue),
            s(7, Blue),
            s(8, Blue),
            s(9, Blue),
            s(10, Blue),
            s(11, Blue),
        ],
    ),
    ("sort-ascending", &[s(3, Blue), s(4, Blue)]),
    ("sort-descending", &[s(3, Orange), s(4, Orange)]),
    ("square", &[s(0, Blue)]),
    ("square-rounded", &[f(0, Blue)]),
    ("stack-2", &[t(0, Blue)]),
    ("sum", &[s(0, Blue)]),
    ("table-column", &[f(0, Blue), s(3, Orange), s(4, Orange), s(5, Orange), s(6, Orange)]),
    ("table-heart", &[f(0, Blue), t(3, Red)]),
    ("table-options", &[f(0, Blue), t(3, Gold), s(4, Gold), s(5, Gold), s(6, Gold), s(7, Gold), s(8, Gold), s(9, Gold)]),
    ("text-direction-rtl", &[s(3, Blue), s(4, Blue)]),
    ("text-recognition", &[s(0, Blue), s(1, Blue), s(2, Blue), s(3, Blue), s(4, Purple), s(5, Purple)]),
    ("transition-left", &[s(2, Red), s(3, Red)]),
    ("transition-right", &[s(2, Green), s(3, Green)]),
    ("user-square", &[t(0, Blue), s(1, Blue)]),
    ("zoom-in-area", &[t(2, Blue)]),
    ("zoom-reset", &[s(1, Blue), s(2, Blue)]),
    // KhitKrub and PhutKrub panels, menus, status bars and dialogs (2026-11-25)
    ("arrows-minimize", &[s(0, Blue), s(1, Blue), s(2, Blue), s(3, Blue), s(4, Blue), s(5, Blue), s(6, Blue), s(7, Blue)]),
    ("eye-off", &[s(0, Blue), s(3, Red)]),
    ("layout-align-center", &[t(2, Blue)]),
    ("layout-align-right", &[t(1, Blue)]),
    ("lock", &[t(0, Gold)]),
    ("lock-open", &[t(0, Gold)]),
    ("player-stop", &[t(0, Red)]),
    ("stack-pop", &[s(2, Blue), s(3, Blue)]),
    ("stack-push", &[s(2, Blue), s(3, Blue)]),
    ("stairs", &[s(0, Blue)]),
];

/// The coloured parts of a Tabler drawing (by its Tabler name), or none.
pub fn parts(tabler: &str) -> &'static [Part] {
    TABLE.iter().find(|(name, _)| *name == tabler).map_or(&[], |(_, parts)| parts)
}

fn flag() -> Id {
    crate::pane_ui::id("krub_icons_tabler_colour")
}

/// Run `draw` with accent colours turned on (or off) for every Tabler icon it paints, then restore
/// the previous setting.
pub fn with_colour<R>(ctx: &Context, on: bool, draw: impl FnOnce() -> R) -> R {
    let before = enabled(ctx);
    ctx.data_mut(|d| d.insert_temp(flag(), on));
    let out = draw();
    ctx.data_mut(|d| d.insert_temp(flag(), before));
    out
}

/// Whether accent colours are on for icons painted now.
pub fn enabled(ctx: &Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(flag())).unwrap_or(false)
}

/// The stroke colour of outline `index`, and its tint when it has one: the ink for uncoloured
/// parts. The hue takes the ink's opacity, and its dark-theme shade when the ink is light.
pub fn colours(parts: &[Part], index: usize, ink: Color32) -> (Color32, Option<Color32>) {
    let Some(part) = parts.iter().find(|p| usize::from(p.index) == index) else {
        return (ink, None);
    };
    let [r, g, b, a] = ink.to_srgba_unmultiplied();
    let light_ink = u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114 > 128_000;
    let (on_light, on_dark) = part.hue.rgb();
    let [hr, hg, hb] = if light_ink { on_dark } else { on_light };
    let hue = Color32::from_rgba_unmultiplied(hr, hg, hb, a);
    let strength = if light_ink { 0.26 } else { 0.18 };
    let tint = part.tint.then(|| hue.gamma_multiply(strength));
    (if part.stroke { hue } else { ink }, tint)
}

/// Fill the inside of `outline` (closed from its last point back to its first) in `colour`.
pub fn fill(p: &Painter, outline: &[Pos2], colour: Color32) {
    let mut poly: Vec<Pos2> = Vec::with_capacity(outline.len());
    for &q in outline {
        if q.is_finite() && poly.last().is_none_or(|l: &Pos2| l.distance(q) > 0.01) {
            poly.push(q);
        }
    }
    while poly.len() > 3 && poly.first().zip(poly.last()).is_some_and(|(a, b)| a.distance(*b) <= 0.01) {
        poly.pop();
    }
    let triangles = triangulate(&poly);
    if triangles.is_empty() {
        return;
    }
    let mut mesh = egui::Mesh::default();
    for q in &poly {
        mesh.colored_vertex(*q, colour);
    }
    for [a, b, c] in triangles {
        mesh.add_triangle(a, b, c);
    }
    p.add(Shape::mesh(mesh));
}

/// Ear clipping for a simple polygon (any winding): triangles as vertex indices.
fn triangulate(poly: &[Pos2]) -> Vec<[u32; 3]> {
    let n = poly.len();
    if !(3..=4096).contains(&n) {
        return Vec::new();
    }
    let area: f32 = (0..n).map(|i| poly[i].x * poly[(i + 1) % n].y - poly[(i + 1) % n].x * poly[i].y).sum();
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let cross = |o: Pos2, a: Pos2, b: Pos2| ((a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)) * sign;
    let mut left: Vec<usize> = (0..n).collect();
    let mut out = Vec::with_capacity(n - 2);
    let mut guard = 0;
    while left.len() > 3 && guard < n * n {
        guard += 1;
        let m = left.len();
        let mut clipped = false;
        for i in 0..m {
            let (ia, ib, ic) = (left[(i + m - 1) % m], left[i], left[(i + 1) % m]);
            let (a, b, c) = (poly[ia], poly[ib], poly[ic]);
            if cross(a, b, c) <= 0.0 {
                continue;
            }
            let inside = left.iter().any(|&j| {
                j != ia && j != ib && j != ic && {
                    let q = poly[j];
                    cross(a, b, q) >= 0.0 && cross(b, c, q) >= 0.0 && cross(c, a, q) >= 0.0
                }
            });
            if !inside {
                out.push([ia as u32, ib as u32, ic as u32]);
                left.remove(i);
                clipped = true;
                break;
            }
        }
        if !clipped {
            // Not simple (or degenerate): drop the sharpest-turning vertex and go on.
            left.remove(0);
        }
    }
    if let [a, b, c] = left[..] {
        out.push([a as u32, b as u32, c as u32]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coloured_parts_exist_in_their_drawings() {
        for (tabler, parts) in TABLE {
            let count = crate::tabler::outline_count(tabler);
            assert!(count.is_some(), "{tabler} is not a Tabler drawing in use");
            let count = count.unwrap_or(0);
            for part in parts.iter().filter(|p| p.index != SOLID) {
                assert!(usize::from(part.index) < count, "{tabler} part {} of {count}", part.index);
            }
        }
    }

    /// Paint `name` and say whether any vertex took a hue (red, green and blue not all equal).
    fn paints_a_hue(name: &str, colour_on: bool, disabled: bool) -> bool {
        let ctx = egui::Context::default();
        ctx.begin_pass(egui::RawInput::default());
        let p = ctx.layer_painter(egui::LayerId::background());
        let ink = Color32::from_rgb(0x42, 0x42, 0x42);
        let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(32.0, 32.0));
        let drawn = with_colour(&ctx, colour_on, || crate::paint_named(&p, rect, name, ink, false, disabled));
        assert!(drawn, "{name}");
        assert!(!enabled(&ctx), "the setting is restored after drawing");
        let mut output = ctx.end_pass();
        output.textures_delta.clear();
        ctx.tessellate(output.shapes, output.pixels_per_point).into_iter().any(|primitive| match primitive.primitive {
            egui::epaint::Primitive::Mesh(mesh) => mesh.vertices.iter().any(|v| {
                let [r, g, b, _] = v.color.to_array();
                r != g || g != b
            }),
            egui::epaint::Primitive::Callback(_) => false,
        })
    }

    #[test]
    fn colour_only_when_asked_and_never_when_disabled() {
        for name in [
            "paste",
            "currency",
            "freeze",
            "cond-format",
            "pivot-table",
            "play",
            "anim-entrance",
            "hide-slide",
            "design-ideas",
            "cut",
            "bullets",
            "clear",
            "find",
            "dictate",
            "table",
            "picture",
            "chart",
            "bookmark",
            "equation",
            "themes",
            "watermark",
            "setDefault",
            "margins",
            "columns",
            "selectionPane",
            "footnote",
            "citation",
            "markEntry",
            "tableOfAuthorities",
            "spelling",
            "accept",
            "restrict",
            "hideInk",
            "ruler",
            "zoomIn",
            "textBoundaries",
            "fullScreen",
            "charborder",
            "char-spacing",
            "stylesPane",
            "chartTitle",
            "insertAbove",
            "del_row",
            "picColor",
            "crop",
            "addShape",
            "tableOfFigures",
            "delete",
            "lasso",
            "envelope",
            "ccCheckBox",
            "unlinkCitations",
            "outlineExpand",
            "arabicDigits",
            "check",
        ] {
            assert!(paints_a_hue(name, true, false), "{name} coloured");
            assert!(!paints_a_hue(name, false, false), "{name} monochrome by default");
            assert!(!paints_a_hue(name, true, true), "{name} monochrome when disabled");
        }
        // Solid shapes take their colour too: Bring Forward's front layer.
        assert!(paints_a_hue("bringForward", true, false));
        // Bold keeps the ink: nothing in it carries a colour.
        assert!(!paints_a_hue("bold", true, false));
    }

    #[test]
    fn concave_shapes_triangulate_fully() {
        let star = [
            Pos2::new(0.0, -10.0),
            Pos2::new(2.0, -2.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(2.0, 2.0),
            Pos2::new(0.0, 10.0),
            Pos2::new(-2.0, 2.0),
            Pos2::new(-10.0, 0.0),
            Pos2::new(-2.0, -2.0),
        ];
        assert_eq!(triangulate(&star).len(), 6);
        let reversed: Vec<Pos2> = star.iter().rev().copied().collect();
        assert_eq!(triangulate(&reversed).len(), 6);
        assert!(triangulate(&star[..2]).is_empty());
    }

    #[test]
    fn colour_keeps_ink_alpha_and_picks_theme_shade() {
        let parts = [s(0, Blue), t(1, Red), f(2, Green)];
        let dark_ink = Color32::from_rgb(0x42, 0x42, 0x42);
        assert_eq!(colours(&parts, 0, dark_ink).0, Color32::from_rgb(0x25, 0x63, 0xEB));
        let light_ink = Color32::from_rgb(0xDD, 0xDD, 0xDD);
        assert_eq!(colours(&parts, 0, light_ink).0, Color32::from_rgb(0x60, 0xA5, 0xFA));
        assert_eq!(colours(&parts, 5, dark_ink), (dark_ink, None));
        assert!(colours(&parts, 1, dark_ink).1.is_some());
        let (stroke, tint) = colours(&parts, 2, dark_ink);
        assert_eq!(stroke, dark_ink, "tint-only parts keep the ink outline");
        assert!(tint.is_some());
        let faded = Color32::from_rgba_unmultiplied(0x42, 0x42, 0x42, 100);
        assert_eq!(colours(&parts, 0, faded).0.a(), 100);
    }
}
