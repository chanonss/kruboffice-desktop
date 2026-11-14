//! Google Drive on the web's File pages: the place cards on Open and Save As, explicit clicks only,
//! and Save going back to the Drive file a presentation came from.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use deckcraft_engine::Session;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::json;

use crate::backstage::Page;
use crate::{DriveServices, Services, SlideApp};

type Calls<T> = Rc<RefCell<Vec<T>>>;

/// Drive services that record what they were asked; `linked` names the presentation (by uid) that
/// came from Drive.
fn drive(
    status: &'static str,
    linked: Rc<Cell<Option<u64>>>,
) -> (DriveServices, Calls<(u64, u64)>, Calls<(String, Vec<u8>)>, Calls<(u64, u64, Vec<u8>)>) {
    let (opened, copies, replaced): (Calls<(u64, u64)>, Calls<(String, Vec<u8>)>, Calls<(u64, u64, Vec<u8>)>) = Default::default();
    let (o, c, r) = (opened.clone(), copies.clone(), replaced.clone());
    let services = DriveServices {
        status: Box::new(move || status),
        connect: Box::new(|| {}),
        disconnect: None,
        open: Box::new(move |doc, rev| o.borrow_mut().push((doc, rev))),
        files: Box::new(|| None),
        select_file: Box::new(|_, _, _| {}),
        save_copy: Box::new(move |name, bytes| c.borrow_mut().push((name, bytes))),
        linked_name: Box::new(move |doc| (linked.get() == Some(doc)).then(|| "แผนงาน.pptx".to_string())),
        save_over: Box::new(move |doc, rev, bytes| r.borrow_mut().push((doc, rev, bytes))),
    };
    (services, opened, copies, replaced)
}

fn harness(app: SlideApp) -> Harness<'static, SlideApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(3);
    h
}

fn web_app(drive: DriveServices) -> SlideApp {
    let services = Services { google_drive: Some(drive), download: Some(Box::new(|_: &str, _: &[u8]| {})), ..Default::default() };
    SlideApp::new(Session::with_new(), services)
}

#[test]
fn open_and_save_a_copy_are_explicit_clicks_on_the_place_cards() {
    let (services, opened, copies, replaced) = drive("Connected", Rc::default());
    let mut app = web_app(services);
    app.open_backstage(Page::Open);
    let mut h = harness(app);
    assert!(h.query_by_label("This device").is_some(), "the device card is beside Drive");
    assert!(opened.borrow().is_empty() && copies.borrow().is_empty(), "nothing is sent before a click");
    h.get_by_label("Choose from Google Drive…").click();
    h.run_steps(3);
    let (uid, rev) = h.state().session.active().map(|d| (d.uid, d.revision)).unwrap();
    assert_eq!(opened.borrow().as_slice(), &[(uid, rev)]);
    h.state_mut().open_backstage(Page::SaveAs);
    h.run_steps(3);
    assert!(h.query_by_label("Save to this file on Google Drive").is_none(), "not opened from Drive");
    h.get_by_label("Save a new copy to Google Drive").click();
    h.run_steps(3);
    let copies = copies.borrow();
    assert_eq!(copies.len(), 1);
    assert!(copies[0].0.ends_with(".pptx"), "{}", copies[0].0);
    assert!(copies[0].1.starts_with(b"PK"), "a PowerPoint file");
    assert!(replaced.borrow().is_empty(), "a copy never replaces a Drive file");
}

#[test]
fn a_presentation_from_drive_saves_back_over_its_file_from_save_as() {
    let linked = Rc::new(Cell::new(None));
    let (services, _, copies, replaced) = drive("Connected", linked.clone());
    let mut app = web_app(services);
    linked.set(app.session.active().map(|d| d.uid));
    app.open_backstage(Page::SaveAs);
    let mut h = harness(app);
    assert!(h.query_by_label("Opened from Google Drive: แผนงาน.pptx").is_some());
    assert!(h.query_by_label("Google Drive › แผนงาน.pptx").is_some(), "where it is saved, not \"Not saved yet\"");
    assert!(replaced.borrow().is_empty());
    h.get_by_label("Save to this file on Google Drive").click();
    h.run_steps(3);
    let (uid, rev) = h.state().session.active().map(|d| (d.uid, d.revision)).unwrap();
    let calls = replaced.borrow();
    assert_eq!(calls.len(), 1);
    assert_eq!((calls[0].0, calls[0].1), (uid, rev));
    assert!(calls[0].2.starts_with(b"PK"));
    assert!(copies.borrow().is_empty());
    assert!(h.state().backstage.is_none(), "back to the presentation");
}

