//! Google Drive on the web's File pages: the place cards on Open and Save As, explicit clicks only,
//! and Save going back to the Drive file a workbook came from.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use serde_json::json;

use crate::backstage::Page;
use crate::i18n::Language;
use crate::{DriveServices, Services, SheetApp};

type Calls<T> = Rc<RefCell<Vec<T>>>;

struct Recorded {
    opened: Calls<(u64, u64)>,
    copies: Calls<(String, Vec<u8>)>,
    replaced: Calls<(u64, u64, Vec<u8>)>,
}

/// Drive services that record what they were asked; `linked` names the workbook (by uid) that came
/// from Drive.
fn drive(status: &'static str, linked: Rc<Cell<Option<u64>>>) -> (DriveServices, Recorded) {
    let recorded = Recorded { opened: Rc::default(), copies: Rc::default(), replaced: Rc::default() };
    let (o, c, r) = (recorded.opened.clone(), recorded.copies.clone(), recorded.replaced.clone());
    let services = DriveServices {
        status: Box::new(move || status),
        connect: Box::new(|| {}),
        disconnect: None,
        open: Box::new(move |doc, rev| o.borrow_mut().push((doc, rev))),
        files: Box::new(|| None),
        select_file: Box::new(|_, _, _| {}),
        save_copy: Box::new(move |name, bytes| c.borrow_mut().push((name, bytes))),
        linked_name: Box::new(move |doc| (linked.get() == Some(doc)).then(|| "budget.xlsx".to_string())),
        save_over: Box::new(move |doc, rev, bytes| r.borrow_mut().push((doc, rev, bytes))),
    };
    (services, recorded)
}

fn web_app(drive: DriveServices, downloads: Calls<String>) -> SheetApp {
    let mut session = gridcraft_engine::Session::new();
    session.new_workbook();
    let services = Services {
        google_drive: Some(drive),
        download: Some(Box::new(move |name: &str, _: &[u8]| downloads.borrow_mut().push(name.to_string()))),
        ..Default::default()
    };
    SheetApp::new(session, services)
}

fn harness(mut app: SheetApp) -> Harness<'static, SheetApp> {
    app.use_prepared_fonts();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_context_setup(|ctx| {
            ctx.set_fonts(crate::theme::pinned_font_definitions());
            crate::theme::apply(ctx, false);
        })
        .renderer(egui_kittest::LazyRenderer::Uninitialized { textures_delta: Default::default(), builder: None })
        .build_ui_state(
            |ui, app: &mut SheetApp| {
                app.logic(ui.ctx());
                app.ui(ui);
            },
            app,
        );
    h.run_steps(3);
    h
}

fn active(app: &SheetApp) -> (u64, u64) {
    app.session.active().map(|d| (d.uid, d.revision)).unwrap()
}

#[test]
fn open_and_save_a_copy_are_explicit_clicks_on_the_place_cards() {
    let (services, rec) = drive("Connected", Rc::default());
    let mut app = web_app(services, Rc::default());
    app.open_backstage(Page::Open);
    let mut h = harness(app);
    assert!(h.query_by_label("This device").is_some(), "the device card is beside Drive");
    assert!(rec.opened.borrow().is_empty() && rec.copies.borrow().is_empty(), "nothing is sent before a click");
    h.get_by_label("Choose from Google Drive…").click();
    h.run_steps(3);
    assert_eq!(rec.opened.borrow().as_slice(), &[active(h.state())]);
    h.state_mut().open_backstage(Page::SaveAs);
    h.run_steps(3);
    assert!(h.query_by_label("Save to this file on Google Drive").is_none(), "not opened from Drive");
    h.get_by_label("Save a new copy to Google Drive").click();
    h.run_steps(3);
    let copies = rec.copies.borrow();
    assert_eq!(copies.len(), 1);
    assert!(copies[0].0.ends_with(".xlsx"), "{}", copies[0].0);
    assert!(copies[0].1.starts_with(b"PK"), "an Excel workbook");
    assert!(rec.replaced.borrow().is_empty(), "a copy never replaces a Drive file");
}

