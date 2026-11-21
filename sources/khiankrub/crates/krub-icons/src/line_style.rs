//! Line styles shown as lines, the same in every KrubOffice app (owner, 2026-11-19: "show the
//! real line, it says more than words"). [`paint`] draws a style as a short line; [`combo`] is a
//! drop-down whose box and rows show the lines; [`swatch`] is one selectable button with a line.
//! The names stay: they are what assistive technology reads and the tooltips show. A choice with
//! no line (None) shows its name.
//!
//! The apps run different egui versions, so this uses only what they share: the rows are
//! selectable buttons whose (invisible) text is the name, with the line painted over them.

use egui::{Color32, Painter, Response, RichText, Stroke, Ui, pos2, vec2};

/// How a line style looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// A plain line.
    Solid,
    /// A very thin line (a spreadsheet's hair border).
    Hair,
    /// A plain line, a little heavier (a spreadsheet's medium border).
    Medium,
    /// A heavy line.
    Thick,
    Dotted,
    /// Square dots (PowerPoint's Square Dot).
    SquareDot,
    Dashed,
    LongDash,
    DashDot,
    LongDashDot,
    DashDotDot,
    Double,
    Triple,
    /// Thick above thin.
    ThickThin,
    /// Thin above thick.
    ThinThick,
    /// A thin line between two thick ones.
    ThickBetweenThin,
    Wave,
    /// An underline under the words only: short pieces with gaps.
    Words,
}

/// Draws `look` as a line from `x0` to `x1` at height `y` in `ink`.
pub fn paint(p: &Painter, x0: f32, x1: f32, y: f32, look: Look, ink: Color32) {
    let seg = |a: f32, b: f32, w: f32, dy: f32| {
        if x0 + a < x1 {
            p.line_segment([pos2(x0 + a, y + dy), pos2((x0 + b).min(x1), y + dy)], Stroke::new(w, ink));
        }
    };
    let full = |w: f32, dy: f32| seg(0.0, x1 - x0, w, dy);
    // A repeating pattern of (length, drawn) pieces along the line.
    let pattern = |pieces: &[(f32, bool)], w: f32| {
        let mut x = 0.0;
        let step: f32 = pieces.iter().map(|(l, _)| l).sum();
        if step <= 0.0 {
            return;
        }
        while x0 + x < x1 {
            for (len, drawn) in pieces {
                if *drawn {
                    seg(x, x + len, w, 0.0);
                }
                x += len;
            }
        }
    };
    match look {
        Look::Solid => full(1.3, 0.0),
        Look::Hair => pattern(&[(1.0, true), (1.0, false)], 0.8),
        Look::Medium => full(2.0, 0.0),
        Look::Thick => full(3.0, 0.0),
        Look::Dotted => {
            let mut x = 1.0;
            while x0 + x < x1 {
                p.circle_filled(pos2(x0 + x, y), 0.9, ink);
                x += 3.6;
            }
        }
        Look::SquareDot => pattern(&[(1.8, true), (1.8, false)], 1.8),
        Look::Dashed => pattern(&[(6.0, true), (3.5, false)], 1.3),
        Look::LongDash => pattern(&[(12.0, true), (4.0, false)], 1.3),
        Look::DashDot => pattern(&[(6.0, true), (2.5, false), (1.6, true), (2.5, false)], 1.3),
        Look::LongDashDot => pattern(&[(12.0, true), (3.0, false), (1.6, true), (3.0, false)], 1.3),
        Look::DashDotDot => pattern(&[(6.0, true), (2.5, false), (1.6, true), (2.5, false), (1.6, true), (2.5, false)], 1.3),
        Look::Double => {
            full(1.1, -1.8);
            full(1.1, 1.8);
        }
        Look::Triple => {
            full(1.0, -3.0);
            full(1.0, 0.0);
            full(1.0, 3.0);
        }
        Look::ThickThin => {
            full(2.6, -1.8);
            full(1.0, 2.0);
        }
        Look::ThinThick => {
            full(1.0, -2.0);
            full(2.6, 1.8);
        }
        Look::ThickBetweenThin => {
            full(1.0, -3.4);
            full(2.6, 0.0);
            full(1.0, 3.4);
        }
        Look::Wave => {
            let mut pts = Vec::new();
            let mut x = 0.0;
            while x0 + x <= x1 {
                pts.push(pos2(x0 + x, y + (x * std::f32::consts::TAU / 6.0).sin() * 1.8));
                x += 0.8;
            }
            p.add(egui::Shape::line(pts, Stroke::new(1.2, ink)));
        }
        Look::Words => pattern(&[(10.0, true), (4.0, false), (7.0, true), (4.0, false)], 1.3),
    }
}

