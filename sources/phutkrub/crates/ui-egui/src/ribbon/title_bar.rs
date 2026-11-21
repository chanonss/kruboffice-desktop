use super::{Rect, Sense, SlideApp, Tokens, Ui, json, pos2, theme};

pub const TITLE_H: f32 = krub_icons::chrome::TITLE_H;

/// macOS draws the window into the title bar (`with_fullsize_content_view`) and AppKit paints the
/// traffic lights itself, 16 pt from the top of the window on macOS 27. Moving them would take
/// AppKit calls (`unsafe`, forbidden here), so on macOS the title bar is the band they are centred
/// in, with the controls beside them (#71). Up to macOS 15 the lights are smaller and centred 14 pt
/// from the top, about 2 pt above this bar's centre.
pub const MAC_TITLE_BAR: f32 = krub_icons::chrome::MAC_TITLE_H;

/// The traffic lights as AppKit lays them out, measured on macOS 27: button diameter,
/// centre-to-centre pitch and the first button's left edge.
pub(super) const MAC_LIGHT: f32 = 14.0;

pub(super) const MAC_LIGHT_PITCH: f32 = 23.0;

pub(super) const MAC_LIGHTS_LEFT: f32 = 9.0;

/// Where title-bar content starts on macOS: the end of the three lights plus the gap the lights
/// keep from the top of the window.
pub const MAC_CONTENT_LEFT: f32 = MAC_LIGHTS_LEFT + 2.0 * MAC_LIGHT_PITCH + MAC_LIGHT + (MAC_TITLE_BAR - MAC_LIGHT) / 2.0;

/// The title bar's height, the left edge of its content and the height of its tallest control.
/// With the integrated title bar (macOS) the bar is shorter, so the controls shrink to keep a
/// margin above and below them.
pub(super) fn title_geometry(integrated: bool) -> (f32, f32, f32) {
    if integrated { (MAC_TITLE_BAR, MAC_CONTENT_LEFT, 22.0) } else { (TITLE_H, 10.0, 26.0) }
}

/// The Quick Access Toolbar's width: its buttons, the ▾ beside Undo and ⌄, so the bar keeps them in
/// order at its right end.
pub(super) fn quick_access_width(app: &SlideApp) -> f32 {
    let items = &app.ui.qat;
    let undo = if items.iter().any(|q| q.id == "edit.undo") { krub_icons::qat::ARROW.x + 2.0 } else { 0.0 };
    items.len() as f32 * (crate::ribbon_layout::SMALL_W + 2.0) + undo + krub_icons::qat::ARROW.x
}

/// The user's initials for the user icon: the first letters of the first two words of the name
/// comments are signed with.
fn initials(name: &str) -> String {
    let s: String = name.split_whitespace().take(2).filter_map(|w| w.chars().next()).flat_map(char::to_uppercase).collect();
    if s.is_empty() { "?".into() } else { s }
}

/// The Quick Access Toolbar on the bar's left, after File (owner, 2026-11-18).
pub(super) fn bar_quick_access(app: &mut SlideApp, ui: &mut Ui) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        theme::band_ui(ui);
        crate::qat::show(app, ui);
    });
}

/// The right end of the one bar (owner, 2026-11-18): the user icon with its menu. Returns its left
/// edge.
pub(super) fn bar_right(app: &mut SlideApp, ui: &mut Ui) -> f32 {
    let t = Tokens::get(ui.ctx()).on_band();
    let resp = krub_icons::chrome::avatar(ui, &initials(&app.session.prefs.author), t.accent, t.accent_text, tl!("Account"));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!("Account")));
    egui::Popup::menu(&resp).show(|ui| {
        ui.set_min_width(240.0);
        user_menu(app, ui);
    });
    ui.min_rect().left()
}

