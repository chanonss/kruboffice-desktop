use super::controls::control_keytip;
use super::{Icon, Sense, SheetApp, Tokens, Ui, icon_button, json, pos2, theme, theme_choices, vec2};

/// macOS draws the window into the title bar (`with_fullsize_content_view`) and AppKit paints the
/// traffic lights itself, 16 pt from the top of the window on current macOS. Moving them would take
/// AppKit calls (`unsafe`, forbidden here), so on macOS the title bar is the band they are centred
/// in, with the controls beside them (#170). The height is the KrubOffice one
/// (`krub_icons::chrome::MAC_TITLE_H`), as in PhutKrub; up to macOS 15 the lights are smaller and
/// sit about 2 pt above this bar's centre.
const MAC: bool = cfg!(target_os = "macos");

pub const MAC_TITLE_BAR: f32 = krub_icons::chrome::MAC_TITLE_H;

/// The traffic lights as AppKit lays them out on current macOS: button diameter, centre-to-centre
/// pitch and the first button's left edge (PhutKrub's measurements).
const MAC_LIGHT: f32 = 14.0;

const MAC_LIGHT_PITCH: f32 = 23.0;

const MAC_LIGHTS_LEFT: f32 = 9.0;

/// Where title-bar content starts on macOS: the end of the three lights plus the gap the lights
/// keep from the top of the window.
pub const MAC_CONTENT_LEFT: f32 = MAC_LIGHTS_LEFT + 2.0 * MAC_LIGHT_PITCH + MAC_LIGHT + (MAC_TITLE_BAR - MAC_LIGHT) / 2.0;

/// Leave room for the macOS traffic lights when the content extends into the title bar.
pub const TITLE_LEFT_PAD: f32 = if MAC { MAC_CONTENT_LEFT } else { 8.0 };

pub(super) const TITLE_BAR_H: f32 = if MAC { MAC_TITLE_BAR } else { krub_icons::chrome::TITLE_H };

/// The tallest title-bar controls, smaller on macOS to keep a margin in the shorter bar.
pub(super) const TITLE_CONTROL_H: f32 = if MAC { 22.0 } else { 26.0 };

/// The Quick Access Toolbar (`krub_icons::qat`): Save · Undo ▾ · Redo · ⌄. At the one bar's right
/// end since 2026-11-18.
fn quick_access(app: &mut SheetApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx()).on_band();
    let ch = TITLE_CONTROL_H;
    if icon_button(ui, Icon::Save, t.text_dim, "Save", vec2(26.0, ch)).clicked()
        // File operations must include the cell the user is still editing (as the File > Save menu item already does).
        && app.commit_edit(0, 0, false, false)
    {
        app.run_or_alert("file.save", json!({}));
    }
    // The commands' own availability: there must be a step, and no undo group may be open on
    // the document (an action that spans several calls is still running).
    let can_undo = gridcraft_engine::cmd::can_undo(&app.session).is_ok();
    let can_redo = gridcraft_engine::cmd::can_redo(&app.session).is_ok();
    // Named and marked enabled or not for assistive technology (and the UI tests).
    let undo = icon_button(ui, Icon::Undo, if can_undo { t.text_dim } else { t.text_disabled }, "Undo", vec2(26.0, ch));
    undo.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, can_undo, "Undo"));
    if undo.clicked() && can_undo {
        app.run_or_alert("edit.undo", json!({}));
    }
    let lang = app.ui.language;
    let undo_list = krub_icons::qat::history_arrow(ui, egui::Id::unique("qat_undo_list"), can_undo, t.text_dim, t.text_disabled, t.hover);
    undo_list.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, can_undo, lang.tr("Undo list")));
    egui::Popup::menu(&undo_list.on_hover_text(lang.tr("Undo list"))).show(|ui| {
        let labels: Vec<String> = if can_undo {
            app.session.active().map(|d| d.undo.iter().rev().take(20).map(|e| e.label.clone()).collect()).unwrap_or_default()
        } else {
            Vec::new()
        };
        if labels.is_empty() {
            ui.label(lang.tr("Can't Undo"));
        }
        for (i, l) in labels.iter().enumerate() {
            if ui.button(format!("{} {}", lang.tr("Undo"), lang.tr(l))).clicked() {
                app.run_or_alert("edit.undo", json!({"steps": i + 1}));
            }
        }
    });
    let redo = icon_button(ui, Icon::Redo, if can_redo { t.text_dim } else { t.text_disabled }, "Redo", vec2(26.0, ch));
    redo.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, can_redo, "Redo"));
    if redo.clicked() && can_redo {
        app.run_or_alert("edit.redo", json!({}));
    }
    let more = krub_icons::qat::customize_button(ui, egui::Id::unique("qat_more"), t.text_dim, t.hover);
    more.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, lang.tr("More commands")));
    egui::Popup::menu(&more.on_hover_text(lang.tr("More commands"))).show(|ui| {
        for (label, id) in [
            ("New Workbook", "file.new"),
            ("Open…", "file.open"),
            ("Save As…", "file.saveAs"),
            ("Sort A to Z", "data.sortAscending"),
            ("Calculate Now", "formulas.calculateNow"),
        ] {
            if ui.button(lang.tr(label)).clicked() {
                app.run_or_alert(id, json!({}));
            }
        }
        if ui.button(lang.tr("Print…")).clicked() {
            app.print_now(json!({}));
        }
    });
}

