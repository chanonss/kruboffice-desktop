//! Pieces every KrubOffice side pane is built from (owner, 2026-11-18, draft B): a segmented
//! control that splits a pane with many settings into sub-panels, and section headings drawn as
//! shaded bars that fold their section. Both keep their state in egui's memory, so a pane opens
//! on the sub-panel and folds the person left it with.
//!
//! The apps run different egui versions, so this uses only what they share.

use egui::{Color32, FontId, Rect, Sense, Stroke, Ui, pos2, vec2};

/// An id from `source`, for [`segmented`] and [`section`]. egui 0.37 (KhitKrub, PhutKrub)
/// deprecates `Id::new`, and 0.36 (KhianKrub) has nothing in its place, so the one call that
/// works in both lives here.
#[allow(deprecated)]
pub fn id(source: impl std::hash::Hash + std::fmt::Debug) -> egui::Id {
    egui::Id::new(source)
}

/// One slot of a [`segmented`] control: an icon id (see [`crate::paint_named`]) and its label.
pub struct Segment<'a> {
    pub icon: &'a str,
    pub label: String,
}

/// Height of a [`segmented`] control: icon, label and padding.
pub const SEGMENTED_H: f32 = 44.0;
/// Height of a [`section`] heading bar.
pub const HEADING_H: f32 = 22.0;
/// Space between rows inside a section (the grids' vertical spacing).
pub const ROW_GAP: f32 = 4.0;
/// Width of the label column, so the boxes of every section line up.
pub const LABEL_W: f32 = 92.0;

/// The accent the selected slot is drawn in: the first of the app's selection text, selection
/// fill and link colours that stands out against the pane (PhutKrub's selection text is white,
/// for its coral selection).
fn accent(ui: &Ui) -> Color32 {
    let v = ui.visuals();
    let pane = v.panel_fill;
    [v.selection.stroke.color, v.selection.bg_fill, v.hyperlink_color]
        .into_iter()
        .find(|c| contrast(*c, pane) >= 3.0)
        .unwrap_or_else(|| v.strong_text_color())
}

/// The WCAG contrast ratio of two opaque colours (1 to 21).
fn contrast(a: Color32, b: Color32) -> f32 {
    fn lum(c: Color32) -> f32 {
        let ch = |x: u8| {
            let x = f32::from(x) / 255.0;
            if x <= 0.039_28 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
    }
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `c` at `alpha` (0–255) over transparent.
fn faded(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

/// A row of equal slots in one frame; returns the selected slot's index. The choice is kept
/// under `id` (persisted), `default` until the person picks one. Every slot is the same width,
/// whatever its label, and labels stay on one line.
pub fn segmented(ui: &mut Ui, id: egui::Id, segments: &[Segment], default: usize) -> usize {
    let mut selected = ui.data_mut(|d| *d.get_persisted_mut_or(id, default));
    if segmented_at(ui, id, segments, &mut selected) {
        ui.data_mut(|d| d.insert_persisted(id, selected));
    }
    selected
}

/// [`segmented`] with the choice kept by the caller in `selected` (clamped to the slots);
/// `true` when the person picked a slot this frame.
pub fn segmented_at(ui: &mut Ui, id: egui::Id, segments: &[Segment], selected: &mut usize) -> bool {
    let n = segments.len();
    if n == 0 {
        *selected = 0;
        return false;
    }
    *selected = (*selected).min(n - 1);
    let width = ui.available_width().max(1.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, SEGMENTED_H), Sense::hover());
    let visuals = ui.visuals().clone();
    let border = visuals.widgets.noninteractive.bg_stroke.color;
    let text = visuals.text_color();
    let ink = accent(ui);
    let slot_w = rect.width() / n as f32;
    let mut picked = None;
    for (i, seg) in segments.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + slot_w * i as f32, rect.top()), vec2(slot_w, rect.height()));
        let resp = ui.interact(r, id.with(i), Sense::click());
        let on = i == *selected;
        if on {
            ui.painter().rect_filled(r.shrink(1.0), 5.0, faded(ink, 34));
        } else if resp.hovered() {
            ui.painter().rect_filled(r.shrink(1.0), 5.0, faded(text, 14));
        }
        let color = if on { ink } else { visuals.weak_text_color() };
        let icon = Rect::from_center_size(pos2(r.center().x, r.top() + 15.0), vec2(18.0, 18.0));
        crate::paint_named(ui.painter(), icon, seg.icon, color, false, false);
        // The label is a real label (what assistive technology reads for the slot), placed
        // centred under the icon and cut at the slot's edges.
        // A long label (แถวและคอลัมน์) is set a little smaller to fit, down to 9.5 pt.
        let room = r.width() - 10.0;
        let wide = ui.painter().layout_no_wrap(seg.label.clone(), FontId::proportional(12.0), text).size().x;
        let font = FontId::proportional(if wide > room && wide > 0.0 { (12.0 * room / wide).max(9.5) } else { 12.0 });
        let size = ui.painter().layout_no_wrap(seg.label.clone(), font.clone(), text).size();
        let w = size.x.min(r.width() - 4.0);
        let at = Rect::from_min_size(pos2(r.center().x - w / 2.0, r.bottom() - 6.0 - size.y), vec2(w, size.y));
        let label = ui.put(
            at,
            egui::Label::new(egui::RichText::new(seg.label.clone()).font(font).color(if on { ink } else { text })).selectable(false).truncate(),
        );
        let _ = resp.clone().labelled_by(label.id);
        if i > 0 {
            ui.painter().line_segment([pos2(r.left(), r.top() + 6.0), pos2(r.left(), r.bottom() - 6.0)], Stroke::new(1.0, border));
        }
        if resp.clicked() || label.clicked() {
            picked = Some(i);
        }
    }
    ui.painter().rect_stroke(rect, 6.0, Stroke::new(1.0, border), egui::StrokeKind::Inside);
    ui.add_space(6.0);
    match picked {
        Some(i) => {
            *selected = i;
            true
        }
        None => false,
    }
}

