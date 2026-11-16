//! The right-hand dock (owner, 2026-11-16): a selected table, picture, chart, SmartArt graphic or
//! shape opens its pane instead of a contextual tab, one pane at a time with a strip of buttons;
//! an open task pane stays in front; the slide keeps the keyboard.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use deckcraft_engine::Session;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::json;

use crate::{Services, SlideApp};

fn harness(s: Session) -> Harness<'static, SlideApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SlideApp::new(s, Services::default()),
    );
    h.run_steps(4);
    h
}

fn blank() -> Session {
    let mut s = Session::with_new();
    s.execute("slide.layout", &json!({"layout": "blank"})).unwrap();
    s
}

/// Each object opens its own pane; none of them adds a tab.
#[test]
fn objects_open_their_panes_instead_of_tabs() {
    for (cmd, params, pane) in [
        ("insert.table", json!({"rows": 2, "cols": 2, "rect": [100, 150, 500, 200]}), "obj:table"),
        ("insert.chart", json!({"type": "column", "rect": [80, 140, 600, 300]}), "obj:chart"),
        ("insert.smartArt", json!({"kind": "process"}), "obj:smartart"),
        ("shape.insert", json!({"preset": "rect", "rect": [80.0, 80.0, 200.0, 120.0]}), "obj:shape"),
    ] {
        let mut s = blank();
        let id = s.execute(cmd, &params).unwrap().get("id").and_then(serde_json::Value::as_u64);
        if let Some(id) = id {
            s.execute("edit.select", &json!({"ids": [id]})).unwrap();
        }
        let h = harness(s);
        assert_eq!(crate::panes::active_pane(&h.ctx), Some(pane), "{cmd}");
        for tab in ["Shape Format", "Picture Format", "Table Design", "Layout", "Chart Design", "SmartArt Design"] {
            assert!(!crate::ribbon::contextual_tabs_for_tests(h.state()).contains(&tab), "{cmd} still shows {tab}");
        }
    }
}

/// With the Selection pane open, choosing a shape doesn't cover it: the Shape pane waits in the
/// strip, and its strip button brings it forward. Closing it shows the Selection pane again.
#[test]
fn an_open_task_pane_stays_in_front() {
    let mut s = blank();
    let id = s.execute("shape.insert", &json!({"preset": "rect", "rect": [80.0, 80.0, 200.0, 120.0]})).unwrap()["id"].as_u64().unwrap();
    s.execute("edit.select", &json!({"ids": []})).unwrap();
    let mut h = harness(s);
    h.state_mut().run("view.pane", json!({"pane": "selection"})).unwrap();
    h.run_steps(3);
    h.state_mut().session.execute("edit.select", &json!({"ids": [id]})).unwrap();
    h.run_steps(3);
    assert_eq!(crate::panes::active_pane(&h.ctx), Some("selection"), "the Selection pane stays in front");
    h.get_by_label("Pane: Shape").click();
    h.run_steps(3);
    assert_eq!(crate::panes::active_pane(&h.ctx), Some("obj:shape"));
    crate::panes::close_object_pane(&h.ctx, "shape");
    h.run_steps(3);
    assert_eq!(crate::panes::active_pane(&h.ctx), Some("selection"));
}

/// The pane opening when a shape is chosen doesn't take the keyboard from the slide: Delete
/// right after removes the shape.
#[test]
fn the_slide_keeps_the_keyboard_when_a_pane_opens() {
    let mut s = blank();
    s.execute("shape.insert", &json!({"preset": "rect", "rect": [300.0, 200.0, 200.0, 120.0]})).unwrap();
    s.execute("edit.select", &json!({"ids": []})).unwrap();
    let mut h = harness(s);
    let count = |h: &Harness<'static, SlideApp>| {
        let st = h.state().session.active().unwrap();
        st.doc.slides[st.selection.slide].shapes.len()
    };
    let before = count(&h);
    let (slide, k) = (h.state().slide_rect.unwrap(), h.state().canvas_scale);
    let at = slide.min + egui::vec2(400.0 * k, 260.0 * k);
    h.hover_at(at);
    h.run_steps(1);
    h.drag_at(at);
    h.run_steps(1);
    h.drop_at(at);
    h.run_steps(3);
    assert_eq!(crate::panes::active_pane(&h.ctx), Some("obj:shape"), "choosing the shape opened its pane");
    h.key_press(egui::Key::Delete);
    h.run_steps(3);
    assert_eq!(count(&h), before - 1, "Delete removed the shape");
}

