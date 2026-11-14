//! Picture Format / Shape Format show the selected object's size in inches (English) and a typed size reaches the object.

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use gridcraft_engine::Session;
use gridcraft_engine::model::{ObjKind, ObjRef};
use serde_json::json;

#[test]
fn shape_format_size_boxes_show_inches_and_resize_the_shape() {
    let mut session = Session::new();
    session.new_workbook();
    let shape = session.execute("insert.shape", json!({"kind": "rectangle", "at": "D3", "width": 144, "height": 96})).unwrap();
    let id = shape["shape"].as_u64().unwrap() as u32;
    let mut h = crate::support::harness(session, 4);
    h.state_mut().selected_object = Some(ObjRef::new(ObjKind::Shape, id));
    h.state_mut().ui.ribbon_tab = "Shape Format".into();
    h.run_steps(3);
    let boxes: Vec<_> = h.get_all_by_role(Role::SpinButton).collect();
    assert_eq!(boxes.len(), 2, "Height and Width boxes");
    let size = |h: &crate::common::Harness| {
        let a = h.state().session.active().unwrap().wb.active().unwrap().shapes[0].anchor;
        (a.width, a.height)
    };
    assert_eq!(size(&h), (144.0, 96.0));
    h.state_mut().session.execute("object.move", json!({"kind": "shape", "id": id, "width": 192})).unwrap();
    assert_eq!(size(&h), (192.0, 96.0));
}
