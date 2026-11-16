//! A table's and a chart's commands are in the right-hand pane now (owner, 2026-11-16), not in
//! contextual tabs. A Table Design or Chart Design tab remembered in the saved UI state must not
//! leave the ribbon blank: it falls back to Home. The Table pane follows the selection.

use gridcraft_engine::Session;
use gridcraft_ui_egui::SheetApp;
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn harness(session: Session) -> Harness {
    crate::support::harness(session, 4)
}

fn session_with_table() -> Session {
    let mut session = Session::new();
    session.new_workbook();
    session.execute("range.setValues", json!({"range": "A1", "values": [["Name", "Qty"], ["a", 1], ["b", 2]]})).unwrap();
    session.execute("insert.table", json!({"range": "A1:B3", "header": true})).unwrap();
    session.execute("selection.set", json!({"range": "A2"})).unwrap();
    session
}

/// Sets a saved ribbon tab on a fresh app and builds the stepped harness, for unavailable-tab cases.
fn app_restored_on(tab: &str, session: Session) -> Harness {
    let mut app = crate::common::app(session);
    app.ui.ribbon_tab = tab.into();
    crate::support::app_harness(app, 4)
}

#[test]
fn a_saved_table_design_tab_without_a_table_opens_on_home() {
    let mut session = Session::new();
    session.new_workbook();
    let h = app_restored_on("Table Design", session);
    assert_eq!(h.state().ui.ribbon_tab, "Home");
}

#[test]
fn a_saved_chart_design_tab_without_a_chart_opens_on_home() {
    let mut session = Session::new();
    session.new_workbook();
    let h = app_restored_on("Chart Design", session);
    assert_eq!(h.state().ui.ribbon_tab, "Home");
}

#[test]
fn leaving_the_table_puts_its_pane_away() {
    let mut h = harness(session_with_table());
    h.run_steps(2);
    assert_eq!(gridcraft_ui_egui::panes::active_pane(&h.ctx), Some("obj:table"), "a cell in the table opens its pane");
    assert_eq!(h.state().ui.ribbon_tab, "Home", "no Table Design tab");
    h.state_mut().session.execute("selection.set", json!({"range": "D10"})).unwrap();
    h.run_steps(2);
    assert_eq!(gridcraft_ui_egui::panes::active_pane(&h.ctx), Some("obj:idle"), "outside it the dock keeps its place, empty");
}

#[test]
fn startup_opens_on_home_not_the_saved_tab() {
    let saved = gridcraft_ui_egui::UiState { ribbon_tab: "Table Design".into(), dark: true, ..Default::default() };
    let json = serde_json::to_string(&saved).unwrap();
    assert!(json.contains(r#""ribbonTab":"Table Design""#), "{json}");
    let restored = serde_json::from_str::<gridcraft_ui_egui::UiState>(&json).unwrap().at_startup();
    assert_eq!(restored.ribbon_tab, "Home", "the saved tab is not restored");
    assert!(restored.dark, "the other preferences are");
}

#[test]
fn an_ordinary_tab_stays_put_in_a_session() {
    let mut session = Session::new();
    session.new_workbook();
    let mut h = harness(session);
    h.state_mut().ui.ribbon_tab = "Data".into();
    h.run_steps(3);
    assert_eq!(h.state().ui.ribbon_tab, "Data");
}

#[test]
fn each_window_keeps_its_own_table_pane() {
    let mut h = harness(session_with_table());
    let first = h.state().session.active_window().unwrap();
    // A second window on the same workbook, with its active cell outside the table.
    h.state_mut().run("view.newWindow", json!({})).unwrap();
    let second = h.state().session.active_window().unwrap();
    h.state_mut().run("selection.set", json!({"cell": "D10"})).unwrap();
    h.state_mut().run("window.activate", json!({"window": first})).unwrap();
    h.run_steps(3);
    assert_eq!(gridcraft_ui_egui::panes::active_pane(&h.ctx), Some("obj:table"), "the first window's cell is in the table");
    h.state_mut().show_window(second);
    h.run_steps(3);
    h.state_mut().enter_window(first);
    h.run_steps(3);
    assert_eq!(h.state().session.active_window(), Some(first));
    assert_eq!(gridcraft_ui_egui::panes::active_pane(&h.ctx), Some("obj:table"), "the second window's frame leaves it");
    let d = h.state().session.active().unwrap();
    assert_eq!(d.selection.active.a1(), "A2", "the first window still selects a cell in the table");
}

/// The tab strip and the ribbon are the KrubOffice height (`krub_icons::chrome`), the same in all
/// three apps: collapsing the ribbon to its tab strip moves the sheet up by the ribbon's 70 points,
/// and the strip and ribbon together are 100.
#[test]
fn the_ribbon_is_the_krub_office_height() {
    let mut session = Session::new();
    session.new_workbook();
    let mut h = harness(session);
    let grid_top = |h: &Harness| h.state().grid.rect.expect("the sheet").top();
    let open = grid_top(&h);
    h.state_mut().ui.ribbon_collapsed = true;
    h.run_steps(4);
    let collapsed = grid_top(&h);
    assert_eq!(open - collapsed, krub_icons::chrome::RIBBON_H, "sheet at {open} with the ribbon, {collapsed} without");
    assert_eq!(krub_icons::chrome::TAB_STRIP_H + krub_icons::chrome::RIBBON_H, 100.0);
}
