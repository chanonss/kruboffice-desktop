//! The Quick Access Toolbar at the top left of every KrubOffice app, laid out the same way:
//!
//! `[AutoSave]` · Save · Undo ▾ · Redo · `[the app's own command]` · ⌄
//!
//! AutoSave shows only in an app that has it; the ▾ beside Undo opens the undo history; ⌄ at the
//! end opens the Customize menu. The bar stays on screen while File is open. Each app draws its
//! own buttons with these sizes and the drawings here.

use egui::{Color32, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, pos2, vec2};

/// A command button.
pub const BUTTON: egui::Vec2 = vec2(26.0, 24.0);
/// The undo-history arrow beside Undo.
pub const ARROW: egui::Vec2 = vec2(12.0, 24.0);
/// Space between buttons.
pub const GAP: f32 = 2.0;
/// Space after the AutoSave switch, before the first button.
pub const AFTER_AUTOSAVE: f32 = 8.0;

/// The commands every app's toolbar starts with, in order (the app adds its own after them).
pub const DEFAULTS: [&str; 3] = ["file.save", "edit.undo", "edit.redo"];

/// A small downward chevron centred in `r` (the undo-history arrow and the Customize button).
pub fn chevron(p: &Painter, r: Rect, size: f32, ink: Color32) {
    let c = r.center();
    let (w, h) = (size * 0.5, size * 0.28);
    let pts: [Pos2; 3] = [pos2(c.x - w, c.y - h * 0.5), pos2(c.x, c.y + h * 0.5), pos2(c.x + w, c.y - h * 0.5)];
    p.add(Shape::line(pts.to_vec(), Stroke::new(1.4, ink)));
}

/// The Customize button at the end of the bar: ⌄, with the hover fill. Returns its response
/// (the app opens its menu from it).
pub fn customize_button(ui: &mut Ui, id: egui::Id, ink: Color32, hover: Color32) -> Response {
    let (r, _) = ui.allocate_exact_size(BUTTON, Sense::hover());
    let resp = ui.interact(r, id, Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, 4.0, hover);
    }
    chevron(ui.painter(), r, 11.0, ink);
    resp
}

/// The undo-history arrow beside Undo: ▾ with the hover fill.
pub fn history_arrow(ui: &mut Ui, id: egui::Id, enabled: bool, ink: Color32, disabled: Color32, hover: Color32) -> Response {
    let (r, _) = ui.allocate_exact_size(ARROW, Sense::hover());
    let resp = ui.interact(r, id, if enabled { Sense::click() } else { Sense::hover() });
    if resp.hovered() && enabled {
        ui.painter().rect_filled(r, 4.0, hover);
    }
    chevron(ui.painter(), r, 8.0, if enabled { ink } else { disabled });
    resp
}