#[test]
fn a_workbook_from_drive_saves_back_over_its_file_from_save_as() {
    let linked = Rc::new(Cell::new(None));
    let (services, rec) = drive("Connected", linked.clone());
    let mut app = web_app(services, Rc::default());
    linked.set(app.session.active().map(|d| d.uid));
    app.open_backstage(Page::SaveAs);
    let mut h = harness(app);
    assert!(h.query_by_label("Opened from Google Drive: budget.xlsx").is_some());
    assert!(rec.replaced.borrow().is_empty());
    h.get_by_label("Save to this file on Google Drive").click();
    h.run_steps(3);
    let calls = rec.replaced.borrow();
    assert_eq!(calls.len(), 1);
    assert_eq!((calls[0].0, calls[0].1), active(h.state()));
    assert!(calls[0].2.starts_with(b"PK"));
    assert!(rec.copies.borrow().is_empty());
    assert!(h.state().backstage.is_none(), "back to the sheet");
}

/// Save (File › Save, Ctrl+S, the title bar) of a workbook from Drive replaces its Drive file
/// instead of downloading; another workbook still downloads.
#[test]
fn save_goes_back_to_drive_only_for_a_workbook_from_drive() {
    let linked = Rc::new(Cell::new(None));
    let (services, rec) = drive("Connected", linked.clone());
    let downloads: Calls<String> = Rc::default();
    let mut app = web_app(services, downloads.clone());
    linked.set(app.session.active().map(|d| d.uid));
    app.run("cell.set", json!({"cell": "A1", "input": "42"})).unwrap();
    let v = app.run("file.save", json!({})).unwrap();
    assert_eq!(v["googleDrive"], "budget.xlsx");
    assert_eq!(rec.replaced.borrow().iter().map(|c| (c.0, c.1)).collect::<Vec<_>>(), vec![active(&app)]);
    assert!(downloads.borrow().is_empty(), "nothing is downloaded");
    assert!(app.session.active().unwrap().is_dirty(), "saved only when Drive says so");
    app.run("file.new", json!({})).unwrap();
    app.run("file.save", json!({})).unwrap();
    assert_eq!(rec.replaced.borrow().len(), 1, "another workbook does not go to Drive");
}

#[test]
fn a_stuck_sign_in_can_be_tried_again() {
    let connects = Rc::new(Cell::new(0));
    let count = connects.clone();
    let (mut services, _) = drive("Signing in…", Rc::default());
    services.connect = Box::new(move || count.set(count.get() + 1));
    let mut app = web_app(services, Rc::default());
    app.open_backstage(Page::Open);
    let mut h = harness(app);
    h.get_by_label("Try signing in again").click();
    h.run_steps(2);
    assert_eq!(connects.get(), 1);
}

/// The web start screen has a Google Drive button that goes to Open, where Drive is.
#[test]
fn the_start_screen_leads_to_drive() {
    let (services, _) = drive("Not connected", Rc::default());
    let mut app = web_app(services, Rc::default());
    app.services.browser_recovery = Some(crate::BrowserRecoveryServices {
        drafts: Box::new(|_| Some(Vec::new())),
        act: Box::new(|_, _| {}),
        fragile: Box::new(|| false),
        open_draft: Box::new(|| None),
    });
    app.set_web_start_screen(true);
    let mut h = harness(app);
    h.get_by_label("Google Drive").click();
    h.run_steps(3);
    assert_eq!(h.state().backstage.as_ref().map(|b| b.page), Some(Page::Open));
    assert!(h.query_by_label("Connect Google Drive").is_some());
}

/// Everything the Drive cards and the Drive code can show has a Thai translation.
#[test]
fn drive_is_in_thai() {
    let shown = krub_drive::STATUS.iter().chain(krub_drive::MESSAGES).copied().chain([
        "This device",
        "Choose a file stored on this computer or iPad.",
        "Browse files on this device…",
        "Download a copy to this computer or iPad.",
        "Save to your Google Drive.",
        "Open an Excel workbook from your Google Drive and save it back there.",
        "Try signing in again",
        "Connect Google Drive",
        "Disconnect Google Drive",
        "Choose from Google Drive…",
        "Files available to KhitKrub (up to 100):",
        "No available Excel workbooks found.",
        "Opened from Google Drive: {name}",
        "Replaces the file on Google Drive. Drive keeps earlier versions for a while.",
        "Each save creates a new file. Changes remain unsaved in the editor.",
        "Save to this file on Google Drive",
        "Save a new copy to Google Drive",
    ]);
    // The test fonts have no Thai letters, so this checks the table rather than the drawn page.
    let missing: Vec<&str> = shown.filter(|s| Language::Th.tr(s) == *s).collect();
    assert!(missing.is_empty(), "add to i18n/th.rs: {missing:?}");
}

