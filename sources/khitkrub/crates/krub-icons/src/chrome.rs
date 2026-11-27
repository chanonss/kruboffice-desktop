//! The band at the top of every KrubOffice app, the same in all three (owner, 2026-11-18: one
//! bar instead of a title bar over a tab strip):
//!
//! | part | height |
//! |---|---|
//! | the one bar: [`ONE_BAR_GAP`] above the tabs, then the tabs | [`ONE_BAR_H`] |
//! | ribbon (the buttons) | [`RIBBON_H`]: [`CONTENT_H`] of buttons and [`RIBBON_PAD`] above and below |
//! | group names (optional) | [`GROUP_NAMES_H`] more |
//!
//! The one bar holds, from the left: on macOS the traffic lights, then File, Save, Undo and Redo;
//! the tabs centred in the window (KrubOffice's signature, kept on the owner's second look the same
//! day); the document's name in a gold-outlined pill ([`file_pill`]; a click opens its
//! [`file_card`] with the state and Rename) in the room between the last tab and the user icon
//! ([`avatar`]), whose menu has Search, Comments and the editing mode. Where the ribbon is hidden
//! (File, Focus) the bar keeps the same height without the tabs.
//!
//! The apps run different egui versions, so this uses only what they share.

use egui::{Color32, FontId, Rect, Sense, Stroke, Ui, pos2, vec2};

/// The title bar of the old two-bar layout (Windows, Linux and the web).
pub const TITLE_H: f32 = 38.0;
/// The old title bar on macOS: the traffic lights are centred in it (16 points from the top of
/// the window on current macOS).
pub const MAC_TITLE_H: f32 = 32.0;
/// The tabs' row.
pub const TAB_STRIP_H: f32 = 30.0;
/// The space above the tabs, so they don't touch the top of the window.
pub const ONE_BAR_GAP: f32 = 8.0;
/// The one bar: the gap and the tabs.
pub const ONE_BAR_H: f32 = ONE_BAR_GAP + TAB_STRIP_H;
/// Where the macOS traffic lights' centres go, below the top of the window: the middle of the
/// tabs' row, in line with File, the tabs, the file pill and the user icon. AppKit puts them about
/// 16 points down, which looked too high (owner, 2026-11-27), so the apps move them here.
pub const MAC_LIGHTS_CENTRE_Y: f32 = ONE_BAR_GAP + TAB_STRIP_H / 2.0;
/// The buttons' area: two 28-point rows and the gap between them, or a large button.
pub const CONTENT_H: f32 = 60.0;
/// Space above and below the buttons.
pub const RIBBON_PAD: f32 = 5.0;
/// The ribbon under the tab strip.
pub const RIBBON_H: f32 = CONTENT_H + 2.0 * RIBBON_PAD;
/// The row of group names under the buttons, where an app offers it.
pub const GROUP_NAMES_H: f32 = 18.0;
/// The tab strip and the ribbon together.
pub const TABS_AND_RIBBON_H: f32 = TAB_STRIP_H + RIBBON_H;
/// The user icon's diameter.
pub const AVATAR: f32 = 26.0;
/// The narrowest the file pill gets before the tabs give way to the » menu.
pub const PILL_MIN_W: f32 = 140.0;
/// The space kept on each side of the pill.
pub const PILL_MARGIN: f32 = 12.0;

const _: () = assert!(TABS_AND_RIBBON_H == 100.0, "the KrubOffice ribbon is 100 points high");

/// Draws a mark in front of the file name (Google Drive's cloud) in the square it is given.
pub type Marker<'a> = &'a dyn Fn(&egui::Painter, Rect);