/// One choice: its value, how it looks (`None`: no line, its name shows instead) and its name,
/// already translated.
pub struct Item<'a, T> {
    pub value: T,
    pub look: Option<Look>,
    pub name: &'a str,
}

impl<'a, T> Item<'a, T> {
    pub fn new(value: T, look: Option<Look>, name: &'a str) -> Self {
        Item { value, look, name }
    }
}

/// The line's ink on a button: the selection's text colour when selected (it may be light on a
/// dark selection), the text colour otherwise, faded when disabled.
fn ink(ui: &Ui, selected: bool) -> Color32 {
    let v = ui.visuals();
    let c = if selected { v.selection.stroke.color } else { v.text_color() };
    if ui.is_enabled() { c } else { c.gamma_multiply(0.4) }
}

/// A selectable button `size` across showing `look` (or the name, at least `size`, when it has
/// none), named
/// `name` for assistive technology and in its tooltip.
pub fn swatch(ui: &mut Ui, size: egui::Vec2, look: Option<Look>, name: &str, selected: bool) -> Response {
    let resp = match look {
        // The line takes the text's place: the text is there but not seen.
        Some(_) => ui.add_sized(size, egui::Button::selectable(selected, RichText::new(name).color(Color32::TRANSPARENT)).truncate()),
        // No line: the name, at least `size` (wider when it needs to be).
        None => ui.add(egui::Button::selectable(selected, name).min_size(size)),
    };
    if let Some(look) = look {
        let r = resp.rect.shrink2(vec2(7.0, 0.0));
        paint(ui.painter(), r.left(), r.right(), r.center().y, look, ink(ui, selected));
    }
    resp.on_hover_text(name)
}

/// A drop-down `width` across over `items`: the box shows the current choice as a line, the list
/// one line per row. `current` is the chosen value, `None` when none matches (the box is empty).
/// Returns the box's response (to label it) and the value picked this frame.
pub fn combo<T: PartialEq + Copy>(ui: &mut Ui, id: egui::Id, width: f32, current: Option<T>, items: &[Item<T>]) -> (Response, Option<T>) {
    let now = items.iter().find(|i| Some(i.value) == current);
    let text = match now {
        Some(Item { look: Some(_), name, .. }) => RichText::new(*name).color(Color32::TRANSPARENT),
        Some(Item { name, .. }) => RichText::new(*name),
        None => RichText::new(""),
    };
    let mut picked = None;
    // A fixed list that never changes while open, so egui's combo box is fine here (the apps'
    // own wrappers can't be reached from this shared crate).
    #[allow(clippy::disallowed_types)]
    let combo = egui::ComboBox::from_id_salt(id).width(width).selected_text(text).show_ui(ui, |ui| {
        for item in items {
            let on = Some(item.value) == current;
            if swatch(ui, vec2(width, 22.0), item.look, item.name, on).clicked() {
                picked = Some(item.value);
            }
        }
    });
    if let Some(Item { look: Some(look), .. }) = now {
        // Clear of the box's arrow on the right.
        let r = combo.response.rect;
        paint(ui.painter(), r.left() + 8.0, r.right() - 26.0, r.center().y, *look, ink(ui, false));
    }
    let resp = match now {
        Some(i) => combo.response.on_hover_text(i.name),
        None => combo.response,
    };
    (resp, picked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_look_draws_inside_its_span() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let p = ui.painter();
            for look in [
                Look::Solid,
                Look::Hair,
                Look::Medium,
                Look::Thick,
                Look::Dotted,
                Look::SquareDot,
                Look::Dashed,
                Look::LongDash,
                Look::DashDot,
                Look::LongDashDot,
                Look::DashDotDot,
                Look::Double,
                Look::Triple,
                Look::ThickThin,
                Look::ThinThick,
                Look::ThickBetweenThin,
                Look::Wave,
                Look::Words,
            ] {
                paint(p, 10.0, 90.0, 20.0, look, Color32::BLACK);
                // A span with no room draws nothing and doesn't loop.
                paint(p, 50.0, 50.0, 20.0, look, Color32::BLACK);
            }
        });
        out.textures_delta.clear();
        for clipped in &out.shapes {
            let r = clipped.shape.visual_bounding_rect();
            assert!(r.left() >= 8.0 && r.right() <= 92.0, "{r:?} outside 10–90");
        }
    }
}
