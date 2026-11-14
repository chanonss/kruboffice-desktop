//! Ribbon dimensions and column placement ported from KhianKrub's b2f99125 change.
//! Keep the same logical grid in KhitKrub and PhutKrub.

use egui::{Ui, vec2};

/// Two rows of small buttons, or one large button as tall as both (owner's two-size ribbon,
/// 2026-11-13, as in KhianKrub).
pub const CONTENT_H: f32 = 60.0;
pub const CONTENT_MARGIN: f32 = 8.0;
pub const BIG_MIN_W: f32 = 44.0;
/// Small buttons: icon only (the name is the tooltip), 28 × 28.
pub const SMALL_W: f32 = 28.0;
pub const SMALL_H: f32 = 28.0;
pub const SMALL_ICON: f32 = 20.0;
/// Large buttons: a 24 px icon over a label of up to two lines.
pub const BIG_ICON: f32 = 24.0;
pub const BIG_ICON_Y: f32 = 17.0;
pub const BIG_LABEL_Y: f32 = 31.0;
pub const LABEL_PITCH: f32 = 13.0;
pub const LABEL_LINE_H: f32 = 14.0;
pub const ROW_GAP: f32 = 4.0;

const _: () = assert!(2.0 * SMALL_H + ROW_GAP <= CONTENT_H);
const _: () = assert!(BIG_LABEL_Y + LABEL_PITCH + LABEL_LINE_H <= CONTENT_H);
const _: () = assert!(BIG_ICON_Y + BIG_ICON / 2.0 <= BIG_LABEL_Y);

/// Center a column inside the ribbon body, using its measured height from the
/// preceding pass. Nested rows keep their ordinary layout. A changed measurement
/// discards the pass so a stale offset is never presented as the final frame.
pub fn column<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let id = ui.next_auto_id().with("ribbon_column_height");
    let center = ui.layout().is_horizontal() && ui.available_height() >= CONTENT_H - 0.5;
    let last: Option<f32> = ui.data(|data| data.get_temp(id));
    let pad = if center { last.map_or(0.0, |height| ((CONTENT_H - height) / 2.0).max(0.0).floor()) } else { 0.0 };
    let inner = ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(1.0, 0.0);
        ui.add_space(pad);
        add(ui)
    });
    let height = inner.response.rect.height() - pad;
    if center && last.is_none_or(|previous| (previous - height).abs() > 0.5) {
        ui.data_mut(|data| data.insert_temp(id, height));
        ui.ctx().request_discard("ribbon column height");
    }
    inner.inner
}

/// Number badges take up the same horizontal space in each editor.
pub const TAB_BADGE_SPACE: f32 = 21.0;
const TAB_GAP: f32 = 2.0;
const OVERFLOW_WIDTH: f32 = 24.0;

/// Centre the visible tabs in the window (or start them at `left`), clamped between File and the action buttons.
/// A selected tab takes priority over contextual tabs and then the ordinary tabs.
pub fn tab_slots(strip: egui::Rect, left: f32, right: f32, tabs: &[(f32, u8)], centered: bool) -> (Vec<Option<egui::Rect>>, Option<egui::Rect>) {
    let available = (right - left).max(0.0);
    let mut visible = vec![true; tabs.len()];
    let mut width: f32 = tabs.iter().map(|(w, _)| w + TAB_GAP).sum();
    let overflow = width > available;
    let menu_width = if overflow { OVERFLOW_WIDTH.min(available) } else { 0.0 };
    if overflow {
        for priority in 0..=2 {
            for (fit, (w, p)) in visible.iter_mut().zip(tabs).rev() {
                if width + menu_width <= available {
                    break;
                }
                if *p == priority {
                    *fit = false;
                    width -= w + TAB_GAP;
                }
            }
        }
    }
    let occupied = width.max(0.0) + menu_width;
    // Centred by default; off (Options › General) the tabs start at the left, right after File.
    let mut x = if centered { (strip.center().x - occupied / 2.0).clamp(left, (right - occupied).max(left)) } else { left };
    let slots = tabs
        .iter()
        .zip(visible)
        .map(|((width, _), fit)| {
            fit.then(|| {
                let rect = egui::Rect::from_min_size(egui::pos2(x, strip.top()), vec2(*width, strip.height()));
                x += width + TAB_GAP;
                rect
            })
        })
        .collect();
    let menu = (menu_width > 0.0).then(|| egui::Rect::from_min_size(egui::pos2(x, strip.top()), vec2(menu_width, strip.height())));
    (slots, menu)
}

pub fn tab_label(p: &egui::Painter, rect: egui::Rect, label: &str, font: egui::FontId, color: egui::Color32, badge: Option<(usize, egui::Color32)>) {
    let mut center = rect.center();
    if let Some((number, ink)) = badge {
        let pos = egui::pos2(rect.left() + 16.0, center.y);
        p.circle_filled(pos, 8.0, egui::Color32::WHITE);
        p.circle_stroke(pos, 8.0, egui::Stroke::new(0.6, ink.gamma_multiply(0.38)));
        p.text(pos, egui::Align2::CENTER_CENTER, number, egui::FontId::proportional(9.0), ink);
        center.x += TAB_BADGE_SPACE / 2.0;
    }
    p.text(center, egui::Align2::CENTER_CENTER, label, font, color);
}

