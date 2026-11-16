//! Pinned fonts, the same in every KrubOffice app.
//!
//! A pin at the right end of a font's row in a font list pins it; pinned fonts head every font
//! list of that app under **Pinned Fonts**, in the order they were pinned, and stay in the
//! alphabetical list below (**All Fonts**) with a filled pin, so either row unpins them. The app
//! keeps the names in its saved settings (`ui.json` on the desktop, `localStorage` on the web).
//!
//! The lists read the pins through the egui context: each frame the app calls [`sync`] with its
//! saved list, which applies the pins and unpins clicked since the last frame and hands the list
//! to every font list drawn in that frame ([`pinned`]). A click only queues the change
//! ([`toggle_later`]), so a list never needs the app's state to be pinned from.
//!
//! The English names here ([`PINNED_FONTS`], [`ALL_FONTS`], [`PIN`], [`UNPIN`]) are the keys the
//! apps translate; [`thai`] has their Thai.

use std::sync::Arc;

use egui::{Color32, Context, Id, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, pos2, vec2};

/// The most fonts that stay pinned: pinning one more drops the oldest pin.
pub const MAX_PINNED: usize = 30;
/// Longer names are not pinned (no font family name is this long; a damaged settings file may be).
pub const MAX_NAME_CHARS: usize = 200;

/// The heading over the pinned fonts.
pub const PINNED_FONTS: &str = "Pinned Fonts";
/// The heading over the whole list, under the pinned fonts.
pub const ALL_FONTS: &str = "All Fonts";
/// The pin's hover note on a font that isn't pinned.
pub const PIN: &str = "Pin to Top";
/// The pin's hover note on a pinned font.
pub const UNPIN: &str = "Unpin";

/// The Thai of the English names above (written from their meaning, in plain Thai).
pub fn thai(english: &str) -> Option<&'static str> {
    Some(match english {
        PINNED_FONTS => "ฟอนต์ที่ปักหมุด",
        ALL_FONTS => "ฟอนต์ทั้งหมด",
        PIN => "ปักหมุดไว้ด้านบน",
        UNPIN => "เลิกปักหมุด",
        _ => return None,
    })
}

/// Whether `name` is pinned (font names compare without regard to case).
pub fn is_pinned(pins: &[String], name: &str) -> bool {
    pins.iter().any(|p| p.eq_ignore_ascii_case(name))
}

/// Pins `name` (at the end of the pinned list) or unpins it. Blank and over-long names are left
/// alone. Returns whether the list changed.
pub fn toggle(pins: &mut Vec<String>, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return false;
    }
    if let Some(i) = pins.iter().position(|p| p.eq_ignore_ascii_case(name)) {
        pins.remove(i);
    } else {
        pins.push(name.to_string());
        if pins.len() > MAX_PINNED {
            let extra = pins.len() - MAX_PINNED;
            pins.drain(..extra);
        }
    }
    true
}

/// A saved list made usable: names trimmed, blank, over-long and repeated ones dropped, at most
/// [`MAX_PINNED`] (the newest kept).
pub fn sanitized(pins: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for p in pins {
        let p = p.trim();
        if !p.is_empty() && p.chars().count() <= MAX_NAME_CHARS && !is_pinned(&out, p) {
            out.push(p.to_string());
        }
    }
    if out.len() > MAX_PINNED {
        let extra = out.len() - MAX_PINNED;
        out.drain(..extra);
    }
    out
}

/// One row of a font list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    /// A section heading: [`PINNED_FONTS`] or [`ALL_FONTS`] (to translate).
    Heading(&'static str),
    /// The font `items[index]`, pinned or not.
    Font { index: usize, pinned: bool },
}

/// The rows of a font list of `items`: with any of `pins` among them, a Pinned Fonts heading and
/// those fonts in pin order, then an All Fonts heading and every item; otherwise just the items.
pub fn rows(items: &[String], pins: &[String]) -> Vec<Row> {
    let pinned: Vec<usize> = pins.iter().filter_map(|p| items.iter().position(|it| it.eq_ignore_ascii_case(p))).collect();
    let any = !pinned.is_empty();
    let all = items.iter().enumerate().map(move |(index, it)| Row::Font { index, pinned: any && is_pinned(pins, it) });
    if !any {
        return all.collect();
    }
    let mut out = Vec::with_capacity(items.len() + pinned.len() + 2);
    out.push(Row::Heading(PINNED_FONTS));
    out.extend(pinned.into_iter().map(|index| Row::Font { index, pinned: true }));
    out.push(Row::Heading(ALL_FONTS));
    out.extend(all);
    out
}

fn pins_id() -> Id {
    Id::NULL.with("krub_icons::fonts::pinned")
}

fn queue_id() -> Id {
    Id::NULL.with("krub_icons::fonts::queue")
}