/// The user icon's menu (owner, 2026-11-18): the name comments are signed with (opens
/// Preferences), Present, Search commands, Comments and the light or dark appearance.
fn user_menu(app: &mut SlideApp, ui: &mut Ui) {
    let name = app.session.prefs.author.clone();
    if ui
        .button(egui::RichText::new(if name.is_empty() { tl!("Your name").to_string() } else { name }).strong())
        .on_hover_text(tl!("Set your name in Preferences"))
        .clicked()
    {
        let _ = app.run("app.preferences", json!({"pane": "general"}));
        ui.close();
    }
    ui.separator();
    let doc = app.session.active().is_some();
    if ui.add_enabled(doc, egui::Button::new(tl!("Present from Current Slide"))).clicked()
        && let Some(from) = app.session.active().map(|d| d.selection.slide)
    {
        app.start_show(from, false);
        ui.close();
    }
    if ui.button(tl!("Search commands and help")).clicked() {
        app.palette = Some((String::new(), 0));
        ui.close();
    }
    if ui.add_enabled(doc, egui::Button::new(tl!("Comments"))).clicked() {
        let _ = app.run("view.pane", json!({"pane": "comments", "toggle": true}));
        ui.close();
    }
    ui.separator();
    ui.label(egui::RichText::new(tl!("Appearance")).small().weak());
    let dark = app.ui.brightness == theme::Brightness::Dark;
    for (want_dark, label) in [(false, "Light"), (true, "Dark")] {
        if ui.radio(dark == want_dark, tl!(label)).clicked() && dark != want_dark {
            let _ = app.run("view.dark", json!({}));
            ui.close();
        }
    }
}

/// Dragging the bar's empty parts moves the window; a double-click maximises it. Registered before
/// the bar's buttons, which take their own clicks.
pub(super) fn window_drag(ui: &mut Ui, bar: Rect) {
    let drag = ui.interact(bar, ui.scope_id().with("titledrag"), Sense::click_and_drag());
    // The drag surface is not a button: a full-width Button would be announced as one.
    drag.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Window, true, tl!("Window title bar")));
    if drag.drag_started() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if drag.double_clicked() {
        let max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
    }
}

/// The file pill (`krub_icons::chrome::file_pill`) centred in `area`: the presentation's name. A
/// click opens its card: the whole name, its state, where it is kept and Save As (owner,
/// 2026-11-18).
pub(super) fn file_pill(app: &mut SlideApp, ui: &mut Ui, area: Rect) {
    let t = Tokens::get(ui.ctx()).on_band();
    let Some(d) = app.session.active() else { return };
    let name = shown_name(d);
    let draft = app.services.browser_recovery.as_ref().and_then(|r| (r.open_draft)());
    let state = match draft {
        Some(true) => tl!("Saved on this device"),
        Some(false) => tl!("Saving…"),
        None if d.is_dirty() => tl!("Edited"),
        None if d.path.is_some() => tl!("Saved"),
        None => tl!("Not saved"),
    }
    .to_string();
    let place = match (&d.path, draft) {
        (Some(path), _) => Some(std::path::Path::new(path).parent().map_or_else(|| path.clone(), |p| p.display().to_string())),
        (None, Some(_)) => Some(tl!("Draft in this browser").to_string()),
        (None, None) => None,
    };
    let Some(resp) = krub_icons::chrome::file_pill(ui, area, &name, t.accent, t.text, None) else { return };
    // Named as the old title was ("Trip plan (draft) — Saving…", "Presentation1 — Edited").
    let label = document_title(app);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
    egui::Popup::menu(&resp).show(|ui| {
        let card = krub_icons::chrome::FileCard { name: &name, state: &state, place: place.as_deref(), action: Some(tl!("Save As…")) };
        if krub_icons::chrome::file_card(ui, &card) {
            app.open_backstage(crate::backstage::Page::SaveAs);
            ui.close();
        }
    });
}

