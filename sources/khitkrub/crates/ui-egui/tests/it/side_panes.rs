//! The right-hand dock (owner, 2026-11-16, as in KhianKrub and PhutKrub): a cell in a table opens
//! the Table pane instead of the Table Design tab; the dock stays open once used so the sheet
//! keeps still; an open task pane stays in front; the sheet keeps the keyboard.

use egui::Event;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use gridcraft_engine::Session;
use gridcraft_engine::core::{CellRef, Value};
use gridcraft_ui_egui::panes::active_pane;
use serde_json::json;

use crate::support::harness;

type Harness = egui_kittest::Harness<'static, gridcraft_ui_egui::SheetApp>;

fn with_table() -> Session {
    let mut session = crate::support::blank_session();
    session.execute("range.setValues", json!({"range": "A1", "values": [["Name", "Qty"], ["a", 1], ["b", 2]]})).unwrap();
    session.execute("insert.table", json!({"range": "A1:B3", "header": true})).unwrap();
    session.execute("selection.set", json!({"range": "D10"})).unwrap();
    session
}

fn select(h: &mut Harness, cell: &str) {
    h.state_mut().session.execute("selection.set", json!({"range": cell})).unwrap();
    h.run_steps(3);
}

fn value(h: &Harness, a: &str) -> Value {
    h.state().session.active().and_then(|d| d.wb.active().map(|s| s.value(CellRef::parse(a).unwrap_or_default()))).unwrap_or_default()
}

#[test]
fn a_cell_in_a_table_opens_the_table_pane_and_the_sheet_keeps_still() {
    let mut h = harness(with_table(), 4);
    assert_eq!(active_pane(&h.ctx), None, "no pane outside the table at first");
    select(&mut h, "A2");
    assert_eq!(active_pane(&h.ctx), Some("obj:table"));
    assert_eq!(h.state().ui.ribbon_tab, "Home", "no Table Design tab");
    let grid = h.state().grid.rect;
    select(&mut h, "D10");
    assert_eq!(active_pane(&h.ctx), Some("obj:idle"), "the dock stays open, empty");
    assert_eq!(h.state().grid.rect, grid, "the sheet didn't move");
    select(&mut h, "B3");
    assert_eq!(active_pane(&h.ctx), Some("obj:table"));
    assert_eq!(h.state().grid.rect, grid);
}

#[test]
fn an_open_task_pane_stays_in_front_and_the_strip_switches() {
    let mut h = harness(with_table(), 4);
    h.state_mut().grid.pane = Some("watch".into());
    h.run_steps(3);
    select(&mut h, "A2");
    assert_eq!(active_pane(&h.ctx), Some("watch"), "the Watch Window stays in front");
    h.get_by_label("Pane: Table").click();
    h.run_steps(3);
    assert_eq!(active_pane(&h.ctx), Some("obj:table"));
}

#[test]
fn typing_after_the_pane_opens_goes_into_the_cell() {
    let mut h = harness(with_table(), 4);
    select(&mut h, "A2");
    assert_eq!(active_pane(&h.ctx), Some("obj:table"));
    for c in "Zed".chars() {
        h.input_mut().events.push(Event::Text(c.to_string()));
        h.run_steps(1);
    }
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert_eq!(value(&h, "A2"), Value::text("Zed"));
}

/// A shape's pane is the one shared by the three apps (`krub_icons::shape_pane`): Area, Line,
/// Effect, Position and Size in that order, then KhitKrub's Properties. A change applies at once:
/// a shadow picked from the menu, the fill transparency typed in.
#[test]
fn a_shape_pane_has_the_shared_sections() {
    use gridcraft_engine::model::{ObjKind, ObjRef};
    let mut session = crate::support::blank_session();
    let r = session.execute("insert.shape", json!({"kind": "rectangle", "at": "C5", "width": 120, "height": 60})).unwrap();
    let id = r["shape"].as_u64().unwrap() as u32;
    let mut h = harness(session, 3);
    let shape = ObjRef::new(ObjKind::Shape, id);
    h.state_mut().selected_object = Some(shape);
    h.run_steps(3);
    assert_eq!(active_pane(&h.ctx), Some("obj:shape"));
    // Three sub-panels (owner, 2026-11-18): Fill and Line, Effects, Size and Position.
    use krub_icons::shape_pane::Tab;
    let top = |h: &Harness, label: &str| h.get_by_label(label).rect().top();
    for tab in ["Fill and Line", "Effects", "Size and Position"] {
        assert!(h.query_by_label(tab).is_some(), "the {tab} slot");
    }
    let order = ["Area", "Line"].map(|l| top(&h, l));
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
    for row in ["Fill:", "Line style:", "Thickness:"] {
        assert!(h.query_by_label(row).is_some(), "{row}");
    }
    crate::support::show_shape_tab(&mut h, Tab::SizePosition);
    let order = ["Width:", "Flip:", "Properties"].map(|l| top(&h, l));
    assert!(order.windows(2).all(|w| w[0] < w[1]), "Size, Arrange, then KhitKrub's Properties: {order:?}");
    for row in ["Arrange:", "Flip:", "Keep ratio"] {
        assert!(h.query_by_label(row).is_some(), "{row}");
    }
    crate::support::show_shape_tab(&mut h, Tab::Effects);
    for row in ["Shadow ⏷", "Soft Edges", "Glow"] {
        assert!(h.query_by_label(row).is_some(), "{row}");
    }
    // The Shadow menu's tiles set and remove the shadow.
    h.get_by_label("Shadow ⏷").click();
    h.run_steps(2);
    h.get_by_label("Bottom shadow").click();
    h.run_steps(2);
    let fmt = |h: &Harness| h.state().session.active().unwrap().wb.active().unwrap().shapes[0].format.clone();
    assert_eq!(fmt(&h).shadow.map(|s| s.dir), Some(90.0));
    h.get_by_label("Shadow ⏷").click();
    h.run_steps(2);
    h.get_by_label("No Shadow").click();
    h.run_steps(2);
    assert_eq!(fmt(&h).shadow, None);
    crate::support::show_shape_tab(&mut h, Tab::SizePosition);
    h.get_by_label("Bring to Front").click();
    h.run_steps(2);
    h.state_mut().run("shape.format", json!({"id": id, "fillTransparency": 30})).unwrap();
    h.run_steps(2);
    let f = &h.state().session.active().unwrap().wb.active().unwrap().shapes[0].format;
    assert_eq!(f.fill_transparency, 30.0);
}