/// Save (File › Save, Ctrl+S, the toolbar) of a presentation from Drive replaces its Drive file
/// instead of downloading; another presentation still downloads.
#[test]
fn save_goes_back_to_drive_only_for_a_presentation_from_drive() {
    let linked = Rc::new(Cell::new(None));
    let (services, _, _, replaced) = drive("Connected", linked.clone());
    let downloads: Calls<String> = Rc::default();
    let got = downloads.clone();
    let mut app = SlideApp::new(
        Session::with_new(),
        Services {
            google_drive: Some(services),
            download: Some(Box::new(move |name: &str, _: &[u8]| got.borrow_mut().push(name.to_string()))),
            ..Default::default()
        },
    );
    linked.set(app.session.active().map(|d| d.uid));
    app.run("slide.new", json!({})).unwrap();
    app.save();
    let (uid, rev) = app.session.active().map(|d| (d.uid, d.revision)).unwrap();
    assert_eq!(replaced.borrow().iter().map(|c| (c.0, c.1)).collect::<Vec<_>>(), vec![(uid, rev)]);
    assert!(downloads.borrow().is_empty(), "nothing is downloaded");
    assert!(app.session.active().unwrap().is_dirty(), "saved only when Drive says so");
    app.run("file.new", json!({})).unwrap();
    app.save();
    assert_eq!(replaced.borrow().len(), 1);
    assert_eq!(downloads.borrow().len(), 1);
}

#[test]
fn a_stuck_sign_in_can_be_tried_again() {
    let connects = Rc::new(Cell::new(0));
    let count = connects.clone();
    let (mut services, ..) = drive("Signing in…", Rc::default());
    services.connect = Box::new(move || count.set(count.get() + 1));
    let mut app = web_app(services);
    app.open_backstage(Page::Open);
    let mut h = harness(app);
    h.get_by_label("Try signing in again").click();
    h.run_steps(2);
    assert_eq!(connects.get(), 1);
}

/// The web start screen has a Google Drive button that goes to Open, where Drive is.
#[test]
fn the_start_screen_leads_to_drive() {
    let (services, ..) = drive("Not connected", Rc::default());
    let mut app = SlideApp::new(Session::new(), Services { google_drive: Some(services), ..Default::default() });
    app.services.browser_recovery = Some(crate::BrowserRecoveryServices {
        drafts: Box::new(|_| Some(Vec::new())),
        act: Box::new(|_, _| {}),
        fragile: Box::new(|| false),
        open_draft: Box::new(|| None),
    });
    app.open_backstage(Page::Home);
    let mut h = harness(app);
    h.get_by_label("Google Drive").click();
    h.run_steps(3);
    assert_eq!(h.state().backstage.as_ref().map(|b| b.page), Some(Page::Open));
    assert!(h.query_by_label("Connect Google Drive").is_some());
}

/// Everything the Drive cards and the Drive code can show has a Thai translation.
#[test]
fn drive_is_in_thai() {
    let th = crate::i18n::Lang::from_code("th").unwrap();
    let missing: Vec<&str> = krub_drive::STATUS.iter().chain(krub_drive::MESSAGES).copied().filter(|s| !crate::i18n::has(th, s)).collect();
    assert!(missing.is_empty(), "add to th.tsv: {missing:?}");
    for status in ["Connected", "Not connected"] {
        let linked = Rc::new(Cell::new(None));
        let (services, ..) = drive(status, linked.clone());
        let mut app = web_app(services);
        linked.set(app.session.active().map(|d| d.uid));
        app.run("ui.language", json!({"value": "th"})).unwrap();
        let mut h = harness(app);
        crate::i18n::record::start();
        for page in [Page::Open, Page::SaveAs] {
            h.state_mut().open_backstage(page);
            h.run_steps(3);
        }
        let seen = crate::i18n::record::stop();
        crate::i18n::set_current(crate::i18n::Lang::EN);
        let untranslated: Vec<String> = seen
            .into_iter()
            .map(|(_, s)| s)
            .filter(|s| (s.contains("Drive") || s.contains("device") || s.contains("Connect")) && !crate::i18n::has(th, s))
            .collect();
        assert!(untranslated.is_empty(), "add to th.tsv: {untranslated:?}");
    }
}
