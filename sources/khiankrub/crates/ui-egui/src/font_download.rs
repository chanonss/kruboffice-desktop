//! Get the Missing Fonts: a document opened from a file that asks for fonts KhianKrub knows where
//! to get (the Thai national fonts SIPA gives away, or Microsoft's Thai fonts that come with
//! Windows) is told about once, in a box that lists each font, the font drawn in its place and
//! where to get it. "Don't show this again" turns the box off (File › Options › General turns it
//! back on); the status bar's missing-fonts menu opens it by hand (`ui.dialog fontDownload`).
//!
//! Ported by hand from WordCraft 8331f2e6 (#6826) and drawn as the owner's draft showed it: one
//! table instead of upstream's cards. The list is `font.missing`'s
//! ([`wordcraft_engine::cmd::font::missing_fonts`]), so agents read the same fonts and pages
//! without the box. KhianKrub only links to the publisher's page: it never downloads or bundles
//! the fonts.

use serde::Serialize;

use crate::WordApp;
use crate::dialogs::Dialog;
use crate::theme::{Tokens, semibold};

/// Most documents the box has been considered for that are remembered: beyond that the oldest
/// are forgotten (a document shown again after hundreds of others may be told about twice).
const REMEMBERED: usize = 256;

/// Most fonts the box lists: a hostile file can name thousands.
const LISTED: usize = 12;

/// The table's column widths: the document's font, what it is shown with, where to get it.
const COLUMNS: [f32; 3] = [170.0, 150.0, 190.0];

/// One missing font the box lists.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    /// The font the document asks for.
    pub font: String,
    /// The families its text is drawn in instead.
    pub shown_as: Vec<String>,
    /// Who publishes the font.
    pub publisher: String,
    /// The publisher's download page; `None` when it comes only with Windows.
    pub url: Option<String>,
    /// The publisher limits who may use the font: the box says so.
    pub restricted: bool,
}

/// The shown document's missing fonts the list knows where to get.
pub(crate) fn rows(app: &mut WordApp) -> Vec<Row> {
    let layout = app.session.layout();
    let missing = app.cached_missing_fonts(&layout).0;
    missing
        .iter()
        .filter_map(|f| {
            let d = f.download?;
            Some(Row {
                font: f.font.clone(),
                shown_as: f.shown_as.clone(),
                publisher: d.publisher.clone(),
                url: d.url.clone(),
                restricted: d.restricted,
            })
        })
        .take(LISTED)
        .collect()
}

/// The box for the shown document (`ui.dialog {"name": "fontDownload"}`), listing nothing when
/// the list knows none of its missing fonts.
pub(crate) fn open(app: &mut WordApp) -> Dialog {
    Dialog::FontDownload { fonts: rows(app), dont_show: !app.ui.font_download_notice }
}

/// The document `id` has been considered for the box, and is not considered again.
fn remember(app: &mut WordApp, id: u64) {
    if app.font_notice_seen.len() >= REMEMBERED {
        app.font_notice_seen.remove(0);
    }
    app.font_notice_seen.push(id);
}

/// A document a script opened is not told about: programmatic calls never open dialogs.
pub(crate) fn skip(app: &mut WordApp, id: u64) {
    if !app.font_notice_seen.contains(&id) {
        remember(app, id);
    }
}

impl WordApp {
    /// Bring the box up by itself when a document opened from a file is missing fonts the list
    /// knows. The desktop and web apps call this once at start-up; unit tests and `ui_shot`
    /// don't, so what they show doesn't depend on the fonts installed where they run.
    pub fn tell_about_font_downloads(&mut self) {
        self.font_notices = true;
    }
}

/// Show the box once for a document opened from a file, when it is missing fonts the list knows
/// and the box hasn't been turned off. Called every frame; it waits for an open dialog or the
/// File tab to close rather than replace them.
pub(crate) fn tick(app: &mut WordApp) {
    let id = app.session.document_id();
    if !app.font_notices || app.font_notice_seen.contains(&id) || app.dialog.is_some() || app.ui.backstage {
        return;
    }
    remember(app, id);
    // A new document has no file: its fonts are KhianKrub's choice, not a document's from elsewhere.
    if !app.ui.font_download_notice || (app.session.path.is_none() && app.session.file_name.is_none()) {
        return;
    }
    let fonts = rows(app);
    if !fonts.is_empty() {
        app.dialog = Some(Dialog::FontDownload { fonts, dont_show: false });
    }
}

