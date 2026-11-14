//! The web start screen (nothing opened yet in this tab): the new-workbook tiles and the drafts kept
//! in this browser. Every choice opens in a new tab with its own address, and the start screen
//! stays where it is.

use std::cell::RefCell;
use std::rc::Rc;

use egui_kittest::kittest::Queryable as _;
use gridcraft_engine::Session;
use gridcraft_ui_egui::i18n::Language;
use gridcraft_ui_egui::{BrowserDraft, BrowserRecoveryServices, DraftAction, Inbox, Services, SheetApp};

use crate::common::{Harness, builder, harness_with, prepare};
use crate::support::blank_session;

type Asked = Rc<RefCell<Vec<(DraftAction, String)>>>;

/// A web app showing the start screen, with browser drafts `list` (`None` while loading) and the
/// draft actions the host was asked for. The window is tall, so the drafts are not below its fold.
fn web_start(list: Option<Vec<BrowserDraft>>, inbox: Option<Inbox>) -> (Harness, Asked) {
    web_start_with(list, inbox, false)
}

/// [`web_start`] in a browser that may remove drafts by itself (`fragile`) or not.
fn web_start_with(list: Option<Vec<BrowserDraft>>, inbox: Option<Inbox>, fragile: bool) -> (Harness, Asked) {
    let asked: Asked = Rc::default();
    let log = asked.clone();
    let services = Services {
        inbox,
        browser_recovery: Some(BrowserRecoveryServices {
            drafts: Box::new(move |_| list.clone()),
            act: Box::new(move |what, id| log.borrow_mut().push((what, id.to_string()))),
            fragile: Box::new(move || fragile),
            open_draft: Box::new(|| None),
        }),
        ..Default::default()
    };
    let mut app = prepare(SheetApp::new(blank_session(), services));
    app.ui.language = Language::En;
    app.set_web_start_screen(true);
    (harness_with(builder().with_size(egui::vec2(1440.0, 1100.0)), app, 4), asked)
}

/// The OpenUrl commands (url, new tab) the last frame asked the browser for.
fn opened_urls(h: &Harness) -> Vec<(String, bool)> {
    h.output()
        .platform_output
        .commands
        .iter()
        .filter_map(|c| match c {
            egui::OutputCommand::OpenUrl(o) => Some((o.url.clone(), o.new_tab)),
            _ => None,
        })
        .collect()
}

/// Clicks `label` and returns the addresses opened in the following frames.
fn click_and_collect(h: &mut Harness, label: &str) -> Vec<(String, bool)> {
    let mut opened = Vec::new();
    h.get_all_by_label(label).next().unwrap().click();
    for _ in 0..2 {
        h.step();
        opened.extend(opened_urls(h));
    }
    opened
}

fn two_drafts() -> Vec<BrowserDraft> {
    vec![
        BrowserDraft { id: "doc-1".into(), name: "Monthly report.xlsx".into(), saved: "29/10/2026, 14:32".into() },
        BrowserDraft { id: "doc-2".into(), name: "Class marks".into(), saved: "28/10/2026, 09:10".into() },
    ]
}