/// The panes have named, folding sections, and their buttons show their names beside the icon
/// (the ribbon's small buttons are icon-only).
#[test]
fn panes_have_named_sections_and_labelled_buttons() {
    let mut s = blank();
    s.execute("insert.table", &json!({"rows": 2, "cols": 2, "rect": [100, 150, 500, 200]})).unwrap();
    let h = harness(s);
    assert_eq!(crate::panes::active_pane(&h.ctx), Some("obj:table"));
    for section in ["Table Style Options", "Table Styles", "Rows & Columns", "Merge", "Cell Size", "Alignment"] {
        assert!(h.query_by_label(section).is_some(), "the {section} section");
    }
    let below = h.get_by_label("Insert Below").rect();
    assert!(below.width() > crate::ribbon_layout::SMALL_W + 20.0, "Insert Below shows its name: {below:?}");
    assert!(below.height() <= crate::ribbon_layout::SMALL_H + 1.0, "a one-line button: {below:?}");
}

/// Once a pane has been open the dock stays (owner, 2026-11-16): deselecting shows a placeholder
/// at the same width, so the slide doesn't move as the selection comes and goes; its × closes
/// the dock until something is chosen again.
#[test]
fn the_dock_stays_open_so_the_slide_keeps_still() {
    let mut s = blank();
    let id = s.execute("shape.insert", &json!({"preset": "rect", "rect": [80.0, 80.0, 200.0, 120.0]})).unwrap()["id"].as_u64().unwrap();
    let mut h = harness(s);
    assert_eq!(crate::panes::active_pane(&h.ctx), Some("obj:shape"));
    let slide = h.state().slide_rect.unwrap();
    h.state_mut().session.execute("edit.select", &json!({"ids": []})).unwrap();
    h.run_steps(3);
    assert_eq!(crate::panes::active_pane(&h.ctx), Some("obj:idle"), "nothing selected: the placeholder");
    assert_eq!(h.state().slide_rect.unwrap(), slide, "the slide stayed where it was");
    h.state_mut().session.execute("edit.select", &json!({"ids": [id]})).unwrap();
    h.run_steps(3);
    assert_eq!(crate::panes::active_pane(&h.ctx), Some("obj:shape"));
    assert_eq!(h.state().slide_rect.unwrap(), slide);
    // The placeholder's × closes the dock.
    h.state_mut().session.execute("edit.select", &json!({"ids": []})).unwrap();
    h.run_steps(3);
    h.get_by_label("Close").click();
    h.run_steps(3);
    assert_eq!(crate::panes::active_pane(&h.ctx), None, "closed");
}

/// The Shape pane has the layout shared by the three apps (owner, 2026-11-16): Area, Line,
/// Effect, Position and Size, in that order, before PhutKrub's own sections.
#[test]
fn the_shape_pane_has_the_shared_sections_in_order() {
    let mut s = blank();
    s.execute("shape.insert", &json!({"preset": "rect", "rect": [80.0, 80.0, 200.0, 120.0]})).unwrap();
    let h = harness(s);
    assert_eq!(crate::panes::active_pane(&h.ctx), Some("obj:shape"));
    let top = |label: &str| h.get_by_label(label).rect().top();
    let order = ["Area", "Line", "Effect", "Position and Size", "Insert Shapes"].map(top);
    assert!(order.windows(2).all(|w| w[0] < w[1]), "sections out of order: {order:?}");
    for row in ["Fill:", "Line style:", "Thickness:", "Width:", "Height:", "Rotation:", "Flip:", "Align:", "Arrange:"] {
        assert!(h.query_by_label(row).is_some(), "the {row} row");
    }
}
