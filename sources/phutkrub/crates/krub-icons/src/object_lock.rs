//! How a locked object looks and answers, the same in the three KrubOffice apps (owner,
//! 2026-11-23, draft "Lock an object"): when selected, a grey frame with small corner dots and no
//! handles, and a padlock badge just outside its top-right corner; while hovered and not selected,
//! the badge alone, faint. Trying to move, resize, delete or edit it is refused with
//! [`refuse`]: the badge turns gold for a moment, the pointer shows "not allowed" and a short note
//! appears next to the badge ([`show_note`]).
//!
//! Each app draws its own objects and translates its own note; this module only paints, so it
//! uses nothing the apps' egui versions don't share.

use egui::{Color32, Context, CursorIcon, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, pos2, vec2};

/// The frame, dots and badge outline of a locked object.
pub const GREY: Color32 = Color32::from_rgb(0x8A, 0x8F, 0x98);
/// The badge while a refusal is being shown.
pub const GOLD: Color32 = Color32::from_rgb(0xC9, 0x9A, 0x1E);
/// How long a refusal shows (seconds).
pub const NOTE_SECS: f64 = 1.8;
/// The badge's size (points).
pub const BADGE: egui::Vec2 = vec2(20.0, 18.0);

/// The badge's rectangle for a frame whose corners are `corners` (top-left, top-right,
/// bottom-right, bottom-left on screen): just outside the top-right corner, above the frame.
pub fn badge_rect(corners: [Pos2; 4]) -> Rect {
    let tr = corners[1];
    Rect::from_min_size(pos2(tr.x + 3.0, tr.y - BADGE.y - 3.0), BADGE)
}

/// A locked object's selection: grey frame, corner dots, badge (gold while a refusal shows).
pub fn paint_selected(ctx: &Context, p: &Painter, corners: [Pos2; 4]) {
    if !corners.iter().all(|c| c.is_finite()) {
        return;
    }
    let stroke = Stroke::new(1.5, GREY);
    for i in 0..4 {
        p.line_segment([corners[i], corners[(i + 1) % 4]], stroke);
    }
    for c in corners {
        p.circle_filled(c, 2.5, GREY);
    }
    paint_badge(p, corners, flashing(ctx), false);
}

/// The faint badge on a locked object under the pointer that isn't selected.
pub fn paint_hover(p: &Painter, corners: [Pos2; 4]) {
    if corners.iter().all(|c| c.is_finite()) {
        paint_badge(p, corners, false, true);
    }
}

fn paint_badge(p: &Painter, corners: [Pos2; 4], hot: bool, faint: bool) {
    let r = badge_rect(corners);
    let alpha = |c: Color32| if faint { c.gamma_multiply(0.55) } else { c };
    let (fill, line, ink) = if hot { (GOLD, GOLD, Color32::WHITE) } else { (Color32::WHITE, GREY, Color32::from_rgb(0x5B, 0x60, 0x68)) };
    p.rect(r, 5.0, alpha(fill), Stroke::new(1.0, alpha(line)), StrokeKind::Inside);
    padlock(p, r.shrink2(vec2(4.0, 2.5)), alpha(ink));
}

/// A closed padlock in `r`, drawn with lines (Tabler's `lock` shape, simplified for 12 points).
fn padlock(p: &Painter, r: Rect, ink: Color32) {
    let s = Stroke::new(1.4, ink);
    let body = Rect::from_min_max(pos2(r.left(), r.top() + r.height() * 0.45), r.max);
    p.rect(body, 1.5, Color32::TRANSPARENT, s, StrokeKind::Inside);
    let (cx, rad) = (body.center().x, body.width() * 0.3);
    let top = r.top() + rad;
    let shackle: Vec<Pos2> = (0..=12)
        .map(|i| {
            let a = std::f32::consts::PI * (1.0 + i as f32 / 12.0);
            pos2(cx + rad * a.cos(), top + rad * a.sin())
        })
        .collect();
    p.line_segment([pos2(cx - rad, top), pos2(cx - rad, body.top())], s);
    p.line_segment([pos2(cx + rad, top), pos2(cx + rad, body.top())], s);
    p.add(egui::Shape::line(shackle, s));
    p.circle_filled(body.center(), 1.1, ink);
}

/// The gold line at the top of an object's pane while it is locked: a padlock, `text` (already
/// translated) and an `unlock` button. Returns true when the button was clicked.
pub fn pane_banner(ui: &mut egui::Ui, text: &str, unlock: &str) -> bool {
    let mut clicked = false;
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(0xC9, 0x9A, 0x1E, 40))
        .stroke(Stroke::new(1.0, GOLD))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::hover());
                crate::paint_named(ui.painter(), r, "lock", ui.visuals().text_color(), true, false);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    clicked = ui.add(egui::Button::new(unlock)).clicked();
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(egui::Label::new(text).wrap());
                    });
                });
            });
        });
    ui.add_space(4.0);
    clicked
}