/// The start screen lists the drafts kept in the browser; a click opens one in a new tab, the ⋮ menu
/// downloads or discards it (after asking), and storage still loading or empty says so.
#[test]
fn the_start_screen_lists_browser_drafts_and_opens_one_in_a_new_tab() {
    let (mut h, asked) = web_start(Some(two_drafts()), None);
    assert!(h.query_by_label("Hello! What would you like to work on?").is_some(), "the web start screen, not File's Home");
    assert!(h.query_by_label("Drafts on this device").is_some());
    assert!(h.query_by_label("Monthly report.xlsx (draft)").is_some(), "each draft says it is one");
    assert!(h.query_by_label_contains("may remove drafts by itself").is_none(), "this browser keeps them");
    assert!(h.query_by_label("Open file").is_some() && h.query_by_label("KrubOffice apps").is_some(), "the top bar");
    assert!(h.query_by_label("Print").is_none(), "no File sidebar, no ribbon");
    assert_eq!(click_and_collect(&mut h, "Class marks (draft)"), [("./?draft=doc-2".to_string(), true)]);
    assert!(asked.borrow().is_empty(), "this tab does not restore the draft");
    h.get_all_by_label("More options").next().unwrap().click();
    h.run_steps(2);
    h.get_by_label("Download a copy").click();
    h.run_steps(2);
    assert_eq!(asked.borrow().last(), Some(&(DraftAction::Download, "doc-1".to_string())));
    h.get_all_by_label("More options").next().unwrap().click();
    h.run_steps(2);
    h.get_by_label("Discard draft…").click();
    h.run_steps(2);
    assert_eq!(asked.borrow().len(), 1, "discarding asks first (only the download was asked)");
    h.get_by_label("Discard draft").click();
    h.run_steps(2);
    assert_eq!(asked.borrow().last(), Some(&(DraftAction::Discard, "doc-1".to_string())));
    assert!(h.state().web_start_screen(), "nothing opens here; the new tab holds the draft");

    let (h, _) = web_start_with(Some(two_drafts()), None, true);
    assert!(h.query_by_label_contains("may remove drafts by itself").is_some(), "a browser that may clear them says so");
    let (h, _) = web_start(None, None);
    assert!(h.query_by_label("Loading drafts…").is_some());
    let (h, _) = web_start(Some(Vec::new()), None);
    assert!(h.query_by_label_contains("pick up where you left off").is_some());
}

/// Blank and each sample open in a new tab with their own address; the start screen stays as it is.
#[test]
fn the_new_workbook_tiles_open_in_a_new_tab() {
    let (mut h, _) = web_start(Some(Vec::new()), None);
    let mut tiles = vec![("Blank workbook".to_string(), "./?new=blank".to_string())];
    for (name, title) in gridcraft_engine::sample::SAMPLES {
        tiles.push((title.to_string(), format!("./?sample={name}")));
    }
    for (label, url) in tiles {
        assert_eq!(click_and_collect(&mut h, &label), [(url, true)], "{label}");
        assert!(h.state().web_start_screen(), "{label}: the new tab holds the workbook");
    }
}

/// Options opens over the start screen as a page of its own, and Back returns to the start screen.
#[test]
fn options_opens_over_the_start_screen_and_back_returns_to_it() {
    let (mut h, _) = web_start(Some(Vec::new()), None);
    h.get_by_label("Options").click();
    h.run_steps(3);
    assert!(!h.state().web_start_screen(), "the Options page shows");
    assert!(h.query_by_label("Interface language").is_some());
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Back").click();
    h.run_steps(3);
    assert!(h.state().web_start_screen(), "Back returns to the start screen");
    assert!(h.query_by_label("Hello! What would you like to work on?").is_some());
}

/// A file picked here (the web picker delivers it later through the inbox) opens in this tab: the
/// start screen gives way to it, and the blank workbook the window opened with goes.
#[test]
fn opening_a_file_leaves_the_start_screen_and_drops_the_blank_workbook() {
    let inbox: Inbox = Inbox::default();
    let (mut h, _) = web_start(Some(Vec::new()), Some(inbox.clone()));
    assert_eq!(h.state().session.documents().len(), 1, "the window starts with one blank workbook");
    let mut source = Session::new();
    source.new_workbook();
    source.execute("cell.set", serde_json::json!({"cell": "A1", "input": "42"})).unwrap();
    let wb = source.active().unwrap().wb.clone();
    let bytes = gridcraft_engine::io::save_bytes(&wb, "Marks.xlsx").unwrap();
    inbox.lock().unwrap().push(("Marks.xlsx".into(), bytes));
    h.run_steps(4);
    assert!(!h.state().web_start_screen(), "the opened workbook shows");
    assert_eq!(h.state().session.documents().len(), 1, "the blank workbook did not linger");
    assert_eq!(h.state().session.active().map(|d| d.display_title()).as_deref(), Some("Marks.xlsx"));
}

/// Leaving the start screen keeps a workbook that was edited: only the untouched blank one goes.
#[test]
fn an_edited_workbook_is_not_dropped_when_the_start_screen_closes() {
    let (mut h, _) = web_start(Some(Vec::new()), None);
    h.state_mut().run("cell.set", serde_json::json!({"cell": "A1", "input": "kept"})).unwrap();
    h.state_mut().session.new_workbook();
    h.state_mut().leave_web_start_screen();
    h.run_steps(2);
    assert_eq!(h.state().session.documents().len(), 2, "the edited workbook stays beside the new one");
}