pub const RIBBON_COMMANDS: &str = "krub_ribbon_commands";

pub fn in_ribbon(ui: &Ui) -> bool {
    ui.stack().iter().any(|node| node.tags().contains(RIBBON_COMMANDS))
}

pub fn control_surface(p: &egui::Painter, rect: egui::Rect, fill: egui::Color32, border: egui::Color32, ink: egui::Color32, pressed: bool) {
    p.rect(rect, 3.0, fill, egui::Stroke::new(0.75, border), egui::StrokeKind::Inside);
    if pressed && rect.width() >= 8.0 {
        p.line_segment(
            [egui::pos2(rect.left() + 3.0, rect.top() + 1.5), egui::pos2(rect.right() - 3.0, rect.top() + 1.5)],
            egui::Stroke::new(1.0, ink.gamma_multiply(0.22)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Align, CentralPanel, Context, Layout, RawInput, Rect, Sense};

    #[test]
    fn tab_row_centers_and_keeps_the_selected_tab_in_a_narrow_window() {
        let strip = Rect::from_min_size(egui::Pos2::ZERO, vec2(800.0, 30.0));
        let items = [(70.0, 0), (90.0, 2), (110.0, 0), (80.0, 1)];
        let (slots, more) = tab_slots(strip, 50.0, 730.0, &items, true);
        assert!(more.is_none());
        let center = slots.into_iter().flatten().fold(Rect::NOTHING, Rect::union).center().x;
        assert!((center - strip.center().x).abs() <= 1.1);
        let (slots, more) = tab_slots(strip, 50.0, 260.0, &items, true);
        assert!(slots.get(1).is_some_and(Option::is_some));
        assert!(slots.get(3).is_some_and(Option::is_some));
        assert!(more.is_some_and(|rect| rect.right() <= 260.0));
        assert!(slots.into_iter().flatten().all(|rect| rect.left() >= 50.0 && rect.right() <= 260.0));
    }

    fn measure(ctx: &Context, count: usize, room: f32) -> (Rect, Vec<Rect>) {
        let mut large = Rect::NOTHING;
        let mut rows = Vec::new();
        let mut output = ctx.run_ui(RawInput::default(), |root| {
            rows.clear();
            CentralPanel::default().show(root, |ui| {
                ui.allocate_ui_with_layout(vec2(240.0, room), Layout::left_to_right(Align::Min), |ui| {
                    large = ui.allocate_exact_size(vec2(44.0, room), Sense::click()).0;
                    column(ui, |ui| {
                        for row in 0..count {
                            if count == 2 && row == 1 {
                                ui.add_space(ROW_GAP);
                            }
                            rows.push(ui.allocate_exact_size(vec2(SMALL_W, SMALL_H), Sense::click()).0);
                        }
                    });
                });
            });
        });
        output.textures_delta.clear();
        (large, rows)
    }

    #[test]
    fn two_rows_center_without_clipping() {
        // The two-size ribbon has two rows at most (owner, 2026-11-13).
        for count in [2] {
            let ctx = Context::default();
            let _ = measure(&ctx, count, CONTENT_H);
            let (large, rows) = measure(&ctx, count, CONTENT_H);
            let bounds = rows.iter().copied().fold(Rect::NOTHING, Rect::union);
            assert!((bounds.center().y - large.center().y).abs() <= 0.5);
            assert!(bounds.top() >= large.top() && bounds.bottom() <= large.bottom());
            for pair in rows.windows(2) {
                let gap = pair.get(1).map(|r| r.top()).unwrap_or_default() - pair.first().map(|r| r.bottom()).unwrap_or_default();
                assert!((gap - if count == 2 { ROW_GAP } else { 0.0 }).abs() <= 0.5);
            }
        }
    }

    #[test]
    fn a_changed_column_height_is_recentered() {
        let ctx = Context::default();
        let _ = measure(&ctx, 3, CONTENT_H);
        let _ = measure(&ctx, 2, CONTENT_H);
        let (large, rows) = measure(&ctx, 2, CONTENT_H);
        let bounds = rows.iter().copied().fold(Rect::NOTHING, Rect::union);
        assert!((bounds.center().y - large.center().y).abs() <= 0.5);
    }

    #[test]
    fn a_nested_short_row_does_not_get_ribbon_padding() {
        let ctx = Context::default();
        let _ = measure(&ctx, 1, SMALL_H);
        let (large, rows) = measure(&ctx, 1, SMALL_H);
        let row = rows.first().copied().unwrap_or(Rect::NOTHING);
        assert!((row.top() - large.top()).abs() <= 0.5);
    }
}
