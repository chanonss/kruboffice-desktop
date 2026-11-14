//! Web: the places to open from or save to, as cards side by side: this device and Google Drive
//! (the shared `krub-drive` crate does the Drive work). A workbook opened from Drive saves back over
//! its file with Save, Ctrl+S and the title bar's Save (`file.save`, [`SheetApp::save_to_drive`]).

use egui::{RichText, Sense, Stroke, Ui, vec2};
use serde_json::json;

use super::{Backstage, Outcome, dim};
use crate::SheetApp;
use crate::theme::{self, Tokens};

const CARD_W: f32 = 330.0;

impl SheetApp {
    /// The Google Drive file the active workbook was opened from, when it can be saved back.
    pub fn drive_file_name(&self) -> Option<String> {
        let drive = self.services.google_drive.as_ref()?;
        (drive.linked_name)(self.session.active()?.uid)
    }

    /// Replace the active workbook's Drive file with it. The upload finishes later; the web host
    /// marks the workbook saved only then, and only if it hasn't changed meanwhile.
    pub fn save_to_drive(&mut self) -> Result<serde_json::Value, String> {
        let (uid, revision) = self.session.active().map(|d| (d.uid, d.revision)).ok_or("No workbook is open")?;
        let bytes = self.xlsx_bytes()?;
        if let Some(drive) = &self.services.google_drive {
            (drive.save_over)(uid, revision, bytes);
        }
        Ok(json!({"googleDrive": self.drive_file_name()}))
    }

    /// Upload the active workbook to Drive as a new file; the workbook stays as it is.
    fn save_copy_to_drive(&mut self) -> Result<(), String> {
        let title = self.session.active().map(|d| d.display_title()).ok_or("No workbook is open")?;
        let bytes = self.xlsx_bytes()?;
        let name = if title.to_ascii_lowercase().ends_with(".xlsx") { title } else { format!("{title}.xlsx") };
        if let Some(drive) = &self.services.google_drive {
            (drive.save_copy)(name, bytes);
        }
        Ok(())
    }

    fn xlsx_bytes(&mut self) -> Result<Vec<u8>, String> {
        let r = self.run("file.saveBytes", json!({"format": "xlsx"}))?;
        r.get("base64")
            .and_then(serde_json::Value::as_str)
            .and_then(gridcraft_engine::io::base64_decode)
            .ok_or_else(|| "Could not encode the workbook".into())
    }
}

/// The places, side by side, wrapping to a column when narrow.
fn places(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
        add(ui);
    });
}

/// A place: an icon on a tinted tile, a title with an optional status chip, then `body`.
fn card(ui: &mut Ui, icon: &str, title: &str, chip: Option<&str>, body: impl FnOnce(&mut Ui)) {
    let t = Tokens::get(ui.ctx());
    egui::Frame::new().stroke(Stroke::new(1.0, t.input_border)).corner_radius(10.0).inner_margin(egui::Margin::same(16)).show(ui, |ui| {
        ui.set_width(CARD_W);
        ui.set_min_height(160.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            let (tile, _) = ui.allocate_exact_size(vec2(38.0, 38.0), Sense::hover());
            ui.painter().rect_filled(tile, 8.0, t.accent.linear_multiply(0.22));
            krub_icons::paint_named(ui.painter(), tile.shrink(8.0), icon, t.text, false, false);
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).font(theme::ui_bold(16.0)).color(t.text));
                if let Some(chip) = chip {
                    egui::Frame::new()
                        .fill(t.accent.linear_multiply(0.22))
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::symmetric(6, 1))
                        .show(ui, |ui| ui.label(RichText::new(chip).font(theme::ui_font(12.0)).color(t.text)));
                }
            });
            body(ui);
        });
    });
}

/// Open (web): this device and Google Drive.
pub(super) fn open_places(app: &mut SheetApp, ui: &mut Ui) {
    let lang = app.ui.language;
    places(ui, |ui| {
        card(ui, "device", lang.tr("This device"), None, |ui| {
            dim(
                ui,
                lang.tr(if cfg!(target_arch = "wasm32") {
                    "Choose a file stored on this computer or iPad."
                } else {
                    "Choose a file stored on this computer."
                }),
            );
            if ui
                .button(
                    RichText::new(lang.tr(if cfg!(target_arch = "wasm32") { "Browse files on this device…" } else { "Browse…" }))
                        .font(theme::ui_font(14.0)),
                )
                .clicked()
            {
                app.open_dialog("open", json!({}));
            }
        });
        drive_card(app, ui, None);
    });
}