#[test]
fn file_navigation_matches_the_suite_and_opens_recover() {
    let (services, _) = drive("Connected", Rc::default());
    let mut app = web_app(services, Rc::default());
    app.open_backstage(Page::Home);
    let mut h = harness(app);
    for label in ["Home", "New", "Open", "Info", "Recover", "Save", "Print", "Export", "Options", "Back", "About"] {
        assert!(h.query_all_by_label(label).any(|n| n.rect().left() < 210.0), "missing File navigation: {label}");
    }
    assert!(!h.query_all_by_label("Save As").any(|n| n.rect().left() < 210.0));
    assert!(!h.query_all_by_label("Close").any(|n| n.rect().left() < 210.0));
    h.query_all_by_label("Options").find(|n| n.rect().left() < 210.0).unwrap().click();
    h.run_steps(3);
    assert_eq!(h.state().backstage.as_ref().map(|b| b.page), Some(Page::Options));
    h.query_all_by_label("Recover").find(|n| n.rect().left() < 210.0).unwrap().click();
    h.run_steps(3);
    assert_eq!(h.state().backstage.as_ref().map(|b| b.page), Some(Page::Recover));
}

#[test]
fn desktop_close_keeps_the_file_page_when_unsaved_work_needs_a_decision() {
    let mut session = gridcraft_engine::Session::new();
    session.new_workbook();
    let mut app = SheetApp::new(session, Services::default());
    app.run("cell.set", json!({"cell": "A1", "input": "unsaved"})).unwrap();
    app.open_backstage(Page::Home);
    let mut h = harness(app);
    h.query_all_by_label("Close").find(|n| n.rect().left() < 210.0).unwrap().click();
    h.run_steps(3);
    assert!(h.state().session.active().is_some());
    assert!(h.state().dialog.is_some(), "unsaved work needs a decision");
    assert!(h.state().backstage.is_some(), "the File page remains behind the prompt");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn desktop_recover_page_restores_an_unsaved_copy() {
    let dir = std::env::temp_dir().join(format!("khitkrub-recover-page-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut old = gridcraft_engine::Session::new();
    old.recovery_dir = Some(dir.clone());
    old.new_workbook();
    old.execute("cell.set", json!({"cell": "A1", "input": "recovered text"})).unwrap();
    gridcraft_engine::recovery::checkpoint(&old, &dir).unwrap();
    let file = dir.read_dir().unwrap().flatten().find(|e| e.path().extension().is_some_and(|e| e == "krubrecovery")).unwrap().path();
    let stem = file.file_stem().unwrap().to_str().unwrap();
    let (owner, uid) = stem.split_once('-').unwrap();
    let foreign = owner.parse::<u64>().unwrap() ^ (1 << 40);
    std::fs::rename(&file, dir.join(format!("{foreign}-{uid}.krubrecovery"))).unwrap();
    let mut session = gridcraft_engine::Session::new();
    session.recovery_dir = Some(dir.clone());
    session.new_workbook();
    let mut app = SheetApp::new(session, Services::default());
    app.open_backstage(Page::Recover);
    let mut h = harness(app);
    h.get_by_label("Discard…").click();
    h.run_steps(2);
    assert!(h.query_by_label("Permanently discard this recovery copy?").is_some());
    h.get_by_label("Cancel").click();
    h.run_steps(2);
    assert!(dir.read_dir().unwrap().flatten().any(|e| e.path().extension().is_some_and(|e| e == "krubrecovery")));
    h.get_by_label("Restore").click();
    h.run_steps(3);
    let doc = h.state().session.active().unwrap();
    assert!(doc.is_dirty() && doc.path.is_none());
    assert!(doc.display_title().contains("(Recovered)"));
    let cell = h.state_mut().session.execute("cell.get", json!({"cell":"A1"})).unwrap();
    assert!(cell.to_string().contains("recovered text"), "{cell}");
    assert!(h.state().backstage.is_none());
    let _ = std::fs::remove_dir_all(dir);
}