/// The Lock tile of an object's pane (Arrange): a padlock, open or closed, and `label`, `width`
/// wide; drawn "on" (gold) while `locked`. `tip` says what it does. Returns its response.
pub fn lock_tile(ui: &mut egui::Ui, locked: bool, label: &str, tip: &str, width: f32) -> egui::Response {
    let h = 28.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(width.max(h), h), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let v = ui.visuals();
        let w = v.widgets.style(&resp);
        let (fill, line) =
            if locked { (Color32::from_rgba_unmultiplied(0xC9, 0x9A, 0x1E, 48), Stroke::new(1.0, GOLD)) } else { (w.weak_bg_fill, w.bg_stroke) };
        let p = ui.painter();
        p.rect(rect, 6.0, fill, line, StrokeKind::Inside);
        let icon = Rect::from_center_size(pos2(rect.left() + 8.0 + 8.0, rect.center().y), vec2(16.0, 16.0));
        let ink = if ui.is_enabled() { v.text_color() } else { v.weak_text_color() };
        crate::paint_named(p, icon, if locked { "lock" } else { "unlock" }, ink, true, !ui.is_enabled());
        let font = egui::TextStyle::Button.resolve(ui.style());
        let text = if locked { egui::RichText::new(label).strong() } else { egui::RichText::new(label) };
        let galley = egui::WidgetText::from(text).into_galley(ui, Some(egui::TextWrapMode::Truncate), rect.width() - 36.0, font);
        p.galley(pos2(icon.right() + 6.0, rect.center().y - galley.size().y / 2.0), galley, ink);
    }
    resp.on_hover_text(tip)
}

#[derive(Clone, Copy)]
struct Refusal {
    at: Pos2,
    since: f64,
}

fn key() -> egui::Id {
    crate::pane_ui::id("krub_object_lock_refusal")
}

/// Show that a locked object refused a change: the badge goes gold, and the note appears by `at`
/// (normally the badge's corner) for [`NOTE_SECS`].
pub fn refuse(ctx: &Context, at: Pos2) {
    let since = ctx.input(|i| i.time);
    ctx.data_mut(|d| d.insert_temp(key(), Refusal { at, since }));
    ctx.request_repaint();
}

/// The words every app's engine puts in the error of a change a locked object refused, so the
/// interface can tell such an error from others ([`is_lock_error`]).
pub const LOCKED_ERROR: &str = "the object is locked";

/// Whether `message` is a refusal by a locked object.
pub fn is_lock_error(message: &str) -> bool {
    message.contains(LOCKED_ERROR)
}

static ASKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Ask for a refusal to be shown on the next frame, where the locked object is drawn (code that
/// sees a refused command doesn't know where the object is on screen).
pub fn ask() {
    ASKED.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Whether a refusal was asked for since the last call; clears the request.
pub fn take_ask() -> bool {
    ASKED.swap(false, std::sync::atomic::Ordering::Relaxed)
}

/// Whether a refusal is showing now.
pub fn flashing(ctx: &Context) -> bool {
    let now = ctx.input(|i| i.time);
    ctx.data(|d| d.get_temp::<Refusal>(key())).is_some_and(|r| now - r.since < NOTE_SECS)
}

/// Draw the refusal's note (`text`, already translated) over everything, set the "not allowed"
/// pointer, and keep repainting until it's gone. Call once a frame.
pub fn show_note(ctx: &Context, text: &str) {
    let now = ctx.input(|i| i.time);
    let Some(r) = ctx.data(|d| d.get_temp::<Refusal>(key())) else { return };
    if now - r.since >= NOTE_SECS || !r.at.is_finite() {
        return;
    }
    ctx.set_cursor_icon(CursorIcon::NotAllowed);
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, key()));
    let galley = p.layout_no_wrap(text.to_owned(), FontId::proportional(12.5), Color32::WHITE);
    let pad = vec2(8.0, 4.0);
    let size = galley.size() + 2.0 * pad;
    let screen = ctx.content_rect();
    let mut min = pos2(r.at.x - size.x, r.at.y - size.y - 6.0);
    min.x = min.x.clamp(screen.left() + 4.0, (screen.right() - size.x - 4.0).max(screen.left() + 4.0));
    if min.y < screen.top() + 4.0 {
        min.y = r.at.y + BADGE.y + 8.0;
    }
    let rect = Rect::from_min_size(min, size);
    p.rect_filled(rect, 5.0, Color32::from_rgb(0x2B, 0x2E, 0x33));
    p.galley(rect.min + pad, galley, Color32::WHITE);
    ctx.request_repaint();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_shows_then_goes() {
        let ctx = Context::default();
        let mut input = egui::RawInput { time: Some(10.0), ..Default::default() };
        ctx.begin_pass(input.clone());
        assert!(!flashing(&ctx));
        refuse(&ctx, pos2(100.0, 100.0));
        assert!(flashing(&ctx));
        show_note(&ctx, "ล็อกอยู่");
        ctx.end_pass().textures_delta.clear();
        input.time = Some(10.0 + NOTE_SECS + 0.1);
        ctx.begin_pass(input);
        assert!(!flashing(&ctx));
        ctx.end_pass().textures_delta.clear();
    }

    #[test]
    fn lock_errors_are_recognised_and_asks_taken_once() {
        assert!(is_lock_error("shape.move: the object is locked"));
        assert!(!is_lock_error("no shapes selected"));
        ask();
        assert!(take_ask());
        assert!(!take_ask());
    }

    #[test]
    fn painting_tolerates_bad_corners() {
        let ctx = Context::default();
        ctx.begin_pass(egui::RawInput::default());
        let p = ctx.layer_painter(egui::LayerId::background());
        let good = [pos2(0.0, 0.0), pos2(50.0, 0.0), pos2(50.0, 30.0), pos2(0.0, 30.0)];
        paint_selected(&ctx, &p, good);
        paint_hover(&p, good);
        let bad = [pos2(f32::NAN, 0.0), good[1], good[2], good[3]];
        paint_selected(&ctx, &p, bad);
        paint_hover(&p, bad);
        assert!(badge_rect(good).min.x > 50.0 && badge_rect(good).max.y < 0.0);
        ctx.end_pass().textures_delta.clear();
    }
}