/// Applies the pins and unpins clicked since the last call to the app's saved list `pins`, then
/// hands it to this frame's font lists. Call once a frame, before the interface is drawn.
/// Returns whether `pins` changed.
pub fn sync(ctx: &Context, pins: &mut Vec<String>) -> bool {
    let queued: Vec<String> = ctx.data_mut(|d| d.remove_temp(queue_id())).unwrap_or_default();
    let mut changed = false;
    for name in queued {
        changed |= toggle(pins, &name);
    }
    let shown: Arc<Vec<String>> = ctx.data(|d| d.get_temp(pins_id())).unwrap_or_default();
    if *shown != *pins {
        ctx.data_mut(|d| d.insert_temp(pins_id(), Arc::new(pins.clone())));
    }
    changed
}

/// The pinned fonts handed over by the last [`sync`], plus the clicks queued since.
pub fn pinned(ctx: &Context) -> Vec<String> {
    let mut pins: Vec<String> = ctx.data(|d| d.get_temp::<Arc<Vec<String>>>(pins_id())).map(|a| a.as_ref().clone()).unwrap_or_default();
    let queued: Vec<String> = ctx.data(|d| d.get_temp(queue_id())).unwrap_or_default();
    for name in queued {
        toggle(&mut pins, &name);
    }
    pins
}

/// Pins or unpins `name` at the next [`sync`] (and asks for that frame).
pub fn toggle_later(ctx: &Context, name: &str) {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<String>>(queue_id()).push(name.to_string()));
    ctx.request_repaint();
}

/// A pushpin drawn in `r`, tilted, its point at the lower left: filled when `pinned`, an outline
/// otherwise. Original drawing.
pub fn paint_pin(p: &Painter, r: Rect, ink: Color32, pinned: bool) {
    let s = r.width().min(r.height()) / 16.0;
    let c = r.center();
    // Drawn upright on a 16-unit grid about (8, 8), then turned 45° clockwise.
    let (sin, cos) = std::f32::consts::FRAC_PI_4.sin_cos();
    let at = |x: f32, y: f32| -> Pos2 {
        let (dx, dy) = ((x - 8.0) * s, (y - 8.0) * s);
        pos2(c.x + dx * cos - dy * sin, c.y + dx * sin + dy * cos)
    };
    let width = (1.3 * s).max(1.0);
    let stroke = Stroke::new(width, ink);
    let head = vec![at(5.5, 1.5), at(10.5, 1.5), at(10.5, 3.2), at(5.5, 3.2)];
    let body = vec![at(6.4, 3.2), at(9.6, 3.2), at(10.4, 8.0), at(5.6, 8.0)];
    let fill = if pinned { ink } else { Color32::TRANSPARENT };
    p.add(Shape::convex_polygon(body, fill, stroke));
    p.add(Shape::convex_polygon(head, fill, stroke));
    p.line_segment([at(3.6, 8.4), at(12.4, 8.4)], stroke);
    p.line_segment([at(8.0, 8.4), at(8.0, 14.6)], stroke);
}

/// How a pin button looks and announces itself.
#[derive(Clone, Debug)]
pub struct PinStyle {
    pub ink: Color32,
    pub hover: Color32,
    /// The hover note and accessible name on a font that isn't pinned (the app's [`PIN`]).
    pub pin: String,
    /// The same on a pinned font (the app's [`UNPIN`]).
    pub unpin: String,
    /// Names the button for assistive technology; the apps run different egui versions, so
    /// each says it in its own API.
    pub announce: fn(&Response, &str),
}

/// A section heading row of a font list, `width` by `height`: `text` (translated) in `font` and
/// `color` at its left. Not clickable.
pub fn heading(ui: &mut Ui, width: f32, height: f32, text: &str, color: Color32, font: egui::FontId) -> Response {
    let (r, resp) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    ui.painter().text(pos2(r.left() + 6.0, r.center().y), egui::Align2::LEFT_CENTER, text, font, color);
    resp
}

/// The pin button's size.
pub const BUTTON: f32 = 20.0;