/// The Quick Access Toolbar on the bar's left, after File (owner, 2026-11-18).
pub(super) fn bar_quick_access(app: &mut SheetApp, ui: &mut Ui) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = vec2(krub_icons::qat::GAP, 0.0);
        quick_access(app, ui);
    });
}

/// The Quick Access Toolbar's width: Save, Undo, its list arrow, Redo and ⌄.
pub(super) const QUICK_ACCESS_W: f32 = 3.0 * 26.0 + 2.0 * krub_icons::qat::ARROW.x + 4.0 * krub_icons::qat::GAP;

/// The user's initials for the user icon: the first letters of the first two words of the name
/// comments are signed with.
fn initials(name: &str) -> String {
    let s: String = name.split_whitespace().take(2).filter_map(|w| w.chars().next()).flat_map(char::to_uppercase).collect();
    if s.is_empty() { "?".into() } else { s }
}

/// The right end of the one bar (owner, 2026-11-18), right to left: room for the window buttons
/// (a frameless window) and the user icon with its menu. Returns its left edge.
pub(super) fn bar_right(app: &mut SheetApp, ui: &mut Ui) -> f32 {
    let t = Tokens::get(ui.ctx()).on_band();
    let lang = app.ui.language;
    if app.window_controls {
        ui.add_space(3.0 * CONTROL_W);
    }
    ui.add_space(4.0);
    let resp = krub_icons::chrome::avatar(ui, &initials(&app.session.prefs.user_name), t.accent, t.on_accent, lang.tr("Account"));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, lang.tr("Account")));
    control_keytip(app, ui, &resp, "commandSearch");
    egui::Popup::menu(&resp).show(|ui| {
        ui.set_min_width(240.0);
        user_menu(app, ui);
    });
    ui.min_rect().left()
}

/// The user icon's menu (owner, 2026-11-18): the name comments are signed with (opens Options),
/// Search commands, Comments, the light or dark theme and AutoSave. Share left the bar with it.
fn user_menu(app: &mut SheetApp, ui: &mut Ui) {
    let lang = app.ui.language;
    let name = app.session.prefs.user_name.clone();
    if ui
        .button(egui::RichText::new(if name.is_empty() { lang.tr("Your name").to_string() } else { name }).strong())
        .on_hover_text(lang.tr("Set your name in File › Options"))
        .clicked()
    {
        app.open_backstage(crate::backstage::Page::Options);
        ui.close();
    }
    ui.separator();
    if ui.button(lang.tr("Search commands and help")).clicked() {
        app.open_dialog("commandSearch", json!({}));
        ui.close();
    }
    if ui.button(lang.tr("Comments")).clicked() {
        app.open_dialog("comments", json!({}));
        ui.close();
    }
    ui.separator();
    ui.label(egui::RichText::new(lang.tr("Theme")).small().weak());
    theme_choices(app, ui);
    if !cfg!(target_arch = "wasm32") {
        ui.separator();
        let mut on = app.ui.autosave;
        if ui.checkbox(&mut on, lang.tr("AutoSave")).on_hover_text(lang.tr("AutoSave: save after every change (workbooks saved to a file)")).changed()
        {
            app.toggle_autosave();
        }
    }
}