/// The one bar where the ribbon is hidden (File): the same band and height as the ribbon's bar,
/// the traffic lights' room on macOS, the file pill and the right end, no tabs.
pub fn title_bar(app: &mut SlideApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx()).on_band();
    let (_, left, _) = title_geometry(app.integrated_titlebar);
    egui::Panel::top("title_bar").exact_size(krub_icons::chrome::ONE_BAR_H).show_separator_line(false).frame(egui::Frame::NONE.fill(t.band)).show(
        ui,
        |ui| {
            let rect = ui.max_rect();
            window_drag(ui, rect);
            let row = Rect::from_min_max(pos2(rect.min.x + left, rect.min.y + krub_icons::chrome::ONE_BAR_GAP), pos2(rect.max.x - 10.0, rect.max.y));
            let right = ui
                .scope_builder(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::right_to_left(egui::Align::Center)), |ui| {
                    theme::band_ui(ui);
                    bar_right(app, ui)
                })
                .inner;
            let left = ui
                .scope_builder(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| {
                    bar_quick_access(app, ui);
                    ui.min_rect().right()
                })
                .inner;
            file_pill(app, ui, krub_icons::chrome::pill_area(row, left, right));
        },
    );
}

/// The centred title: the active presentation's name and whether it is saved. A new presentation on
/// the web lives as its draft in this browser until it is saved as a file or to Google Drive.
pub(crate) fn document_title(app: &SlideApp) -> String {
    let Some(d) = app.session.active() else { return tl!("PhutKrub").into() };
    match app.services.browser_recovery.as_ref().and_then(|r| (r.open_draft)()) {
        Some(saved) => format!("{} ({}) — {}", shown_name(d), tl!("draft"), if saved { tl!("Saved on this device") } else { tl!("Saving…") }),
        None => format!("{}{}", shown_name(d), if d.is_dirty() { format!(" — {}", tl!("Edited")) } else { String::new() }),
    }
}

/// The presentation's name as shown: a new, never-saved one ("Presentation1") in the interface
/// language ("งานนำเสนอ1"); any other name exactly as it is.
fn shown_name(d: &deckcraft_engine::DocState) -> String {
    let title = d.title();
    match title.strip_prefix("Presentation") {
        Some(n) if d.path.is_none() && !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => format!("{}{n}", tl!("Presentation")),
        _ => title,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A new web presentation kept only as its browser draft says so in the title bar, instead of
    /// "Edited", and says whether its latest edit is in the draft yet.
    #[test]
    fn the_title_names_a_browser_draft() {
        let saved = std::rc::Rc::new(std::cell::Cell::new(Some(false)));
        let shown = saved.clone();
        let mut app = SlideApp::new(deckcraft_engine::Session::new(), crate::Services::default());
        app.ui.language = "en".into();
        app.run("file.new", json!({})).unwrap();
        let d = app.session.active_mut().unwrap();
        d.title = "Trip plan".into();
        d.saved_doc = std::sync::Arc::new((*d.doc).clone());
        app.services.browser_recovery = Some(crate::BrowserRecoveryServices {
            drafts: Box::new(|_| Some(Vec::new())),
            act: Box::new(|_, _| {}),
            fragile: Box::new(|| false),
            open_draft: Box::new(move || shown.get()),
        });
        assert_eq!(document_title(&app), "Trip plan (draft) — Saving…");
        saved.set(Some(true));
        assert_eq!(document_title(&app), "Trip plan (draft) — Saved on this device");
        saved.set(None);
        assert_eq!(document_title(&app), "Trip plan — Edited", "a presentation saved elsewhere is not a draft");
    }

    /// In Thai a new, never-saved presentation is "งานนำเสนอ1" and an edit says "แก้ไขแล้ว"; a name
    /// the user gave stays as it is.
    #[test]
    fn the_title_is_in_thai() {
        let mut app = SlideApp::new(deckcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        app.run("slide.new", json!({})).unwrap();
        crate::i18n::set_current(crate::i18n::lang_from_tag("th").unwrap());
        let shown = document_title(&app);
        let named = {
            let d = app.session.active_mut().unwrap();
            d.title = "Presentation plan".into();
            document_title(&app)
        };
        crate::i18n::set_current(crate::i18n::Lang::EN);
        assert!(shown.starts_with("งานนำเสนอ") && shown.ends_with("— แก้ไขแล้ว") && !shown.contains("Presentation"), "{shown}");
        assert_eq!(named, "Presentation plan — แก้ไขแล้ว");
    }
}
