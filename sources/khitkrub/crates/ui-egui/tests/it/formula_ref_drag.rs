//! Editing a formula, its references can be dragged on the sheet as in Excel: a corner resizes
//! one, the border moves it, and the formula's text follows. Through real pointer events.

use egui::{Event, Modifiers, PointerButton, Pos2, vec2};
use gridcraft_engine::Session;
use gridcraft_engine::core::RangeRef;
use gridcraft_ui_egui::{SheetApp, grid::Geo};
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;

/// B2:B5 hold numbers and B7 is being edited as `formula` (as after a double-click on it).
fn editing(formula: &str) -> Harness {
    let mut session = Session::new();
    session.new_workbook();
    session.execute("range.setValues", json!({"range": "B2:B5", "values": [[1], [2], [3], [4]]})).unwrap();
    session.execute("selection.set", json!({"range": "B7"})).unwrap();
    let mut h = crate::common::builder().with_step_dt(1.0 / 60.0).with_size(vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        crate::common::app(session),
    );
    h.run_steps(4);
    h.state_mut().begin_edit(Some(formula.into()), false);
    h.run_steps(3);
    h
}

fn rect_of(h: &Harness, range: &str) -> egui::Rect {
    let app = h.state();
    let sheet = app.session.active().unwrap().wb.active().unwrap();
    let geo = Geo::new(sheet, app.grid.rect.unwrap(), app.view().scroll);
    geo.range_rect(sheet, RangeRef::parse(range).unwrap())
}

fn drag(h: &mut Harness, from: Pos2, to: Pos2) {
    h.input_mut().events.push(Event::PointerMoved(from));
    h.run_steps(2);
    h.input_mut().events.push(Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    for k in 1..=4 {
        let p = from + (to - from) * (k as f32 / 4.0);
        h.input_mut().events.push(Event::PointerMoved(p));
        h.run_steps(1);
    }
    h.input_mut().events.push(Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn text(h: &Harness) -> String {
    h.state().editor.as_ref().map(|e| e.text.clone()).unwrap_or_default()
}

#[test]
fn dragging_a_corner_resizes_the_sum_range() {
    let mut h = editing("=SUM(B2:B5)");
    let from = rect_of(&h, "B2:B5").right_bottom();
    let to = rect_of(&h, "B8").center();
    drag(&mut h, from, to);
    assert_eq!(text(&h), "=SUM(B2:B8)");
    assert!(h.state().editor.is_some(), "still editing: Enter commits");
    assert_eq!(h.state().session.active().unwrap().selection.active.a1(), "B7", "the edited cell stays");
}

#[test]
fn dragging_the_border_moves_the_range_and_keeps_dollars() {
    let mut h = editing("=AVERAGE($B$2:$B$5)*2");
    let r = rect_of(&h, "B2:B5");
    let from = egui::pos2(r.left() + 1.0, r.center().y);
    let to = from + vec2(rect_of(&h, "C2").width(), 0.0);
    drag(&mut h, from, to);
    assert_eq!(text(&h), "=AVERAGE($C$2:$C$5)*2");
}

#[test]
fn every_reference_of_the_formula_can_be_dragged() {
    let mut h = editing("=B2+MAX(B3:B4)");
    let from = rect_of(&h, "B3:B4").right_bottom();
    let to = rect_of(&h, "D6").center();
    drag(&mut h, from, to);
    assert_eq!(text(&h), "=B2+MAX(B3:D6)");
}
