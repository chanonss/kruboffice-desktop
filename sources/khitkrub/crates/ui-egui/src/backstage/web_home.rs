//! The web start screen (nothing opened yet in this tab): a top bar (KhitKrub, Open file, Options, the
//! KrubOffice apps), a greeting, the new-workbook tiles, then the drafts kept in this browser.
//! Choosing a tile or a draft opens it in a new tab (`./?new=blank`, `./?sample=…`, `./?draft=…`), so
//! this page stays where it is. File › Home inside the editor stays as it is.

use super::{
    Align2, Color32, Icon, Language, Page, Rect, RichText, Sense, SheetApp, Stroke, StrokeKind, Tokens, Ui, icons, json, pos2, theme, tile, vec2,
};
use crate::DraftAction;

/// The icon's colours: a gold ค on the app's teal, as on the launcher.
const MARK_BACK: Color32 = theme::BAND_TEAL;
const MARK_GOLD: Color32 = Color32::from_rgb(0xF2, 0xB7, 0x05);

/// `text` as a URL query value: letters, digits and `-_.~` stay, every other byte becomes `%XX`.
pub(crate) fn url_component(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// A Thai letter drawn at `centre` when a font has it (the logo's ค, the apps' ข and พ); otherwise
/// nothing, rather than a missing-glyph box.
fn letter(painter: &egui::Painter, centre: egui::Pos2, c: &str, font: egui::FontId, color: Color32) {
    let has = c.chars().next().is_some_and(|ch| painter.fonts_mut(|f| f.has_glyph(&font, ch)));
    if has {
        krub_icons::glass_letter(painter, centre, c, font, color);
    }
}

/// The address `./?{key}={value}` (or `./?{key}` for an empty value) that opens one choice.
pub(crate) fn address(key: &str, value: &str) -> String {
    if value.is_empty() { format!("./?{key}") } else { format!("./?{key}={}", url_component(value)) }
}

/// Open `address(key, value)` in a new tab.
fn open_in_new_tab(ui: &Ui, key: &str, value: &str) {
    ui.ctx().open_url(egui::OpenUrl::new_tab(address(key, value)));
}

/// A button with a drawn icon before its text.
fn icon_text_button(ui: &mut Ui, icon: Icon, text: &str) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let font = theme::ui_font(13.0);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, t.text);
    let size = vec2(10.0 + 16.0 + 6.0 + galley.size().x + 10.0, 30.0);
    let (r, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, text));
    let fill = if resp.is_pointer_button_down_on() {
        t.pressed
    } else if resp.hovered() {
        t.hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect(r, 5.0, fill, Stroke::new(1.0, t.input_border), StrokeKind::Inside);
    icons::paint(ui.painter(), Rect::from_center_size(pos2(r.min.x + 18.0, r.center().y), vec2(16.0, 16.0)), icon, t.text);
    ui.painter().galley(pos2(r.min.x + 32.0, r.center().y - galley.size().y / 2.0), galley, t.text);
    resp
}

/// "Google Drive" with the shared cloud icon: opens File › Open, where Drive is.
fn cloud_button(ui: &mut Ui) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let galley = ui.painter().layout_no_wrap("Google Drive".to_string(), theme::ui_font(13.0), t.text);
    let size = vec2(10.0 + 16.0 + 6.0 + galley.size().x + 10.0, 30.0);
    let (r, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Google Drive"));
    let fill = if resp.hovered() { t.hover } else { Color32::TRANSPARENT };
    ui.painter().rect(r, 5.0, fill, Stroke::new(1.0, t.input_border), StrokeKind::Inside);
    krub_icons::paint_named(
        ui.painter(),
        Rect::from_center_size(pos2(r.min.x + 18.0, r.center().y), vec2(16.0, 16.0)),
        "cloud",
        t.text,
        false,
        false,
    );
    ui.painter().galley(pos2(r.min.x + 32.0, r.center().y - galley.size().y / 2.0), galley, t.text);
    resp
}