/// Enter commits a number typed into the pane and stays there: it doesn't also reach the sheet
/// and move the active cell (seen in the browser, 2026-11-16).
#[test]
fn enter_in_a_pane_box_does_not_move_the_active_cell() {
    use gridcraft_engine::model::{ObjKind, ObjRef};
    let mut session = crate::support::blank_session();
    let r = session.execute("insert.shape", json!({"kind": "rectangle", "at": "C5", "width": 120, "height": 60})).unwrap();
    let id = r["shape"].as_u64().unwrap() as u32;
    session.execute("selection.set", json!({"range": "E8"})).unwrap();
    let mut h = harness(session, 3);
    h.state_mut().selected_object = Some(ObjRef::new(ObjKind::Shape, id));
    h.run_steps(3);
    let boxes: Vec<_> = h.get_all_by_label("Thickness:").filter(|n| n.accesskit_node().role() == egui::accesskit::Role::SpinButton).collect();
    boxes[0].focus();
    h.run_steps(2);
    h.input_mut().events.push(Event::Key {
        key: egui::Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    });
    h.run_steps(1);
    h.input_mut().events.push(Event::Text("3".into()));
    h.run_steps(1);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    let d = h.state().session.active().unwrap();
    assert_eq!(d.selection.active, CellRef::parse("E8").unwrap(), "the active cell stayed");
    assert_eq!(d.wb.active().unwrap().shapes[0].format.line_width, Some(3.0));
}

/// Text typed with the Enter that commits a pane box stays in the box: when the box lets go of
/// the keyboard, the sheet must not pick up that typing, in the same frame or handed over again
/// just after, and start editing the active cell with it (seen in the browser, 2026-11-16).
#[test]
fn typing_and_enter_in_one_frame_stay_in_the_pane_box() {
    use gridcraft_engine::model::{ObjKind, ObjRef};
    let mut session = crate::support::blank_session();
    let r = session.execute("insert.shape", json!({"kind": "rectangle", "at": "C5", "width": 120, "height": 60})).unwrap();
    let id = r["shape"].as_u64().unwrap() as u32;
    session.execute("selection.set", json!({"range": "E8"})).unwrap();
    let mut h = crate::support::stepped_harness(session);
    h.state_mut().selected_object = Some(ObjRef::new(ObjKind::Shape, id));
    h.run_steps(3);
    let boxes: Vec<_> = h.get_all_by_label("Thickness:").filter(|n| n.accesskit_node().role() == egui::accesskit::Role::SpinButton).collect();
    boxes[0].focus();
    h.run_steps(2);
    let key = |key, modifiers| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers };
    h.input_mut().events.extend([key(egui::Key::A, egui::Modifiers::COMMAND), Event::Text("4".into()), key(egui::Key::Enter, egui::Modifiers::NONE)]);
    h.run_steps(1);
    // The web can hand the same text over again just after the box let go.
    h.input_mut().events.push(Event::Text("4".into()));
    h.run_steps(3);
    let d = h.state().session.active().unwrap();
    assert_eq!(d.selection.active, CellRef::parse("E8").unwrap(), "the active cell stayed");
    assert_eq!(d.selection.ranges, vec![gridcraft_engine::core::RangeRef::parse("E8").unwrap()], "Cmd+A stayed in the box");
    assert!(h.state().editor.is_none(), "no cell edit started");
    assert_eq!(value(&h, "E8"), Value::Empty);
    assert_eq!(d.wb.active().unwrap().shapes[0].format.line_width, Some(4.0));
}