/// A section: `title` on a shaded bar across the pane with a chevron; a click folds or opens the
/// rows `add` draws below it. Open at first when `default_open`; the choice is kept under `id`.
pub fn section(ui: &mut Ui, id: egui::Id, title: &str, default_open: bool, add: impl FnOnce(&mut Ui)) {
    let mut open = ui.data_mut(|d| *d.get_persisted_mut_or(id, default_open));
    let width = ui.available_width().max(1.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, HEADING_H), Sense::click());
    let visuals = ui.visuals().clone();
    let text = visuals.text_color();
    let shade = if visuals.dark_mode { faded(Color32::WHITE, 16) } else { faded(Color32::BLACK, 12) };
    let shade = if resp.hovered() { faded(shade, shade.a().saturating_mul(2)) } else { shade };
    ui.painter().rect_filled(rect, 3.0, shade);
    let chevron = Rect::from_center_size(pos2(rect.left() + 11.0, rect.center().y), vec2(14.0, 14.0));
    crate::paint_named(ui.painter(), chevron, if open { "chevron-down" } else { "chevron-right" }, visuals.weak_text_color(), false, false);
    let font = FontId::proportional(12.5);
    let size = ui.painter().layout_no_wrap(title.to_string(), font.clone(), text).size();
    let w = size.x.min(rect.width() - 26.0).max(0.0);
    let at = Rect::from_min_size(pos2(rect.left() + 22.0, rect.center().y - size.y / 2.0), vec2(w, size.y));
    let label = ui.put(at, egui::Label::new(egui::RichText::new(title).font(font).color(text).strong()).selectable(false).truncate());
    let resp = resp.labelled_by(label.id);
    if resp.clicked() || label.clicked() {
        open = !open;
        ui.data_mut(|d| d.insert_persisted(id, open));
    }
    if open {
        ui.add_space(4.0);
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = ROW_GAP;
            add(ui);
        });
        ui.add_space(8.0);
    } else {
        ui.add_space(3.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(f: impl FnMut(&mut Ui)) -> egui::Context {
        let ctx = egui::Context::default();
        let mut f = f;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| f(ui));
        out.textures_delta.clear();
        ctx
    }

    #[test]
    fn segments_share_the_width_and_remember_the_choice() {
        let tabs = id("tabs");
        let segs = || {
            [
                Segment { icon: "paint", label: "Fill and Line".into() },
                Segment { icon: "sparkles", label: "Effects".into() },
                Segment { icon: "dimensions", label: "A much longer label than the others".into() },
            ]
        };
        let mut got = usize::MAX;
        let ctx = run(|ui| got = segmented(ui, tabs, &segs(), 0));
        assert_eq!(got, 0);
        ctx.data_mut(|d| d.insert_persisted(tabs, 2usize));
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| got = segmented(ui, tabs, &segs(), 0));
        out.textures_delta.clear();
        assert_eq!(got, 2, "the stored choice");
        ctx.data_mut(|d| d.insert_persisted(tabs, 9usize));
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| got = segmented(ui, tabs, &segs(), 0));
        out.textures_delta.clear();
        assert_eq!(got, 2, "an out-of-range choice is clamped");
        let mut none = usize::MAX;
        let _ = run(|ui| none = segmented(ui, id("empty"), &[], 0));
        assert_eq!(none, 0);
    }

    #[test]
    fn the_accent_stands_out_against_the_pane() {
        assert!(contrast(Color32::WHITE, Color32::from_rgb(250, 250, 250)) < 1.1);
        assert!(contrast(Color32::BLACK, Color32::WHITE) > 20.0);
        let ctx = egui::Context::default();
        let mut got = Color32::TRANSPARENT;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let v = ui.visuals_mut();
            v.panel_fill = Color32::from_rgb(250, 250, 250);
            v.selection.stroke.color = Color32::WHITE;
            v.selection.bg_fill = Color32::from_rgb(0xC0, 0x4A, 0x2E);
            got = accent(ui);
        });
        out.textures_delta.clear();
        assert_eq!(got, Color32::from_rgb(0xC0, 0x4A, 0x2E), "white selection text is passed over");
    }

    #[test]
    fn a_folded_section_draws_nothing_below_its_heading() {
        let sec = id("sec");
        let mut drawn = false;
        let ctx = run(|ui| section(ui, sec, "Line", true, |_| drawn = true));
        assert!(drawn, "open at first");
        ctx.data_mut(|d| d.insert_persisted(sec, false));
        drawn = false;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| section(ui, sec, "Line", true, |_| drawn = true));
        out.textures_delta.clear();
        assert!(!drawn, "folded");
    }
}
