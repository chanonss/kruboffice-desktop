//! Web: the places to open from or save to, as cards side by side: this device and Google Drive
//! (the shared `krub-drive` crate does the Drive work). A presentation opened from Drive saves back
//! over its file with Save, Ctrl+S and the toolbar ([`SlideApp::save`]).

use super::{Backstage, Outcome, RichText, Sense, SlideApp, Stroke, Tokens, Ui, dim, json, theme, vec2};

const CARD_W: f32 = 330.0;

/// Drive keeps presentations as PowerPoint files.
const DRIVE_FORMAT: &str = "pptx";

impl SlideApp {
    /// The Google Drive file the active presentation was opened from, when it can be saved back.
    pub fn drive_file_name(&self) -> Option<String> {
        let drive = self.services.google_drive.as_ref()?;
        (drive.linked_name)(self.session.active()?.uid)
    }

    /// Replace the active presentation's Drive file with it. The upload finishes later; the web
    /// host marks the presentation saved only then, and only if it hasn't changed meanwhile.
    pub fn save_to_drive(&mut self) {
        let Some((uid, revision, doc)) = self.session.active().map(|d| (d.uid, d.revision, d.doc.clone())) else { return };
        match deckcraft_engine::cmd::file::save_bytes(&doc, DRIVE_FORMAT) {
            Ok(bytes) => {
                if let Some(drive) = &self.services.google_drive {
                    (drive.save_over)(uid, revision, bytes);
                }
            }
            Err(e) => self.set_status(e.to_string()),
        }
    }

    /// Upload the active presentation to Drive as a new file; the presentation stays as it is.
    fn save_copy_to_drive(&mut self) {
        let Some((title, doc)) = self.session.active().map(|d| (d.title(), d.doc.clone())) else { return };
        match deckcraft_engine::cmd::file::save_bytes(&doc, DRIVE_FORMAT) {
            Ok(bytes) => {
                if let Some(drive) = &self.services.google_drive {
                    (drive.save_copy)(format!("{title}.{DRIVE_FORMAT}"), bytes);
                }
            }
            Err(e) => self.set_status(e.to_string()),
        }
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
    egui::Frame::new().stroke(Stroke::new(1.0, t.border)).corner_radius(10.0).inner_margin(egui::Margin::same(16)).show(ui, |ui| {
        ui.set_width(CARD_W);
        ui.set_min_height(160.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            let (tile, _) = ui.allocate_exact_size(vec2(38.0, 38.0), Sense::hover());
            ui.painter().rect_filled(tile, 8.0, t.accent.linear_multiply(0.22));
            krub_icons::paint_named(ui.painter(), tile.shrink(8.0), icon, t.text, false, false);
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).font(theme::bold(16.0)).color(t.text));
                if let Some(chip) = chip {
                    egui::Frame::new()
                        .fill(t.accent.linear_multiply(0.22))
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::symmetric(6, 1))
                        .show(ui, |ui| ui.label(RichText::new(chip).font(theme::font(12.0)).color(t.text)));
                }
            });
            body(ui);
        });
    });
}

/// Open (web): this device and Google Drive.
pub(super) fn open_places(app: &mut SlideApp, ui: &mut Ui) {
    places(ui, |ui| {
        card(ui, "device", tl!("This device"), None, |ui| {
            dim(
                ui,
                tl!(if cfg!(target_arch = "wasm32") {
                    "Choose a file stored on this computer or iPad."
                } else {
                    "Choose a file stored on this computer."
                }),
            );
            if ui
                .button(
                    RichText::new(tl!(if cfg!(target_arch = "wasm32") { "Browse files on this device…" } else { "Browse…" })).font(theme::font(14.0)),
                )
                .clicked()
            {
                let _ = app.run("app.openDialog", json!({}));
            }
        });
        drive_card(app, ui, None);
    });
}

/// Save As (web): this device (`device` draws its type picker and Download) and Google Drive. A
/// presentation opened from Drive shows Drive first.
pub(super) fn save_places(
    app: &mut SlideApp,
    ui: &mut Ui,
    bs: &mut Backstage,
    out: &mut Outcome,
    device: impl FnOnce(&mut SlideApp, &mut Ui, &mut Backstage, &mut Outcome),
) {
    let from_drive = app.drive_file_name().is_some();
    places(ui, |ui| {
        if from_drive {
            drive_card(app, ui, Some(&mut *out));
        }
        card(ui, "device", tl!("This device"), None, |ui| {
            dim(
                ui,
                tl!(if cfg!(target_arch = "wasm32") {
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
fn drive_card(app: &mut SlideApp, ui: &mut Ui, save: Option<&mut Outcome>) {
    let Some(state) = app.services.google_drive.as_ref().map(|drive| (drive.status)()) else { return };
    let connected = state == "Connected";
    card(ui, "cloud", "Google Drive", connected.then(|| tl!("Connected")), |ui| {
        if !connected {
            dim(
                ui,
                if save.is_some() {
                    tl!("Save to your Google Drive.")
                } else {
                    tl!("Open a PowerPoint presentation from your Google Drive and save it back there.")
                },
            );
            dim(ui, tl!(state));
            let again = state == "Signing in…";
            // The sign-in window may not have opened (a blocked pop-up) or was closed unanswered:
            // start it again rather than leaving the page stuck until a reload.
            let label = if again { tl!("Try signing in again") } else { tl!("Connect Google Drive") };
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
            && ui.button(tl!("Disconnect Google Drive")).clicked()
        {
            disconnect();
        }
    });
}

fn drive_open(app: &mut SlideApp, ui: &mut Ui) {
    let Some(drive) = &app.services.google_drive else { return };
    dim(ui, tl!("Open a PowerPoint presentation from your Google Drive and save it back there."));
    let (document, revision) = app.session.active().map(|d| (d.uid, d.revision)).unwrap_or_default();
    if ui.button(RichText::new(tl!("Choose from Google Drive…")).font(theme::font(14.0))).clicked() {
        (drive.open)(document, revision);
    }
    if let Some(files) = (drive.files)() {
        ui.label(tl!("Files available to PhutKrub (up to 100):"));
        if files.is_empty() {
            ui.label(tl!("No available PowerPoint presentations found."));
        }
        for (id, name) in files {
            if ui.button(name).clicked() {
                (drive.select_file)(id, document, revision);
            }
        }
    }
}

fn drive_save(app: &mut SlideApp, ui: &mut Ui, out: &mut Outcome) {
    let linked = app.drive_file_name();
    match &linked {
        Some(name) => {
            dim(ui, &crate::i18n::fmt(tl!("Opened from Google Drive: {name}"), &[("name", name)]));
            dim(ui, tl!("Replaces the file on Google Drive. Drive keeps earlier versions for a while."));
        }
        None => dim(ui, tl!("Each save creates a new file. Changes remain unsaved in the editor.")),
    }
    let mut over = false;
    let mut copy = false;
    ui.horizontal_wrapped(|ui| {
        if linked.is_some() {
            over = ui.button(RichText::new(tl!("Save to this file on Google Drive")).font(theme::font(14.0))).clicked();
        }
        copy = ui.button(RichText::new(tl!("Save a new copy to Google Drive")).font(theme::font(14.0))).clicked();
    });
    if over {
        app.save_to_drive();
        out.close = true;
    } else if copy {
        app.save_copy_to_drive();
    }
}
