use super::{Align2, Icon, Rect, Sense, SlideApp, Tokens, Ui, icon_toggle, json, pos2, theme};

pub const TITLE_H: f32 = 38.0;

/// macOS draws the window into the title bar (`with_fullsize_content_view`) and AppKit paints the
/// traffic lights itself, 16 pt from the top of the window on macOS 27. Moving them would take
/// AppKit calls (`unsafe`, forbidden here), so on macOS the title bar is the band they are centred
/// in, with the controls beside them (#71). Up to macOS 15 the lights are smaller and centred 14 pt
/// from the top, about 2 pt above this bar's centre.
pub const MAC_TITLE_BAR: f32 = 32.0;

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

/// The window's title row: traffic-light space, quick access, centred title, actions.
pub fn title_bar(app: &mut SlideApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx()).on_band();
    let (height, left, _) = title_geometry(app.integrated_titlebar);
    egui::Panel::top("title_bar").exact_size(height).show_separator_line(false).frame(egui::Frame::NONE.fill(t.band)).show(ui, |ui| {
        let rect = ui.max_rect();
        // Dragging the empty title bar moves the window.
        let drag = ui.interact(rect, ui.scope_id().with("titledrag"), Sense::click_and_drag());
        // The drag surface is not a button: a full-width Button would be announced as one.
        drag.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Window, true, tl!("Window title bar")));
        if drag.drag_started() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if drag.double_clicked() {
            let max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
        }
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(pos2(rect.min.x + left, rect.min.y), pos2(rect.min.x + 420.0, rect.max.y)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.spacing_mut().item_spacing.x = 2.0;
        theme::band_ui(&mut child);
        crate::qat::show(app, &mut child);
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, document_title(app), theme::font(13.0), t.text);
        // Right side.
        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(pos2(rect.max.x - 320.0, rect.min.y), pos2(rect.max.x - 10.0, rect.max.y)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        right.spacing_mut().item_spacing.x = 6.0;
        theme::band_ui(&mut right);
        if icon_toggle(&mut right, Icon::Search, tl!("Search commands (⇧⌘P)"), true, false).clicked() {
            app.palette = Some((String::new(), 0));
        }
        if icon_toggle(
            &mut right,
            if app.ui.brightness == theme::Brightness::Dark { Icon::Sparkle } else { Icon::Palette },
            tl!("Light/Dark appearance"),
            true,
            false,
        )
        .clicked()
        {
            let _ = app.run("view.dark", json!({}));
        }
    });
}

/// The centred title: the active presentation's name and whether it is saved. A new presentation on
/// the web lives as its draft in this browser until it is saved as a file or to Google Drive.
pub(crate) fn document_title(app: &SlideApp) -> String {
    let Some(d) = app.session.active() else { return tl!("PhutKrub").into() };
    match app.services.browser_recovery.as_ref().and_then(|r| (r.open_draft)()) {
        Some(saved) => format!("{} ({}) — {}", d.title(), tl!("draft"), if saved { tl!("Saved on this device") } else { tl!("Saving…") }),
        None => format!("{}{}", d.title(), if d.is_dirty() { " — Edited" } else { "" }),
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
}