/// The file name's pill: `title` in `text`, outlined in `gold`, centred in `area` (the room
/// between the last tab and the buttons on the right) and cut with "…" to fit. `marker` draws
/// something in front of the name (Google Drive's cloud) in the 16-point square it is given.
/// Returns the pill's response (a click renames the document), or `None` when `area` has no room.
pub fn file_pill(ui: &mut Ui, area: Rect, title: &str, gold: Color32, text: Color32, marker: Option<Marker>) -> Option<egui::Response> {
    let font = FontId::proportional(12.5);
    let lead = if marker.is_some() { 20.0 } else { 0.0 };
    let room = area.width() - 2.0 * PILL_MARGIN - 24.0 - lead;
    if room < 40.0 {
        return None;
    }
    let width = |s: &str| ui.painter().layout_no_wrap(s.to_string(), font.clone(), text).size().x;
    let mut shown: String = title.chars().take(240).collect();
    if width(&shown) > room {
        while !shown.is_empty() && width(&format!("{shown}…")) > room {
            shown.pop();
        }
        shown.push('…');
    }
    let w = width(&shown) + 24.0 + lead;
    let h = 24.0;
    let pill = Rect::from_center_size(pos2(area.center().x, area.center().y), vec2(w, h));
    let resp = ui.interact(pill, crate::pane_ui::id("krub_file_pill"), Sense::click());
    let fill = if resp.hovered() { gold.gamma_multiply(0.18) } else { Color32::TRANSPARENT };
    ui.painter().rect(pill, h / 2.0, fill, Stroke::new(1.5, gold), egui::StrokeKind::Inside);
    if let Some(m) = marker {
        m(ui.painter(), Rect::from_center_size(pos2(pill.left() + 12.0 + 8.0, pill.center().y), vec2(16.0, 16.0)));
    }
    // Painted, not a label: the app names the pill (its button) with the full title, so the name
    // isn't announced twice.
    let galley = ui.painter().layout_no_wrap(shown, font, text);
    let at = pos2(pill.center().x + lead / 2.0 - galley.size().x / 2.0, pill.center().y - galley.size().y / 2.0);
    ui.painter().with_clip_rect(pill.shrink(2.0)).galley(at, galley, text);
    Some(resp)
}

/// What the file pill's card shows (owner, 2026-11-18): the whole name, its state ("Saved on
/// this device", "Edited"…), where it is kept, and one action (Rename, or Save As where the app
/// can't rename).
pub struct FileCard<'a> {
    pub name: &'a str,
    pub state: &'a str,
    pub place: Option<&'a str>,
    pub action: Option<&'a str>,
}

/// The icons in front of the card's lines (owner, 2026-11-22), each an icon name for
/// [`crate::paint_named`]; a line without one keeps its place in the icon column.
#[derive(Clone, Copy, Default)]
pub struct FileCardIcons<'a> {
    pub name: Option<&'a str>,
    pub state: Option<&'a str>,
    pub place: Option<&'a str>,
    pub action: Option<&'a str>,
}

/// The icon column's width, and the size an icon is drawn at.
const CARD_ICON: f32 = 16.0;
const CARD_ICON_GAP: f32 = 8.0;

/// Draws the file pill's card, opened by a click on the pill; `true` when its action was clicked.
pub fn file_card(ui: &mut Ui, card: &FileCard) -> bool {
    file_card_with_icons(ui, card, &FileCardIcons::default())
}

/// [`file_card`] with an icon in front of each line, the way a menu has them.
pub fn file_card_with_icons(ui: &mut Ui, card: &FileCard, icons: &FileCardIcons) -> bool {
    let any = icons.name.is_some() || icons.state.is_some() || icons.place.is_some() || icons.action.is_some();
    if !any {
        return file_card_plain(ui, card);
    }
    ui.set_min_width(240.0);
    ui.set_max_width(320.0);
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let line = |ui: &mut Ui, icon: Option<&str>, ink: Color32, label: egui::RichText| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = CARD_ICON_GAP;
            let (r, _) = ui.allocate_exact_size(vec2(CARD_ICON, CARD_ICON), Sense::hover());
            if let Some(name) = icon {
                crate::paint_named(ui.painter(), r, name, ink, true, false);
            }
            ui.add(egui::Label::new(label).wrap());
        });
    };
    line(ui, icons.name, text, egui::RichText::new(card.name).strong());
    line(ui, icons.state, weak, egui::RichText::new(card.state));
    if let Some(place) = card.place {
        line(ui, icons.place, weak, egui::RichText::new(place).weak());
    }
    let Some(label) = card.action else { return false };
    ui.add_space(2.0);
    ui.separator();
    card_row(ui, icons.action, label)
}

