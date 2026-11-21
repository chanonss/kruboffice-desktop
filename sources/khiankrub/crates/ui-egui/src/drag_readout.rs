//! The distances shown beside a picture, shape or text box while it is dragged (a contributed
//! patch, see FORK-CHANGES.md): how far its left and top edges are from the paper's (or, while resizing,
//! its width and height) in the options' unit, so a drop can land on 1.5 cm by eye.

use egui::{Color32, Painter, Pos2, Rect, vec2};
use wordcraft_geom::Unit;

/// The text of the readout: `Left: 1.5 cm   Top: 1.5 cm` (a move) or `W: …   H: …` (a resize).
pub fn readout_text(unit: Unit, decimal: char, resizing: bool, r: wordcraft_geom::Rect) -> String {
    let n = |pt: f32| unit.format_with(pt, decimal);
    if resizing {
        format!("{} {}   {} {}", tl!("W:"), n(r.w), tl!("H:"), n(r.h))
    } else {
        format!("{} {}   {} {}", tl!("Left:"), n(r.x), tl!("Top:"), n(r.y))
    }
}

/// Draw the readout for an object being dragged: below its frame, above it where the page ends,
/// kept inside the page's width. `frame` is the dragged frame on screen, `page` the page on screen.
pub fn paint_readout(painter: &Painter, text: String, frame: Rect, page: Rect) {
    let galley = painter.layout_no_wrap(text, egui::FontId::proportional(12.0), Color32::WHITE);
    let (pad, gap) = (5.0, 6.0);
    let size = galley.size() + vec2(pad * 2.0, pad);
    let mut min = Pos2::new(frame.min.x, frame.max.y + gap);
    if min.y + size.y > page.max.y {
        min.y = (frame.min.y - gap - size.y).max(page.min.y);
    }
    min.x = min.x.min(page.max.x - size.x).max(page.min.x);
    let rect = Rect::from_min_size(min, size);
    painter.rect_filled(rect, egui::CornerRadius::same(4), Color32::from_rgba_unmultiplied(28, 28, 32, 232));
    painter.galley(rect.min + vec2(pad, pad / 2.0), galley, Color32::WHITE);
}

/// The two lines the right-click menu of a picture, shape or text box shows: its distances from the
/// paper's left and top edges, then from its right and bottom edges, in the options' unit.
pub fn menu_lines(app: &mut crate::WordApp) -> Option<[String; 2]> {
    let (pos, _) = wordcraft_engine::cmd::objects::selected(&app.session)?;
    let hint = app.session.page_hint;
    let layout = app.session.layout();
    let o = layout.object(&pos, hint)?;
    let pg = layout.pages.get(o.page)?;
    let (unit, decimal) = (crate::options::unit(), crate::options::decimal());
    Some(menu_text(unit, decimal, o.rect, (pg.w - pg.markup).max(0.0), pg.h))
}

fn menu_text(unit: Unit, decimal: char, r: wordcraft_geom::Rect, page_w: f32, page_h: f32) -> [String; 2] {
    let n = |pt: f32| unit.format_with(pt, decimal);
    [readout_text(unit, decimal, false, r), format!("{} {}   {} {}", tl!("Right:"), n(page_w - r.x - r.w), tl!("Bottom:"), n(page_h - r.y - r.h))]
}

/// How long (seconds) the guides and distances stay after an arrow-key nudge.
const NUDGE_SHOWN: f64 = 1.5;

fn nudge_id() -> egui::Id {
    egui::Id::new("drag_readout_nudge")
}

/// An arrow key just moved the selected object: show its distances for a moment, as a drag does.
pub fn mark_nudge(ctx: &egui::Context) {
    let now = ctx.input(|i| i.time);
    ctx.data_mut(|d| d.insert_temp(nudge_id(), now));
    ctx.request_repaint_after(std::time::Duration::from_secs_f64(NUDGE_SHOWN + 0.05));
}

/// Whether [`mark_nudge`] was called within the last moment.
pub fn nudged_recently(ctx: &egui::Context) -> bool {
    let now = ctx.input(|i| i.time);
    ctx.data(|d| d.get_temp::<f64>(nudge_id())).is_some_and(|t| now - t < NUDGE_SHOWN)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> wordcraft_geom::Rect {
        wordcraft_geom::Rect::new(x, y, w, h)
    }

    /// 1.5 cm from the paper's edges reads as `1.5 cm`, in the unit the options chose.
    #[test]
    fn readout_shows_the_distances_in_the_options_unit() {
        let cm = 72.0 / 2.54;
        let s = readout_text(Unit::Centimeters, '.', false, rect(1.5 * cm, 3.0 * cm, 100.0, 50.0));
        assert!(s.contains("1.5 cm") && s.contains("3 cm"), "{s}");
        let s = readout_text(Unit::Millimeters, ',', true, rect(0.0, 0.0, 10.0 * cm, 2.0 * cm));
        assert!(s.contains("100 mm") && s.contains("20 mm"), "{s}");
    }

    /// An arrow-key nudge shows the distances for a moment, then they go.
    #[test]
    fn a_nudge_shows_the_distances_for_a_moment() {
        let ctx = egui::Context::default();
        assert!(!nudged_recently(&ctx));
        // One frame at `time`; its texture changes are cleared (egui asserts none are dropped).
        let frame = |time: f64, f: &dyn Fn(&egui::Context)| {
            let mut out = ctx.run_ui(egui::RawInput { time: Some(time), ..Default::default() }, |ui| f(ui.ctx()));
            out.textures_delta.clear();
        };
        frame(10.0, &mark_nudge);
        frame(10.5, &|ctx| assert!(nudged_recently(ctx)));
        frame(12.0, &|ctx| assert!(!nudged_recently(ctx)));
    }

    /// The menu's second line counts from the right and bottom edges of the paper.
    #[test]
    fn the_menu_lines_give_all_four_distances() {
        let cm = 72.0 / 2.54;
        let [a, b] = menu_text(Unit::Centimeters, '.', rect(1.5 * cm, 2.0 * cm, 4.0 * cm, 3.0 * cm), 21.0 * cm, 29.7 * cm);
        assert!(a.contains("1.5 cm") && a.contains("2 cm"), "{a}");
        assert!(b.contains("15.5 cm") && b.contains("24.7 cm"), "{b}");
    }
}
