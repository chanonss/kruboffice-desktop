//! Shared Quiet Glass UI icons; stored workbook artwork retains its original rendering.
pub use krub_icons::sheet::{FnGlyph, Icon, LIBRARY, fallback_for_symbol, from_name, paint, paint_accent, paint_flat};
mod content;

/// Render workbook artwork, preserving the existing stored-icon appearance.
pub fn paint_on_sheet(p: &egui::Painter, rect: egui::Rect, icon: Icon, c: egui::Color32) {
    content::paint_on_sheet(p, rect, icon, c);
}