pub(crate) fn web_home(app: &mut SheetApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let lang = app.ui.language;
    egui::Panel::top("web_home_bar")
        .show_separator_line(false)
        .frame(egui::Frame::NONE.fill(t.ribbon).inner_margin(egui::Margin::symmetric(20, 10)).stroke(Stroke::new(1.0, t.ribbon_border)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                ui.painter().rect_filled(r, 8.0, MARK_BACK);
                letter(ui.painter(), r.center() + vec2(0.0, 1.0), "ค", theme::ui_bold(22.0), MARK_GOLD);
                ui.add_space(4.0);
                // The name gives way to the buttons on narrow screens (an iPad in portrait).
                if ui.available_width() > 560.0 {
                    ui.label(RichText::new(lang.tr("KhitKrub")).font(theme::ui_bold(17.0)).color(t.text));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    app_switcher(ui, lang);
                    if icon_text_button(ui, Icon::Settings, lang.tr("Options")).clicked() {
                        app.open_backstage(Page::Options);
                    }
                    if app.services.google_drive.is_some() && cloud_button(ui).clicked() {
                        app.open_backstage(Page::Open);
                    }
                    if icon_text_button(ui, Icon::OpenFolder, lang.tr("Open file")).clicked() {
                        app.open_dialog("open", json!({}));
                    }
                });
            });
        });
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.window)).show(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            // A centred column on wide windows.
            let width = ui.available_width().min(1000.0);
            let side = ((ui.available_width() - width) / 2.0).max(0.0) + 24.0;
            ui.add_space(28.0);
            ui.horizontal_top(|ui| {
                ui.add_space(side);
                ui.vertical(|ui| {
                    ui.set_max_width(width - 48.0);
                    ui.label(RichText::new(lang.tr("Hello! What would you like to work on?")).font(theme::ui_bold(24.0)).color(t.text));
                    ui.label(
                        RichText::new(lang.tr("Pick a workbook to start with, or pick up a draft where you left off."))
                            .font(theme::ui_font(13.5))
                            .color(t.text_dim),
                    );
                    ui.add_space(18.0);
                    new_tiles(ui, lang);
                    ui.add_space(28.0);
                    drafts_list(app, ui);
                    ui.add_space(28.0);
                });
            });
        });
    });
}

/// Blank workbook and the built-in samples; each opens in a new tab.
fn new_tiles(ui: &mut Ui, lang: Language) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(18.0, 14.0);
        if tile(ui, lang.tr("Blank workbook"), None).clicked() {
            open_in_new_tab(ui, "new", "blank");
        }
        for (i, (name, title)) in gridcraft_engine::sample::SAMPLES.iter().enumerate() {
            if tile(ui, lang.tr(title), Some(i)).clicked() {
                open_in_new_tab(ui, "sample", name);
            }
        }
    });
}

/// The suite's app switcher in the top bar: a 3 x 3 dot button that opens a menu of the KrubOffice
/// apps (this one marked) and a link to kruboffice.app.
fn app_switcher(ui: &mut Ui, lang: Language) {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, lang.tr("KrubOffice apps")));
    let resp = resp.on_hover_text(lang.tr("KrubOffice apps"));
    if resp.hovered() {
        ui.painter().rect_filled(r, 6.0, t.hover);
    }
    for (dx, dy) in (0..9).map(|i| ((i % 3) as f32 - 1.0, (i / 3) as f32 - 1.0)) {
        ui.painter().circle_filled(pos2(r.center().x + dx * 6.0, r.center().y + dy * 6.0), 1.8, t.text);
    }
    egui::Popup::menu(&resp).align(egui::RectAlign::BOTTOM_END).show(|ui| {
        ui.set_min_width(268.0);
        ui.label(RichText::new("KrubOffice").font(theme::ui_font(11.5)).color(t.text_dim));
        let (navy, teal, coral) = (Color32::from_rgb(0x1E, 0x3A, 0x8A), MARK_BACK, Color32::from_rgb(0x9A, 0x3A, 0x1F));
        if app_switcher_row(ui, "ข", navy, lang.tr("KhianKrub"), lang.tr("Documents"), None, true) {
            ui.ctx().open_url(egui::OpenUrl::new_tab("https://khiankrub.app/"));
            ui.close();
        }
        app_switcher_row(ui, "ค", teal, lang.tr("KhitKrub"), lang.tr("Spreadsheets"), Some(lang.tr("Here")), false);
        if app_switcher_row(ui, "พ", coral, lang.tr("PhutKrub"), lang.tr("Presentations"), None, true) {
            ui.ctx().open_url(egui::OpenUrl::new_tab("https://phutkrub.app/"));
            ui.close();
        }
        ui.separator();
        if ui.button(lang.tr("See all apps at kruboffice.app")).clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab("https://kruboffice.app/"));
            ui.close();
        }
    });
}