/// Save As (web): this device (`device` draws its type picker and Download) and Google Drive. A
/// workbook opened from Drive shows Drive first.
pub(super) fn save_places(
    app: &mut SheetApp,
    ui: &mut Ui,
    bs: &mut Backstage,
    out: &mut Outcome,
    device: impl FnOnce(&mut SheetApp, &mut Ui, &mut Backstage, &mut Outcome),
) {
    let lang = app.ui.language;
    let from_drive = app.drive_file_name().is_some();
    places(ui, |ui| {
        if from_drive {
            drive_card(app, ui, Some(&mut *out));
        }
        card(ui, "device", lang.tr("This device"), None, |ui| {
            dim(
                ui,
                lang.tr(if cfg!(target_arch = "wasm32") {
                    "Download a copy to this computer or iPad."
                } else {
                    "Choose where to save on this computer."
                }),
            );
            device(app, ui, bs, out);
        });
        if !from_drive {
            drive_card(app, ui, Some(&mut *out));
        }
    });
}

/// The Google Drive card on Open (`save` None) or Save As.
fn drive_card(app: &mut SheetApp, ui: &mut Ui, save: Option<&mut Outcome>) {
    let lang = app.ui.language;
    let Some(state) = app.services.google_drive.as_ref().map(|drive| (drive.status)()) else { return };
    let connected = state == "Connected";
    card(ui, "cloud", "Google Drive", connected.then(|| lang.tr("Connected")), |ui| {
        if !connected {
            dim(
                ui,
                lang.tr(if save.is_some() {
                    "Save to your Google Drive."
                } else {
                    "Open an Excel workbook from your Google Drive and save it back there."
                }),
            );
            dim(ui, lang.tr(state));
            let again = state == "Signing in…";
            // The sign-in window may not have opened (a blocked pop-up) or was closed unanswered:
            // start it again rather than leaving the page stuck until a reload.
            let label = lang.tr(if again { "Try signing in again" } else { "Connect Google Drive" });
            if (again || matches!(state, "Not connected" | "Click Connect to sign in"))
                && ui.button(label).clicked()
                && let Some(drive) = &app.services.google_drive
            {
                (drive.connect)();
            }
            return;
        }
        match save {
            Some(out) => drive_save(app, ui, out),
            None => drive_open(app, ui),
        }
        if let Some(drive) = &app.services.google_drive
            && let Some(disconnect) = &drive.disconnect
            && ui.button(lang.tr("Disconnect Google Drive")).clicked()
        {
            disconnect();
        }
    });
}

fn drive_open(app: &mut SheetApp, ui: &mut Ui) {
    let lang = app.ui.language;
    let Some(drive) = &app.services.google_drive else { return };
    dim(ui, lang.tr("Open an Excel workbook from your Google Drive and save it back there."));
    let (document, revision) = app.session.active().map(|d| (d.uid, d.revision)).unwrap_or_default();
    if ui.button(RichText::new(lang.tr("Choose from Google Drive…")).font(theme::ui_font(14.0))).clicked() {
        (drive.open)(document, revision);
    }
    if let Some(files) = (drive.files)() {
        ui.label(lang.tr("Files available to KhitKrub (up to 100):"));
        if files.is_empty() {
            ui.label(lang.tr("No available Excel workbooks found."));
        }
        for (id, name) in files {
            if ui.button(name).clicked() {
                (drive.select_file)(id, document, revision);
            }
        }
    }
}

fn drive_save(app: &mut SheetApp, ui: &mut Ui, out: &mut Outcome) {
    let lang = app.ui.language;
    let linked = app.drive_file_name();
    match &linked {
        Some(name) => {
            dim(ui, &lang.tr("Opened from Google Drive: {name}").replace("{name}", name));
            dim(ui, lang.tr("Replaces the file on Google Drive. Drive keeps earlier versions for a while."));
        }
        None => dim(ui, lang.tr("Each save creates a new file. Changes remain unsaved in the editor.")),
    }
    let mut over = false;
    let mut copy = false;
    ui.horizontal_wrapped(|ui| {
        if linked.is_some() {
            over = ui.button(RichText::new(lang.tr("Save to this file on Google Drive")).font(theme::ui_font(14.0))).clicked();
        }
        copy = ui.button(RichText::new(lang.tr("Save a new copy to Google Drive")).font(theme::ui_font(14.0))).clicked();
    });
    // A cell still being edited is part of the workbook.
    if (over || copy) && !app.commit_edit(0, 0, false, false) {
        return;
    }
    let result = if over {
        out.close = true;
        app.save_to_drive().map(|_| ())
    } else if copy {
        app.save_copy_to_drive()
    } else {
        Ok(())
    };
    if let Err(e) = result {
        app.message = Some(("KhitKrub".into(), e));
    }
}
