//! The KrubOffice dialog look (owner, 2026-11-24, draft A of
//! `ui-experiments/dialog-style-2026-11-24`): the title bar as each app draws it, then a soft grey
//! body with a rim, each section a white card, tabs as one segmented control, and the buttons in
//! their own row under a hairline. Shared by the three apps so their dialogs look the same.

use egui::{Color32, CornerRadius, Frame, Margin, Rect, RichText, Sense, Stroke, Ui, pos2, vec2};

/// Space between the dialog's rim and its content (points).
pub const RIM: f32 = 14.0;
/// Space between two section cards.
pub const CARD_GAP: f32 = 10.0;
/// Space inside a section card.
pub const CARD_PAD: f32 = 12.0;
/// A section card's corners.
pub const CARD_RADIUS: u8 = 10;
/// The height of fields, drop-downs and buttons.
pub const FIELD_H: f32 = 28.0;
/// Fields', drop-downs' and buttons' corners.
pub const FIELD_RADIUS: u8 = 6;

/// The dialog body's grey.
pub fn body_fill(v: &egui::Visuals) -> Color32 {
    if v.dark_mode { Color32::from_gray(30) } else { Color32::from_rgb(0xEE, 0xF0, 0xF3) }
}

/// A section card's colour: the window's own (white in the light theme).
pub fn card_fill(v: &egui::Visuals) -> Color32 {
    if v.dark_mode { Color32::from_gray(44) } else { Color32::WHITE }
}

fn card_edge(v: &egui::Visuals) -> Stroke {
    Stroke::new(1.0, if v.dark_mode { Color32::from_gray(58) } else { Color32::from_rgb(0xE1, 0xE4, 0xE9) })
}

fn heading_color(v: &egui::Visuals) -> Color32 {
    if v.dark_mode { Color32::from_gray(200) } else { Color32::from_rgb(0x3B, 0x42, 0x50) }
}

/// A dialog window's frame: the app's window frame with the grey body (the title bar keeps its
/// own height; [`rim`] adds the rest of the space inside).
pub fn window_frame(style: &egui::Style) -> Frame {
    Frame::window(style).fill(body_fill(&style.visuals))
}

/// The dialog's content `add`, [`RIM`] from the window's edges (the window's own margin counted).
pub fn rim<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let own = ui.style().spacing.window_margin;
    let m = Margin {
        left: (RIM as i8 - own.left).max(0),
        right: (RIM as i8 - own.right).max(0),
        top: (RIM as i8 - own.top).max(0),
        bottom: (RIM as i8 - own.bottom).max(0),
    };
    Frame::new()
        .inner_margin(m)
        .show(ui, |ui| {
            set_area(ui.ctx(), ui.max_rect());
            add(ui)
        })
        .inner
}

/// Fields, drop-downs and buttons at one height with one corner radius, inside a dialog.
pub fn field_style(style: &mut egui::Style) {
    style.spacing.interact_size.y = FIELD_H;
    style.spacing.button_padding = vec2(12.0, 5.0);
    style.spacing.item_spacing = vec2(8.0, 6.0);
    for w in [
        &mut style.visuals.widgets.noninteractive,
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        w.corner_radius = CornerRadius::same(FIELD_RADIUS);
    }
}

const WIDTH_ID: &str = "krub_dialog_content_w";
const SECTIONS_ID: &str = "krub_dialog_sections";
const FOOTER_ID: &str = "krub_dialog_footer_y";
const AREA_ID: &str = "krub_dialog_area";

/// The dialog's content area (inside the rim), so the button row's hairline reaches its edges
/// from inside a padded card.
pub fn set_area(ctx: &egui::Context, area: Rect) {
    ctx.data_mut(|d| d.insert_temp(crate::pane_ui::id(AREA_ID), area));
}

/// Start a dialog's frame: `width` is its content's width on the last frame (so every card is
/// as wide as the widest); forgets what the last frame noted.
pub fn begin(ctx: &egui::Context, width: Option<f32>) {
    ctx.data_mut(|d| {
        d.insert_temp(crate::pane_ui::id(WIDTH_ID), width);
        d.insert_temp(crate::pane_ui::id(SECTIONS_ID), false);
        d.remove_temp::<f32>(crate::pane_ui::id(FOOTER_ID));
    });
}

/// Whether the dialog drew section cards this frame ([`section`]).
pub fn used_sections(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(crate::pane_ui::id(SECTIONS_ID))).unwrap_or(false)
}

/// Where the dialog's button row started this frame ([`footer`]), if it has one.
pub fn footer_top(ctx: &egui::Context) -> Option<f32> {
    ctx.data(|d| d.get_temp::<f32>(crate::pane_ui::id(FOOTER_ID)))
}