/// One app in the switcher: its tile (a gold Thai letter on the app's colour), name and kind.
/// `badge` marks the app you are in; an app that isn't there yet (not `live`, no badge) draws
/// faded. True when a live app other than this one is clicked.
fn app_switcher_row(ui: &mut Ui, thai_letter: &str, tile: Color32, name: &str, kind: &str, badge: Option<&str>, live: bool) -> bool {
    let t = Tokens::get(ui.ctx());
    let faded = badge.is_none() && !live;
    let fade = if faded { 0.45 } else { 1.0 };
    let sense = if live { Sense::click() } else { Sense::hover() };
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width().max(268.0), 44.0), sense);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("{name}, {kind}")));
    if badge.is_some() || resp.hovered() {
        ui.painter().rect_filled(r, 8.0, t.hover);
    }
    let tile_rect = Rect::from_min_size(pos2(r.min.x + 6.0, r.min.y + 7.0), vec2(30.0, 30.0));
    ui.painter().rect_filled(tile_rect, 8.0, tile.gamma_multiply(fade));
    self::letter(ui.painter(), tile_rect.center() + vec2(0.0, 1.0), thai_letter, theme::ui_bold(17.0), MARK_GOLD.gamma_multiply(fade));
    let x = tile_rect.max.x + 10.0;
    let name_color = if faded { t.text_dim } else { t.text };
    ui.painter().text(pos2(x, r.min.y + 15.0), Align2::LEFT_CENTER, name, theme::ui_font(13.5), name_color);
    ui.painter().text(pos2(x, r.min.y + 31.0), Align2::LEFT_CENTER, kind, theme::ui_font(11.5), t.text_dim);
    if let Some(badge) = badge {
        let galley = ui.painter().layout_no_wrap(badge.to_string(), theme::ui_font(11.0), t.on_accent);
        let pill = Rect::from_center_size(pos2(r.max.x - 10.0 - galley.size().x / 2.0 - 8.0, r.center().y), galley.size() + vec2(16.0, 4.0));
        ui.painter().rect_filled(pill, pill.height() / 2.0, t.accent);
        ui.painter().galley(pill.center() - galley.size() / 2.0, galley, t.on_accent);
    }
    live && resp.clicked()
}