/// How to install a downloaded font on this system, after the files are unpacked.
fn system_install_steps() -> &'static str {
    if cfg!(target_arch = "wasm32") {
        ""
    } else if cfg!(target_os = "windows") {
        tl!("Or install it for every app: select the font files, right-click and choose Install, then restart KhianKrub.")
    } else if cfg!(target_os = "macos") {
        tl!("Or install it for every app: double-click each font file and choose Install in Font Book, then restart KhianKrub.")
    } else {
        tl!("Or install it for every app: copy the font files to the .local/share/fonts folder in your home folder, then restart KhianKrub.")
    }
}

/// One table cell of `width`, its text wrapped.
fn cell(ui: &mut egui::Ui, width: f32, text: egui::RichText) {
    ui.allocate_ui_with_layout(egui::vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.set_width(width);
        ui.add(egui::Label::new(text).wrap());
    });
}

/// The box's content: a headline, a table of the fonts (the document's font, what it is shown
/// with, where to get it), how to add a download, then the opt-out and Close. Returns true to
/// close.
pub(crate) fn body(app: &mut WordApp, ui: &mut egui::Ui, fonts: &[Row], dont_show: &mut bool) -> bool {
    let t = Tokens::get(ui.ctx());
    ui.set_width(COLUMNS.iter().sum::<f32>() + 24.0);
    let mut open_url = None;
    let mut add = false;
    if fonts.is_empty() {
        ui.label(tl!("KhianKrub doesn't know where to get the fonts this document is missing."));
    } else {
        let room = (ui.ctx().content_rect().height() - 260.0).max(80.0);
        ui.label(egui::RichText::new(tl!("This document uses fonts that aren't installed")).font(semibold(15.0)));
        ui.label(egui::RichText::new(tl!("Their text is shown in another font, so it may not look as it was written.")).color(t.text_dim));
        ui.add_space(8.0);
        egui::Frame::NONE.fill(t.panel).stroke(egui::Stroke::new(1.0, t.border)).corner_radius(8).inner_margin(egui::Margin::symmetric(10, 8)).show(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    for (w, h) in COLUMNS.into_iter().zip(["Font in the document", "Shown with", "Get it from"]) {
                        cell(ui, w, egui::RichText::new(tl!(h)).small().color(t.text_dim));
                    }
                });
                egui::ScrollArea::vertical().id_salt("font_downloads").max_height(room).show(ui, |ui| {
                    for f in fonts {
                        ui.horizontal(|ui| {
                            // The document's own names, not interface text.
                            cell(ui, COLUMNS[0], egui::RichText::new(f.font.as_str()).strong());
                            cell(ui, COLUMNS[1], egui::RichText::new(f.shown_as.join(", ")));
                            ui.allocate_ui_with_layout(egui::vec2(COLUMNS[2], 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                                ui.set_width(COLUMNS[2]);
                                match &f.url {
                                    Some(url) => {
                                        let label = crate::i18n::fmt(tl!("Download from {publisher}"), &[("publisher", &f.publisher)]);
                                        if ui.link(label).on_hover_text(url.as_str()).clicked() {
                                            open_url = Some(url.clone());
                                        }
                                    }
                                    None => {
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(tl!("Comes with Windows; not offered on its own")).color(t.text_dim),
                                            )
                                            .wrap(),
                                        );
                                    }
                                }
                                if f.restricted {
                                    ui.label(
                                        egui::RichText::new(tl!("Its licence limits who may use it: read the terms first."))
                                            .small()
                                            .color(t.text_dim),
                                    );
                                }
                            });
                        });
                    }
                });
            },
        );
        ui.add_space(6.0);
        // Whose part the licence is: KhianKrub only points at the publisher's page.
        let terms = tl!("Fonts come with licence terms. Read the publisher's terms on the download page: following them is up to you.");
        ui.label(egui::RichText::new(terms).small().color(t.text_dim));
        if fonts.iter().any(|f| f.url.is_some()) {
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(tl!("Download the font, unpack the .zip file, then pick the font files:"));
                add = ui.button(tl!("Add Font…")).clicked();
            });
            let system = system_install_steps();
            if !system.is_empty() {
                ui.label(egui::RichText::new(system).small().color(t.text_dim));
            }
        }
    }
    if open_url.is_some() {
        // The publisher's page, in the browser: KhianKrub downloads nothing itself.
        app.canvas.open_url = open_url;
    }
    ui.add_space(10.0);
    let mut close = ui.input(|i| i.key_pressed(egui::Key::Escape));
    ui.horizontal(|ui| {
        if !fonts.is_empty() && ui.checkbox(dont_show, tl!("Don't show this again")).changed() {
            // Kept at once, however the box is closed.
            app.ui.font_download_notice = !*dont_show;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            close |= ui.add(crate::widgets::primary_button(ui, tl!("Close"))).clicked();
        });
    });
    if add {
        // The file picker replaces the box; the added font lays the document out again.
        let _ = app.run("font.add", serde_json::json!({}));
        return true;
    }
    close
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;
    use serde_json::json;
    use wordcraft_engine::Session;

    fn row(font: &str, url: Option<&str>) -> Row {
        Row { font: font.into(), shown_as: vec!["Tahoma".into()], publisher: "SIPA".into(), url: url.map(Into::into), restricted: false }
    }

    /// The interface fonts take effect from the frame after they're installed: the first frame
    /// installs them and draws nothing.
    fn fonts_ready(ui: &egui::Ui) -> bool {
        let id = egui::Id::new("font_download_test_fonts");
        let ready = ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        if !ready {
            crate::theme::install_fonts(ui.ctx());
            ui.ctx().data_mut(|d| d.insert_temp(id, true));
        }
        ready
    }

    fn app() -> WordApp {
        WordApp::new(Session::new(wordcraft_doc::Document::from_text("ข้อความ")), crate::Services::default())
    }

    /// The box lists each font with where to get it; a download link opens the publisher's page,
    /// and a Windows font says it comes with Windows.
    #[test]
    fn the_box_links_to_the_publishers_page() {
        let url = "https://www.f0nt.com/release/13-free-fonts-from-sipa/";
        let fonts = vec![row("TH Niramit AS", Some(url)), row("Angsana New", None)];
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(800.0, 700.0)).build_ui_state(
            |ui, (app, fonts, dont): &mut (WordApp, Vec<Row>, bool)| {
                if fonts_ready(ui) {
                    body(app, ui, fonts, dont);
                }
            },
            (app(), fonts, false),
        );
        h.run_steps(3);
        h.get_by_label("TH Niramit AS");
        h.get_by_label("Comes with Windows; not offered on its own");
        h.get_by_label("Download from SIPA").click();
        h.run_steps(2);
        assert_eq!(h.state().0.canvas.open_url.as_deref(), Some(url));
    }

    /// "Don't show this again" turns the box off at once.
    #[test]
    fn dont_show_turns_the_box_off() {
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(800.0, 700.0)).build_ui_state(
            |ui, (app, fonts, dont): &mut (WordApp, Vec<Row>, bool)| {
                if fonts_ready(ui) {
                    body(app, ui, fonts, dont);
                }
            },
            (app(), vec![row("TH Niramit AS", Some("https://www.f0nt.com/release/13-free-fonts-from-sipa/"))], false),
        );
        h.run_steps(3);
        assert!(h.state().0.ui.font_download_notice);
        h.get_by_label("Don't show this again").click();
        h.run_steps(2);
        assert!(!h.state().0.ui.font_download_notice);
    }

    /// The box comes up by itself only when the app asked for it, once per document, never for a
    /// new document, and never for one a script opened.
    #[test]
    fn the_box_comes_up_once_for_a_document_from_a_file() {
        let mut a = app();
        tick(&mut a);
        assert!(a.dialog.is_none(), "not asked to tell");
        a.tell_about_font_downloads();
        let id = a.session.document_id();
        tick(&mut a);
        assert!(a.dialog.is_none(), "a new document has no file");
        assert!(a.font_notice_seen.contains(&id), "and is not considered again");
        skip(&mut a, 42);
        assert!(a.font_notice_seen.contains(&42));
        // The list is bounded.
        for i in 0..(REMEMBERED as u64 + 10) {
            remember(&mut a, 1000 + i);
        }
        assert_eq!(a.font_notice_seen.len(), REMEMBERED);
        // ui.dialog opens it by hand; every font it lists has somewhere to get it.
        a.run("ui.dialog", json!({"name": "fontDownload"})).unwrap();
        let Some(Dialog::FontDownload { fonts, .. }) = &a.dialog else { panic!("the box is open") };
        assert!(fonts.iter().all(|f| f.url.is_some() || f.publisher == "Microsoft"), "{fonts:?}");
    }
}
