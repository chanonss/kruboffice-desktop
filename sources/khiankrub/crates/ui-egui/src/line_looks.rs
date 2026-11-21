//! KhianKrub's line-style choices shown as lines (owner, 2026-11-19), through the shared
//! `krub_icons::line_style` picker: the OOXML style names and the document model's
//! `BorderStyle` mapped to how they look.

use krub_icons::line_style::{Item, Look, combo};
use wordcraft_doc::props::BorderStyle;

/// How an OOXML border or underline style (`single`, `dash`, `dotDash`…) looks; `None` for one
/// with no line (`none`), which shows its name.
pub(crate) fn word_look(style: &str) -> Option<Look> {
    Some(match style {
        "single" => Look::Solid,
        "double" => Look::Double,
        "triple" => Look::Triple,
        "thick" => Look::Thick,
        "dotted" => Look::Dotted,
        "dash" | "dashed" => Look::Dashed,
        "dotDash" => Look::DashDot,
        "dotDotDash" => Look::DashDotDot,
        "wave" => Look::Wave,
        "words" => Look::Words,
        _ => return None,
    })
}

/// How a border style of the document model looks.
pub(crate) fn border_look(style: BorderStyle) -> Option<Look> {
    Some(match style {
        BorderStyle::None => return None,
        BorderStyle::Single => Look::Solid,
        BorderStyle::Double => Look::Double,
        BorderStyle::Dotted => Look::Dotted,
        BorderStyle::Dashed => Look::Dashed,
        BorderStyle::Thick => Look::Thick,
        BorderStyle::Triple => Look::Triple,
        BorderStyle::DotDash => Look::DashDot,
        BorderStyle::Wave => Look::Wave,
        BorderStyle::ThickThinSmallGap | BorderStyle::ThickThinMediumGap => Look::ThickThin,
        BorderStyle::ThinThickSmallGap | BorderStyle::ThinThickMediumGap | BorderStyle::ThinThickLargeGap => Look::ThinThick,
        _ => Look::ThickBetweenThin,
    })
}

/// A drop-down over OOXML style names: `items` are (name, English label). Sets `current` and
/// returns `true` when one is picked.
pub(crate) fn names(ui: &mut egui::Ui, id: egui::Id, width: f32, current: &mut String, items: &[(&str, &str)]) -> bool {
    let list: Vec<Item<&str>> = items.iter().map(|(k, l)| Item::new(*k, word_look(k), tl!(*l))).collect();
    let now = items.iter().map(|(k, _)| *k).find(|k| *k == current.as_str());
    match combo(ui, id, width, now, &list).1 {
        Some(k) => {
            *current = k.to_string();
            true
        }
        None => false,
    }
}

/// A drop-down over border styles: `items` are (style, English label). Sets `current` and
/// returns `true` when one is picked.
pub(crate) fn borders(ui: &mut egui::Ui, id: egui::Id, width: f32, current: &mut BorderStyle, items: &[(BorderStyle, &str)]) -> bool {
    let list: Vec<Item<BorderStyle>> = items.iter().map(|(s, l)| Item::new(*s, border_look(*s), tl!(*l))).collect();
    match combo(ui, id, width, Some(*current), &list).1 {
        Some(s) => {
            *current = s;
            true
        }
        None => false,
    }
}