/// The one bar where the ribbon is hidden (File): the same band and height as the ribbon's bar,
/// the traffic lights' room on macOS, the file pill and the right end, no tabs.
pub fn title_bar(app: &mut SheetApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx()).on_band();
    egui::Panel::top("title_bar").exact_size(krub_icons::chrome::ONE_BAR_H).frame(egui::Frame::NONE.fill(t.band)).show_separator_line(false).show(
        ui,
        |ui| {
            theme::band_ui(ui);
            let rect = ui.max_rect();
            window_drag(ui, rect);
            let row = egui::Rect::from_min_max(
                pos2(rect.left() + TITLE_LEFT_PAD, rect.top() + krub_icons::chrome::ONE_BAR_GAP),
                pos2(rect.right() - 8.0, rect.bottom()),
            );
            let right = ui
                .scope_builder(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::right_to_left(egui::Align::Center)), |ui| bar_right(app, ui))
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

/// Dragging the bar's empty parts moves the window; a double-click maximises it. Registered
/// before the bar's buttons, which take their own clicks.
pub(super) fn window_drag(ui: &mut Ui, bar: egui::Rect) {
    let bg = ui.interact(bar, ui.scope_id().with("drag_title"), Sense::click_and_drag());
    if bg.drag_started() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if bg.double_clicked() {
        let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
    }
}

/// The file pill (`krub_icons::chrome::file_pill`) centred in `area`: the workbook's name. A click
/// opens its card: the whole name, its state, where it is kept and Save As (owner, 2026-11-18).
pub(super) fn file_pill(app: &mut SheetApp, ui: &mut Ui, area: egui::Rect) {
    let t = Tokens::get(ui.ctx()).on_band();
    let lang = app.ui.language;
    let Some(d) = app.session.active() else { return };
    let name = d.display_title();
    let draft = app.services.browser_recovery.as_ref().and_then(|r| (r.open_draft)());
    let state = match draft {
        Some(true) => lang.tr("Saved on this device"),
        Some(false) => lang.tr("Saving…"),
        None if d.is_dirty() => lang.tr("Edited"),
        None if d.path.is_some() => lang.tr("Saved"),
        None => lang.tr("Not saved"),
    }
    .to_string();
    let place = match (&d.path, draft) {
        (Some(path), _) => Some(std::path::Path::new(path).parent().map_or_else(|| path.clone(), |p| p.display().to_string())),
        (None, Some(_)) => Some(lang.tr("Draft in this browser").to_string()),
        (None, None) => None,
    };
    let Some(resp) = krub_icons::chrome::file_pill(ui, area, &name, t.accent, t.text, None) else { return };
    let label = if draft.is_some() { format!("{name} ({}) — {state}", lang.tr("draft")) } else { format!("{name} — {state}") };
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
    egui::Popup::menu(&resp).show(|ui| {
        let card = krub_icons::chrome::FileCard { name: &name, state: &state, place: place.as_deref(), action: Some(lang.tr("Save As…")) };
        if krub_icons::chrome::file_card(ui, &card) {
            app.open_backstage(crate::backstage::Page::SaveAs);
            ui.close();
        }
    });
}

/// Width of one window-control button on the title bar.
pub const CONTROL_W: f32 = 46.0;
const CONTROL_TITLE_H: f32 = 38.0;
const RESIZE_BAND: f32 = 5.0;

/// Draw application controls and edge resizing for frameless windows.
pub fn window_frame(app: &mut SheetApp, ctx: &egui::Context) {
    if !app.window_controls {
        return;
    }
    let (maximized, fullscreen) = ctx.input(|i| (i.viewport().maximized.unwrap_or(false), i.viewport().fullscreen.unwrap_or(false)));
    if fullscreen {
        return;
    }
    let t = Tokens::get(ctx);
    let screen = ctx.content_rect();
    let controls = egui::Rect::from_min_size(egui::pos2(screen.max.x - 3.0 * CONTROL_W, screen.min.y), egui::vec2(3.0 * CONTROL_W, CONTROL_TITLE_H));
    egui::Area::new(egui::Id::unique("gc_window_controls")).order(egui::Order::Foreground).fixed_pos(controls.min).show(ctx, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        ui.horizontal(|ui| {
            for kind in ["minimize", "maximize", "close"] {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(CONTROL_W, CONTROL_TITLE_H), egui::Sense::click());
                let label = app.ui.language.tr(match kind {
                    "minimize" => "Minimize",
                    "maximize" if maximized => "Restore Down",
                    "maximize" => "Maximize",
                    _ => "Close",
                });
                let hot = resp.hovered() || resp.is_pointer_button_down_on();
                if hot {
                    ui.painter().rect_filled(
                        r,
                        0.0,
                        if kind == "close" {
                            egui::Color32::from_rgb(0xC4, 0x2B, 0x1C)
                        } else if resp.is_pointer_button_down_on() {
                            t.pressed
                        } else {
                            t.hover
                        },
                    );
                }
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
                let m = r.center();
                let stroke = egui::Stroke::new(1.0, if kind == "close" && hot { egui::Color32::WHITE } else { t.text });
                match kind {
                    "minimize" => {
                        ui.painter().line_segment([egui::pos2(m.x - 5.0, m.y), egui::pos2(m.x + 5.0, m.y)], stroke);
                    }
                    "maximize" if maximized => {
                        let front = egui::Rect::from_min_size(egui::pos2(m.x - 5.0, m.y - 3.0), egui::vec2(8.0, 8.0));
                        ui.painter().rect_stroke(front, 0.0, stroke, egui::StrokeKind::Inside);
                        ui.painter().line_segment([egui::pos2(m.x - 3.0, m.y - 5.0), egui::pos2(m.x + 5.0, m.y - 5.0)], stroke);
                        ui.painter().line_segment([egui::pos2(m.x + 5.0, m.y - 5.0), egui::pos2(m.x + 5.0, m.y + 3.0)], stroke);
                    }
                    "maximize" => {
                        ui.painter().rect_stroke(r.shrink(15.0), 0.0, stroke, egui::StrokeKind::Inside);
                    }
                    _ => {
                        ui.painter().line_segment([m - egui::vec2(5.0, 5.0), m + egui::vec2(5.0, 5.0)], stroke);
                        ui.painter().line_segment([m + egui::vec2(5.0, -5.0), m - egui::vec2(5.0, -5.0)], stroke);
                    }
                }
                if resp.on_hover_text(label).clicked() {
                    match kind {
                        "minimize" => app.run_or_alert("ui.window.minimize", json!({})),
                        "maximize" => app.run_or_alert("ui.window.maximize", json!({"on": !maximized})),
                        "close" => app.run_or_alert("ui.window.close", json!({})),
                        _ => {}
                    }
                }
            }
        });
    });
    if maximized {
        return;
    }
    let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let l = pos.x <= screen.min.x + RESIZE_BAND;
    let r = pos.x >= screen.max.x - RESIZE_BAND;
    let top = pos.y <= screen.min.y + RESIZE_BAND;
    let b = pos.y >= screen.max.y - RESIZE_BAND;
    let dir = match (l, r, top, b) {
        (true, false, true, false) => Some(egui::viewport::ResizeDirection::NorthWest),
        (false, true, true, false) => Some(egui::viewport::ResizeDirection::NorthEast),
        (true, false, false, true) => Some(egui::viewport::ResizeDirection::SouthWest),
        (false, true, false, true) => Some(egui::viewport::ResizeDirection::SouthEast),
        (true, false, _, _) => Some(egui::viewport::ResizeDirection::West),
        (false, true, _, _) => Some(egui::viewport::ResizeDirection::East),
        (_, _, true, false) => Some(egui::viewport::ResizeDirection::North),
        (_, _, false, true) => Some(egui::viewport::ResizeDirection::South),
        _ => None,
    };
    if let Some(dir) = dir
        && ctx.input(|i| i.pointer.primary_pressed())
        && !controls.contains(pos)
    {
        let direction = match dir {
            egui::ResizeDirection::North => "north",
            egui::ResizeDirection::South => "south",
            egui::ResizeDirection::East => "east",
            egui::ResizeDirection::West => "west",
            egui::ResizeDirection::NorthEast => "northeast",
            egui::ResizeDirection::NorthWest => "northwest",
            egui::ResizeDirection::SouthEast => "southeast",
            egui::ResizeDirection::SouthWest => "southwest",
        };
        app.run_or_alert("ui.window.beginResize", json!({"direction": direction}));
    }
    app.flush_viewport_commands(ctx);
}

#[cfg(test)]
mod title_bar_tests {
    use super::*;

    /// A new web workbook kept only as its browser draft says so in the title bar, instead of the
    /// unsaved marker, and says whether its latest edit is in the draft yet.
    #[test]
    fn mac_title_bar_centres_and_clears_the_traffic_lights() {
        // The lights are centred in the bar, and content keeps the same gap after them.
        let margin = (MAC_TITLE_BAR - MAC_LIGHT) / 2.0;
        let lights_end = MAC_LIGHTS_LEFT + 2.0 * MAC_LIGHT_PITCH + MAC_LIGHT;
        assert_eq!(MAC_CONTENT_LEFT - lights_end, margin);
    }
}