/// Every label of the start screen has a Thai entry (the test fonts have no Thai glyphs, so the
/// Thai screen itself cannot be drawn here).
#[test]
fn the_start_screen_has_thai_text() {
    for label in [
        "Hello! What would you like to work on?",
        "Pick a workbook to start with, or pick up a draft where you left off.",
        "Blank workbook",
        "Household Budget",
        "Quarterly Sales Dashboard",
        "Gradebook",
        "Coffee Shop Sales",
        "Open file",
        "Options",
        "KrubOffice apps",
        "KhianKrub",
        "KhitKrub",
        "PhutKrub",
        "Documents",
        "Spreadsheets",
        "Presentations",
        "Here",
        "See all apps at kruboffice.app",
        "Drafts on this device",
        "Kept only in this browser on this device. Save to Google Drive or download a copy to keep it anywhere else.",
        "Kept only in this browser on this device. Download a copy to keep it anywhere else.",
        "This browser may remove drafts by itself: when it runs short of space, after a long time without a visit, or when a private window closes.",
        "draft",
        "Saved on this device",
        "Saving…",
        "Loading drafts…",
        "Workbooks you make here are kept in this browser, so you can pick up where you left off.",
        "More options",
        "Keep working",
        "Download a copy",
        "Discard draft…",
        "Discard draft",
        "Permanently discard this draft and its previous copy?",
        "Drafts: {error}",
        "This draft could not be opened: {error}",
        "This draft could not be downloaded: {error}",
    ] {
        assert_ne!(Language::Th.tr(label), label, "no Thai for {label:?}");
    }
}

/// The Save Changes prompt speaks the interface language: its question and buttons were English
/// even in Thai (the Thai entries existed but the prompt did not use them). It is drawn in Polish,
/// which the test fonts can show, and the Thai it would use is checked as text.
#[test]
fn the_save_changes_prompt_speaks_the_interface_language() {
    let mut app = prepare(SheetApp::new(blank_session(), Services::default()));
    app.ui.language = Language::Pl;
    app.open_dialog("saveChanges", serde_json::json!({"title": "Budget.xlsx"}));
    let h = harness_with(builder().with_size(egui::vec2(1200.0, 800.0)), app, 4);
    assert!(h.query_by_label_contains("Czy chcesz zapisać zmiany wprowadzone w pliku Budget.xlsx?").is_some(), "the question");
    for button in ["Nie zapisuj", "Anuluj", "Zapisz"] {
        assert!(h.query_all_by_label(button).next().is_some(), "{button}");
    }
    for english in ["Don't Save", "Cancel", "Save"] {
        assert!(h.query_by_label(english).is_none(), "{english} is still English");
    }
    assert!(h.query_by_label_contains("Do you want to save").is_none());
    let th = Language::Th;
    assert_eq!(th.tr("Do you want to save the changes you made to {title}?"), "ต้องการบันทึกการเปลี่ยนแปลงที่ทำกับ {title} หรือไม่");
    for (en, thai) in [
        ("Don't Save", "ไม่บันทึก"),
        ("Cancel", "ยกเลิก"),
        ("Save", "บันทึก"),
        ("Save changes to macro “{name}”?", "บันทึกการเปลี่ยนแปลงในแมโคร “{name}” หรือไม่"),
    ] {
        assert_eq!(th.tr(en), thai);
    }
}

/// Without a host that keeps drafts (the desktop app), the start screen never shows.
#[test]
fn without_browser_drafts_there_is_no_start_screen() {
    let mut app = prepare(SheetApp::new(blank_session(), Services::default()));
    app.ui.language = Language::En;
    app.set_web_start_screen(true);
    let mut h = harness_with(builder().with_size(egui::vec2(1200.0, 800.0)), app, 3);
    assert!(!h.state().web_start_screen());
    assert!(h.query_by_label("Hello! What would you like to work on?").is_none());
    h.run_steps(1);
}
