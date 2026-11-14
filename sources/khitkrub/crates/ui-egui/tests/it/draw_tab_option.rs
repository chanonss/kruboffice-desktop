//! The Draw tab is off by default (File › Options › General turns it on).

use gridcraft_engine::Session;

#[test]
fn the_draw_tab_stays_hidden_until_it_is_turned_on() {
    let mut session = Session::new();
    session.new_workbook();
    let mut h = crate::support::harness(session, 4);
    assert!(!h.state().ui.show_draw_tab, "Draw is off by default");
    // A remembered or keytip-selected Draw tab falls back to Home rather than showing without its tab.
    h.state_mut().ui.ribbon_tab = "Draw".into();
    h.run_steps(3);
    assert_eq!(h.state().ui.ribbon_tab, "Home");
    h.state_mut().ui.show_draw_tab = true;
    h.state_mut().ui.ribbon_tab = "Draw".into();
    h.run_steps(3);
    assert_eq!(h.state().ui.ribbon_tab, "Draw", "with the option on the tab is kept");
}

/// The Home groups have no launchers (owner, 2026-11-13: the two-size ribbon, as in KhianKrub);
/// Format Cells still opens on each tab the launchers opened: Alignment at the end of the
/// Orientation menu, Number at the end of the number format list, Font from its shortcut.
#[test]
fn format_cells_tabs_have_other_ways_in_than_launchers() {
    let home = include_str!("../../src/ribbon/home.rs");
    assert!(home.contains(r#""Format Cell Alignment…", "dialog:formatCells", json!({"tab": "Alignment"})"#));
    assert!(home.contains("More Number Formats…") && home.contains(r#"app.open_dialog("formatCells", json!({"tab": "Number"}))"#));
    let keymap = include_str!("../../src/keymap.rs");
    assert!(keymap.contains(r#"Shortcut::Dialog("formatCells", json!({"tab": "Font"}))"#));
    assert!(!home.contains("launcher_button"), "the ribbon has no group launchers");
}
