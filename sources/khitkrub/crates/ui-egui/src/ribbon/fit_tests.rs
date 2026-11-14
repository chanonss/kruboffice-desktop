//! The two-size ribbon (owner, 2026-11-13; KhianKrub is the standard): every group fits in two
//! rows of small buttons, on every tab and every tab that comes with an object.
//!
//! English only: the strict test font set has no Thai faces without `CRAFT_FONTS_DIR`.

use gridcraft_engine::Session;
use gridcraft_engine::model::{ObjKind, ObjRef};
use serde_json::json;

use crate::{Services, SheetApp};

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn harness(session: Session) -> Harness {
    let mut app = SheetApp::new(session, Services::default());
    app.use_prepared_fonts();
    // Wide enough that no tab hides groups behind its scroll arrows.
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(3200.0, 900.0))
        .with_context_setup(|ctx| {
            ctx.set_fonts(crate::theme::pinned_font_definitions());
            crate::theme::apply(ctx, false);
        })
        .build_ui_state(
            |ui, app: &mut SheetApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
    h.run_steps(3);
    h
}

fn tall(h: &mut Harness, tab: &str) -> Vec<String> {
    h.state_mut().ui.ribbon_tab = tab.into();
    h.state_mut().ui.show_draw_tab = true;
    h.run_steps(2);
    crate::ribbon_layout::TALL_GROUPS.with(|g| g.borrow_mut().clear());
    h.run_steps(1);
    crate::ribbon_layout::TALL_GROUPS.with(|g| g.take()).into_iter().map(|g| format!("{tab} › {g}")).collect()
}

/// A 1×1 PNG.
fn png() -> String {
    const HEX: &str =
        "89504e470d0a1a0a0000000d4948445200000001000000010802000000907753de0000000c49444154789c63f8cfc0000003010100c9fe92ef0000000049454e44ae426082";
    let bytes: Vec<u8> = (0..HEX.len()).step_by(2).map(|i| u8::from_str_radix(&HEX[i..i + 2], 16).unwrap()).collect();
    gridcraft_engine::io::base64_encode(&bytes)
}

#[test]
fn ribbon_groups_fit_in_two_rows() {
    let mut found = Vec::new();
    let mut session = Session::new();
    session.new_workbook();
    let mut h = harness(session);
    for tab in super::TABS {
        found.extend(tall(&mut h, tab));
    }
    // Table Design: the caret in a table.
    let mut s = Session::new();
    s.new_workbook();
    s.execute("insert.table", json!({"range": "A1:C4", "header": true})).unwrap();
    s.execute("selection.set", json!({"range": "B2"})).unwrap();
    let mut h = harness(s);
    found.extend(tall(&mut h, "Table Design"));
    // Chart Design: a chart, selected.
    let mut s = Session::new();
    s.new_workbook();
    s.execute("cell.set", json!({"cell": "A1", "input": "1"})).unwrap();
    s.execute("cell.set", json!({"cell": "A2", "input": "2"})).unwrap();
    s.execute("selection.set", json!({"range": "A1:A2"})).unwrap();
    s.execute("insert.chart", json!({"type": "column", "at": "D3", "width": 300, "height": 200})).unwrap();
    let mut h = harness(s);
    found.extend(tall(&mut h, "Chart Design"));
    // Picture Format and Shape Format: the object selected.
    for (cmd, params, key, kind, tab) in [
        ("insert.picture", json!({"base64": png(), "at": "D3", "width": 60, "height": 60}), "image", ObjKind::Image, "Picture Format"),
        ("insert.shape", json!({"kind": "rectangle", "at": "D3", "width": 144, "height": 96}), "shape", ObjKind::Shape, "Shape Format"),
    ] {
        let mut s = Session::new();
        s.new_workbook();
        let r = s.execute(cmd, params).unwrap();
        let id = r[key].as_u64().unwrap() as u32;
        let mut h = harness(s);
        h.state_mut().selected_object = Some(ObjRef::new(kind, id));
        found.extend(tall(&mut h, tab));
    }
    found.sort();
    found.dedup();
    assert!(found.is_empty(), "groups taller than the two rows: {found:#?}");
}