/// One section of a dialog: a white card with `title` (none when empty) above `add`'s content.
pub fn section<R>(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.ctx().data_mut(|d| d.insert_temp(crate::pane_ui::id(SECTIONS_ID), true));
    let width: Option<f32> = ui.ctx().data(|d| d.get_temp::<Option<f32>>(crate::pane_ui::id(WIDTH_ID))).flatten();
    // As wide as the dialog, or as its column when cards stand side by side.
    let width = width.map(|w| w.min(ui.available_width()));
    let v = ui.visuals().clone();
    let r = Frame::new()
        .fill(card_fill(&v))
        .stroke(card_edge(&v))
        .corner_radius(CornerRadius::same(CARD_RADIUS))
        .inner_margin(Margin::same(CARD_PAD as i8))
        .show(ui, |ui| {
            if let Some(w) = width {
                ui.set_min_width((w - 2.0 * CARD_PAD - 2.0).max(0.0));
            }
            if !title.is_empty() {
                ui.label(RichText::new(title).strong().size(12.5).color(heading_color(&v)));
                ui.add_space(2.0);
            }
            add(ui)
        })
        .inner;
    ui.add_space(CARD_GAP - ui.spacing().item_spacing.y);
    r
}

/// The card behind a dialog whose content isn't split into sections: from `top` down to
/// `bottom`, as wide as `content`, painted at `at` (a shape index taken before the content).
pub fn card_behind(ui: &Ui, at: egui::layers::ShapeIdx, content: Rect, bottom: f32) {
    let v = ui.visuals();
    let r = Rect::from_min_max(content.min - vec2(CARD_PAD, CARD_PAD), pos2(content.max.x + CARD_PAD, bottom));
    ui.painter().set(at, egui::epaint::RectShape::new(r, CornerRadius::same(CARD_RADIUS), card_fill(v), card_edge(v), egui::StrokeKind::Inside));
}

/// The button row's start: a hairline across the dialog (to its rim), then room above the
/// buttons. Notes where it is ([`footer_top`]).
pub fn footer(ui: &mut Ui) {
    let y = ui.cursor().min.y;
    ui.ctx().data_mut(|d| d.insert_temp(crate::pane_ui::id(FOOTER_ID), y));
    let r = ui.ctx().data(|d| d.get_temp::<Rect>(crate::pane_ui::id(AREA_ID))).unwrap_or_else(|| ui.max_rect());
    let line_y = y + 2.0;
    ui.painter().line_segment([pos2(r.min.x - RIM, line_y), pos2(r.max.x + RIM, line_y)], card_edge(ui.visuals()));
    ui.add_space(RIM);
}

/// Tabs as one segmented control across the dialog: `labels` (shown as given), the current
/// one raised on white. Returns the one clicked.
pub fn tabs(ui: &mut Ui, labels: &[String], current: usize) -> Option<usize> {
    let v = ui.visuals().clone();
    let track = if v.dark_mode { Color32::from_gray(52) } else { Color32::from_rgb(0xDF, 0xE2, 0xE7) };
    let mut picked = None;
    let width: Option<f32> = ui.ctx().data(|d| d.get_temp::<Option<f32>>(crate::pane_ui::id(WIDTH_ID))).flatten();
    Frame::new().fill(track).corner_radius(CornerRadius::same(8)).inner_margin(Margin::same(3)).show(ui, |ui| {
        if let Some(w) = width {
            ui.set_min_width((w - 8.0).max(0.0));
        }
        let font = egui::FontId::proportional(13.0);
        let n = labels.len().max(1) as f32;
        let texts: Vec<_> = labels.iter().map(|l| ui.painter().layout_no_wrap(l.clone(), font.clone(), v.text_color())).collect();
        let natural = texts.iter().map(|g| g.size().x + 24.0).fold(0.0_f32, f32::max);
        let seg_w = (ui.available_width() / n).max(natural).floor();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (i, (label, galley)) in labels.iter().zip(texts).enumerate() {
                let (r, resp) = ui.allocate_exact_size(vec2(seg_w, 26.0), Sense::click());
                let on = i == current;
                if on {
                    ui.painter().rect_filled(r.shrink(0.5), CornerRadius::same(6), card_fill(&v));
                    ui.painter().rect_stroke(r.shrink(0.5), CornerRadius::same(6), card_edge(&v), egui::StrokeKind::Inside);
                } else if resp.hovered() {
                    ui.painter().rect_filled(r.shrink(0.5), CornerRadius::same(6), track.gamma_multiply(0.7));
                }
                let color = if on { v.text_color() } else { heading_color(&v) };
                ui.painter().galley(r.center() - galley.size() / 2.0, galley, color);
                resp.widget_info(|| tab_info(on, label));
                if resp.clicked() && !on {
                    picked = Some(i);
                }
            }
        });
    });
    ui.add_space(CARD_GAP - ui.spacing().item_spacing.y);
    picked
}

/// A tab's accessible description: a button, selected when it is the current tab. (`WidgetType`
/// is renamed in newer egui; the three apps don't all use the same one yet.)
#[allow(deprecated)]
fn tab_info(on: bool, label: &str) -> egui::WidgetInfo {
    egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, label)
}