/// The pin button at the right end of a font's row `row` (already drawn), for font `name`: it
/// shows while the pointer is over the row or the font is pinned, and a click pins or unpins the
/// font ([`toggle_later`]). `id` tells it from the font's other row (a pinned font has two).
/// Returns whether it was clicked.
pub fn pin_button(ui: &mut Ui, id: Id, row: Rect, name: &str, pinned: bool, style: &PinStyle) -> bool {
    let r = Rect::from_center_size(pos2(row.right() - BUTTON * 0.5 - 2.0, row.center().y), vec2(BUTTON, BUTTON.min(row.height())));
    let resp = ui.interact(r, id, Sense::click());
    let label = if pinned { &style.unpin } else { &style.pin };
    (style.announce)(&resp, &format!("{label}: {name}"));
    let over_row = ui.rect_contains_pointer(row);
    if resp.hovered() {
        ui.painter().rect_filled(r, 3.0, style.hover);
    }
    if pinned || over_row || resp.hovered() || resp.has_focus() {
        let ink = if pinned || resp.hovered() { style.ink } else { style.ink.gamma_multiply(0.55) };
        paint_pin(ui.painter(), Rect::from_center_size(r.center(), vec2(14.0, 14.0)), ink, pinned);
    }
    let clicked = resp.clicked();
    resp.on_hover_text(label.as_str());
    if clicked {
        toggle_later(ui.ctx(), name);
    }
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn toggling_pins_at_the_end_and_unpins_without_regard_to_case() {
        let mut pins = Vec::new();
        assert!(toggle(&mut pins, "Sarabun"));
        assert!(toggle(&mut pins, " Arial "));
        assert_eq!(pins, names(&["Sarabun", "Arial"]));
        assert!(toggle(&mut pins, "sarabun"));
        assert_eq!(pins, names(&["Arial"]));
        assert!(!toggle(&mut pins, "   "));
        assert!(!toggle(&mut pins, &"x".repeat(MAX_NAME_CHARS + 1)));
        assert_eq!(pins, names(&["Arial"]));
    }

    #[test]
    fn pinning_past_the_limit_drops_the_oldest() {
        let mut pins = Vec::new();
        for i in 0..=MAX_PINNED {
            toggle(&mut pins, &format!("Font {i}"));
        }
        assert_eq!(pins.len(), MAX_PINNED);
        assert_eq!(pins.first().map(String::as_str), Some("Font 1"));
        assert_eq!(pins.last().map(String::as_str), Some(format!("Font {MAX_PINNED}").as_str()));
    }

    #[test]
    fn a_damaged_saved_list_is_cleaned() {
        let mut saved = names(&["  Arial ", "", "arial", "Sarabun"]);
        saved.push("y".repeat(MAX_NAME_CHARS + 1));
        saved.extend((0..MAX_PINNED).map(|i| format!("F{i}")));
        let clean = sanitized(saved);
        assert_eq!(clean.len(), MAX_PINNED);
        assert!(!clean.iter().any(|p| p == "Arial" || p == "Sarabun"), "the newest are kept");
        assert_eq!(sanitized(names(&[" Arial", "ARIAL", "Tahoma"])), names(&["Arial", "Tahoma"]));
    }

    #[test]
    fn pinned_fonts_head_the_list_and_stay_in_it() {
        let items = names(&["Arial", "Sarabun", "Tahoma"]);
        assert_eq!(rows(&items, &[]), (0..3).map(|index| Row::Font { index, pinned: false }).collect::<Vec<_>>());
        // A pin for a font that isn't in this list changes nothing.
        assert_eq!(rows(&items, &names(&["Garamond"])).len(), 3);
        let r = rows(&items, &names(&["tahoma", "Garamond", "Arial"]));
        assert_eq!(
            r,
            vec![
                Row::Heading(PINNED_FONTS),
                Row::Font { index: 2, pinned: true },
                Row::Font { index: 0, pinned: true },
                Row::Heading(ALL_FONTS),
                Row::Font { index: 0, pinned: true },
                Row::Font { index: 1, pinned: false },
                Row::Font { index: 2, pinned: true },
            ]
        );
    }

    #[test]
    fn clicks_reach_the_app_list_at_the_next_sync() {
        let ctx = Context::default();
        let mut saved = names(&["Arial"]);
        assert!(!sync(&ctx, &mut saved));
        assert_eq!(pinned(&ctx), names(&["Arial"]));
        toggle_later(&ctx, "Sarabun");
        toggle_later(&ctx, "Arial");
        // The lists drawn before the next sync already show the clicks.
        assert_eq!(pinned(&ctx), names(&["Sarabun"]));
        assert!(sync(&ctx, &mut saved));
        assert_eq!(saved, names(&["Sarabun"]));
        assert_eq!(pinned(&ctx), names(&["Sarabun"]));
        assert!(!sync(&ctx, &mut saved));
    }

    #[test]
    fn every_label_has_thai() {
        for key in [PINNED_FONTS, ALL_FONTS, PIN, UNPIN] {
            assert!(thai(key).is_some_and(|t| !t.is_empty()), "{key}");
        }
        assert_eq!(thai("Bold"), None);
    }

    #[test]
    fn the_pin_draws_in_any_rect() {
        let ctx = Context::default();
        let p = Painter::new(ctx, egui::LayerId::background(), Rect::EVERYTHING);
        for r in [Rect::from_min_size(Pos2::ZERO, vec2(14.0, 14.0)), Rect::NOTHING, Rect::from_min_size(Pos2::ZERO, vec2(0.0, 0.0))] {
            paint_pin(&p, r, Color32::BLACK, true);
            paint_pin(&p, r, Color32::BLACK, false);
        }
    }
}