/// The drafts kept in this browser. Click a row to keep working in a new tab; its ⋮ (or a right-click)
/// downloads a copy or discards it, after asking.
pub(super) fn drafts_list(app: &mut SheetApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let lang = app.ui.language;
    let Some(recovery) = &app.services.browser_recovery else { return };
    let drafts = (recovery.drafts)(lang.code());
    let width = ui.available_width().min(700.0);
    ui.label(RichText::new(lang.tr("Drafts on this device")).font(theme::ui_bold(16.0)).color(t.text));
    let note = if app.services.google_drive.is_some() {
        "Kept only in this browser on this device. Save to Google Drive or download a copy to keep it anywhere else."
    } else {
        "Kept only in this browser on this device. Download a copy to keep it anywhere else."
    };
    ui.label(RichText::new(lang.tr(note)).font(theme::ui_font(12.0)).color(t.text_dim));
    if (recovery.fragile)() {
        let warning = "This browser may remove drafts by itself: when it runs short of space, after a long time without a visit, or when a private window closes.";
        ui.label(RichText::new(lang.tr(warning)).font(theme::ui_font(12.0)).color(t.text_dim));
    }
    ui.add_space(6.0);
    let Some(drafts) = drafts else {
        ui.label(RichText::new(lang.tr("Loading drafts…")).color(t.text_dim));
        return;
    };
    if drafts.is_empty() {
        ui.label(
            RichText::new(lang.tr("Workbooks you make here are kept in this browser, so you can pick up where you left off.")).color(t.text_dim),
        );
        return;
    }
    let confirm_id = ui.make_persistent_id("web_home_discard_draft");
    let confirm = ui.ctx().data(|d| d.get_temp::<String>(confirm_id));
    let mut action: Option<(DraftAction, String)> = None;
    for draft in &drafts {
        // Every row is a draft until it is discarded, saved elsewhere or not.
        let label = format!("{} ({})", draft.name, lang.tr("draft"));
        let (r, resp) = ui.allocate_exact_size(vec2(width, 40.0), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
        let more_rect = Rect::from_center_size(pos2(r.max.x - 20.0, r.center().y), vec2(26.0, 26.0));
        let more = ui.interact(more_rect, resp.id.with("more"), Sense::click());
        more.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, lang.tr("More options")));
        if ui.rect_contains_pointer(r) {
            ui.painter().rect_filled(r, 4.0, t.hover);
        }
        if more.hovered() {
            ui.painter().rect_filled(more_rect, 4.0, t.pressed);
        }
        icons::paint(ui.painter(), Rect::from_center_size(pos2(r.min.x + 18.0, r.center().y), vec2(22.0, 22.0)), Icon::Table, t.text);
        let name = ui.painter().layout_no_wrap(label.clone(), theme::ui_font(13.0), t.text);
        let clip = Rect::from_min_size(pos2(r.min.x + 40.0, r.min.y), vec2((width - 56.0).max(0.0), r.height()));
        ui.painter().with_clip_rect(clip).galley(pos2(r.min.x + 40.0, r.min.y + 13.0 - name.size().y / 2.0), name, t.text);
        ui.painter().text(pos2(r.min.x + 40.0, r.min.y + 29.0), Align2::LEFT_CENTER, &draft.saved, theme::ui_font(11.0), t.text_dim);
        // Three dots, drawn: the ⋮ menu.
        for dy in [-5.0, 0.0, 5.0] {
            ui.painter().circle_filled(pos2(more_rect.center().x, more_rect.center().y + dy), 1.6, t.text);
        }
        let menu = |ui: &mut Ui, action: &mut Option<(DraftAction, String)>| {
            if ui.button(lang.tr("Keep working")).clicked() {
                *action = Some((DraftAction::Open, draft.id.clone()));
                ui.close();
            }
            if ui.button(lang.tr("Download a copy")).clicked() {
                *action = Some((DraftAction::Download, draft.id.clone()));
                ui.close();
            }
            ui.separator();
            if ui.button(lang.tr("Discard draft…")).clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(confirm_id, draft.id.clone()));
                ui.close();
            }
        };
        // Opens leftwards from the ⋮ at the row's end, so it stays inside the window.
        egui::Popup::menu(&more).align(egui::RectAlign::BOTTOM_END).show(|ui| menu(ui, &mut action));
        resp.context_menu(|ui| menu(ui, &mut action));
        if resp.clicked() && !more.clicked() {
            action = Some((DraftAction::Open, draft.id.clone()));
        }
        if confirm.as_deref() == Some(draft.id.as_str()) {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(40.0);
                ui.label(lang.tr("Permanently discard this draft and its previous copy?"));
                if ui.button(lang.tr("Discard draft")).clicked() {
                    action = Some((DraftAction::Discard, draft.id.clone()));
                    ui.ctx().data_mut(|d| d.remove::<String>(confirm_id));
                }
                if ui.button(lang.tr("Cancel")).clicked() {
                    ui.ctx().data_mut(|d| d.remove::<String>(confirm_id));
                }
            });
        }
    }
    if let Some((what, id)) = action {
        if matches!(what, DraftAction::Open) {
            open_in_new_tab(ui, "draft", &id);
        } else if let Some(recovery) = &app.services.browser_recovery {
            (recovery.act)(what, &id);
        }
    }
}
