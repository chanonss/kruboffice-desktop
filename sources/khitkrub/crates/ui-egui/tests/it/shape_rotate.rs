//! The rotation handle above a selected shape, and hit-testing a turned shape. Adapted from
//! GridCraft's `shape_select.rs` (952a3b3f, #1321): KhitKrub keeps the turn in `format.rotation`.

use egui::{Event, Modifiers, PointerButton, vec2};
use gridcraft_engine::Session;
use gridcraft_engine::model::Anchor;
use gridcraft_ui_egui::{SheetApp, grid::Geo};
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn fixture() -> (Harness, u32) {
    let mut session = Session::new();
    session.new_workbook();
    let shape = session.execute("insert.shape", json!({"kind": "rectangle", "at": "D3", "width": 160, "height": 100})).unwrap();
    let id = shape["shape"].as_u64().unwrap() as u32;
    let mut h = egui_kittest::Harness::builder().with_size(vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        SheetApp::new(session, Default::default()),
    );
    h.run_steps(4);
    h.state_mut().selected_object = None;
    h.run_steps(2);
    (h, id)
}

fn anchor(h: &Harness, id: u32) -> Anchor {
    h.state().session.doc().unwrap().wb.active().unwrap().shapes.iter().find(|s| s.id == id).unwrap().anchor
}

fn rotation(h: &Harness, id: u32) -> f32 {
    h.state().session.doc().unwrap().wb.active().unwrap().shapes.iter().find(|s| s.id == id).unwrap().format.rotation
}

fn rect(h: &Harness, id: u32) -> egui::Rect {
    let app = h.state();
    let sh = app.session.doc().unwrap().wb.active().unwrap();
    let geo = Geo::new(sh, app.grid.rect.unwrap(), app.view().scroll);
    let a = anchor(h, id);
    let min = geo.cell_rect(sh, a.cell).min + vec2(a.dx, a.dy) * geo.z;
    egui::Rect::from_min_size(min, vec2(a.width, a.height) * geo.z)
}

fn button(h: &mut Harness, pos: egui::Pos2, pressed: bool) {
    h.input_mut().events.push(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn click(h: &mut Harness, at: egui::Pos2) {
    h.input_mut().events.push(Event::PointerMoved(at));
    h.run_steps(2);
    button(h, at, true);
    button(h, at, false);
}

fn drag(h: &mut Harness, from: egui::Pos2, to: egui::Pos2) {
    h.input_mut().events.push(Event::PointerMoved(from));
    h.run_steps(2);
    button(h, from, true);
    h.input_mut().events.push(Event::PointerMoved(to));
    h.run_steps(3);
    button(h, to, false);
}

fn knob_at(h: &Harness, at: egui::Pos2) -> bool {
    h.output().shapes.iter().any(|c| matches!(&c.shape, egui::Shape::Circle(k) if (k.center - at).length() < 1.0))
}

#[test]
fn dragging_the_rotation_handle_turns_the_shape() {
    let (mut h, id) = fixture();
    let r = rect(&h, id);
    click(&mut h, r.center());
    let knob = gridcraft_ui_egui::frame::rotate_handle(r, 0.0);
    assert!(knob_at(&h, knob), "the rotation handle shows");
    h.input_mut().events.push(Event::PointerMoved(knob));
    h.run_steps(2);
    assert_eq!(h.output().platform_output.cursor_icon, egui::CursorIcon::Crosshair);
    drag(&mut h, knob, r.center() + vec2(200.0, 0.0));
    assert_eq!(rotation(&h, id), 90.0);
    assert_eq!(rect(&h, id), r, "turning keeps the box");
    h.state_mut().run("edit.undo", json!({})).unwrap();
    assert_eq!(rotation(&h, id), 0.0, "one drag is one undo step");
}

#[test]
fn a_turned_shape_is_hit_inside_its_turned_outline() {
    let (mut h, id) = fixture();
    h.state_mut().run("shape.format", json!({"id": id, "rotation": 90})).unwrap();
    h.run_steps(2);
    let r = rect(&h, id);
    // Turned 90°, the shape reaches half its width above and below its centre.
    let below = r.center() + vec2(0.0, r.width() / 2.0 - 5.0);
    assert!(!r.contains(below));
    click(&mut h, below);
    assert_eq!(h.state().selected_object.map(|o| o.id), Some(id));
}

#[test]
fn a_turned_shape_resizes_along_its_own_axes() {
    let (mut h, id) = fixture();
    h.state_mut().run("shape.format", json!({"id": id, "rotation": 90})).unwrap();
    h.run_steps(2);
    let r = rect(&h, id);
    click(&mut h, r.center());
    // Turned 90°, the right-hand handle sits below the centre: dragging it down widens the shape.
    let right = gridcraft_ui_egui::frame::Handle { x: 1, y: 0 }.at_turned(r, 90.0);
    drag(&mut h, right, right + vec2(0.0, 40.0));
    let after = rect(&h, id);
    assert!(after.width() > r.width() + 30.0 && (after.height() - r.height()).abs() < 1.0, "{r:?} -> {after:?}");
    assert_eq!(rotation(&h, id), 90.0);
}