/// One more action under the file card, drawn like its own (an icon in the card's icon column and
/// the label): Save As, Comments. `true` when clicked.
pub fn card_row(ui: &mut Ui, icon_name: Option<&str>, label: &str) -> bool {
    let text = ui.visuals().text_color();
    // A menu row: icon and label inside one clickable, highlighted on hover.
    let font = egui::TextStyle::Button.resolve(ui.style());
    let text_size = ui.painter().layout_no_wrap(label.to_string(), font.clone(), text).size();
    let pad = ui.spacing().button_padding;
    let height = text_size.y.max(CARD_ICON) + pad.y * 2.0;
    let width = ui.available_width().max(pad.x * 2.0 + CARD_ICON + CARD_ICON_GAP + text_size.x);
    let (r, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, 4.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    let icon = Rect::from_min_size(pos2(r.left() + pad.x, r.center().y - CARD_ICON / 2.0), vec2(CARD_ICON, CARD_ICON));
    if let Some(name) = icon_name {
        crate::paint_named(ui.painter(), icon, name, text, true, false);
    }
    // A label, as on the avatar, so assistive technology reads the action; clicks pass through
    // it to the row.
    let at = Rect::from_min_size(pos2(icon.right() + CARD_ICON_GAP, r.center().y - text_size.y / 2.0), text_size);
    ui.put(at, egui::Label::new(egui::RichText::new(label).font(font).color(text)).selectable(false));
    resp.clicked()
}

fn file_card_plain(ui: &mut Ui, card: &FileCard) -> bool {
    ui.set_min_width(240.0);
    ui.set_max_width(320.0);
    ui.add(egui::Label::new(egui::RichText::new(card.name).strong()).wrap());
    ui.add_space(2.0);
    ui.label(card.state);
    if let Some(place) = card.place {
        ui.label(egui::RichText::new(place).weak());
    }
    match card.action {
        Some(label) => {
            ui.add_space(4.0);
            ui.button(label).clicked()
        }
        None => false,
    }
}

/// The user icon: `initials` in `ink` on a `fill` circle, [`AVATAR`] across, as a button named
/// `name` for assistive technology. Its menu is the app's.
pub fn avatar(ui: &mut Ui, initials: &str, fill: Color32, ink: Color32, name: &str) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(AVATAR, AVATAR), Sense::click());
    let ring = if resp.hovered() { Stroke::new(2.0, ink.gamma_multiply(0.6)) } else { Stroke::NONE };
    ui.painter().circle(r.center(), AVATAR / 2.0 - 1.0, fill, ring);
    let label = ui.put(
        Rect::from_center_size(r.center(), vec2(AVATAR - 6.0, 14.0)),
        egui::Label::new(egui::RichText::new(initials).font(FontId::proportional(10.5)).color(ink).strong()).selectable(false),
    );
    // The button is announced by its name (the initials alone say little).
    let _ = label.on_hover_text(name);
    resp.on_hover_text(name)
}

/// The room the pill may use: from `tabs_end` (the right edge of the last tab) to `right_start`
/// (the left edge of the buttons on the right), across the bar's tab row `row`.
pub fn pill_area(row: Rect, tabs_end: f32, right_start: f32) -> Rect {
    Rect::from_min_max(pos2(tabs_end.min(right_start), row.top()), pos2(right_start, row.bottom()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(f: impl FnMut(&mut Ui)) {
        let ctx = egui::Context::default();
        let mut f = f;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| f(ui));
        out.textures_delta.clear();
    }

    #[test]
    fn the_pill_is_centred_in_its_room_and_cut_to_fit() {
        let row = Rect::from_min_size(pos2(0.0, 8.0), vec2(1000.0, 30.0));
        let area = pill_area(row, 400.0, 900.0);
        assert_eq!((area.left(), area.right()), (400.0, 900.0));
        let mut got = None;
        run(|ui| got = file_pill(ui, area, "รายงานประจำปี — บันทึกแล้ว", Color32::GOLD, Color32::WHITE, None).map(|r| r.rect));
        let pill = got.expect("room for the pill");
        assert!((pill.center().x - 650.0).abs() < 0.5, "centred: {pill:?}");
        let narrow = pill_area(row, 400.0, 560.0);
        let mut got = None;
        run(|ui| got = file_pill(ui, narrow, &"ยาว".repeat(60), Color32::GOLD, Color32::WHITE, None).map(|r| r.rect));
        let pill = got.expect("room for a short pill");
        assert!(pill.width() <= narrow.width() - 2.0 * PILL_MARGIN + 0.5, "cut to fit: {pill:?} in {narrow:?}");
        let mut none = Some(Rect::NOTHING);
        run(|ui| none = file_pill(ui, pill_area(row, 400.0, 450.0), "x", Color32::GOLD, Color32::WHITE, None).map(|r| r.rect));
        assert!(none.is_none(), "no pill without room");
    }

    #[test]
    fn the_bar_keeps_the_ribbon_height() {
        assert_eq!(ONE_BAR_H, 38.0);
        assert_eq!(ONE_BAR_H, TITLE_H, "one bar is as tall as the old title bar alone");
    }
}
