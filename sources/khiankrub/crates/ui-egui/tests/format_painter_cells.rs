//! The Format Painter from a numbered table cell, through the pointer as a person uses it: click
//! in the cell, press Format Painter, drag across the text to paint.

use egui::{Event, PointerButton, Pos2};
use serde_json::json;
use wordcraft_doc::props::NumRef;
use wordcraft_doc::{Path, Pos, StoryRef};
use wordcraft_engine::{Selection, Session};
use wordcraft_ui_egui::{Services, WordApp};

type H = egui_kittest::Harness<'static, WordApp>;

fn steps(h: &mut H, n: usize) {
    for _ in 0..n {
        h.step();
    }
}

fn screen(h: &mut H, pos: &Pos) -> Pos2 {
    let c = h.state_mut().session.layout().caret(pos).expect("caret");
    wordcraft_ui_egui::canvas::page_to_screen(h.state_mut(), c.page, c.x, c.top + c.height / 2.0).expect("on screen")
}

fn press(h: &mut H, p: Pos2, pressed: bool) {
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Default::default() });
}

fn drag(h: &mut H, a: Pos2, b: Pos2) {
    h.event(Event::PointerMoved(a));
    press(h, a, true);
    h.step();
    for i in 1..=8 {
        h.event(Event::PointerMoved(a + (b - a) * (i as f32 / 8.0)));
        h.step();
    }
    press(h, b, false);
    steps(h, 3);
}

/// A numbered paragraph in the first cell, other text in the cell below-right and after the table.
fn window() -> (H, Pos, Pos, Pos) {
    let mut s = Session::new(wordcraft_doc::Document::new());
    s.run("insert.table", &json!({"rows": 2, "cols": 2})).unwrap();
    let tp = s.doc.body.iter().position(|b| b.as_table().is_some()).unwrap() as u32;
    let cell = |r, c| Pos { story: StoryRef::Body, path: Path(vec![tp, r, c, 0]), off: 0 };
    let (source, other, body) = (cell(0, 0), cell(1, 1), Pos { story: StoryRef::Body, path: Path(vec![tp + 1]), off: 0 });
    for (at, text) in [(&source, "cell source"), (&other, "other cell"), (&body, "body text here")] {
        s.sel = Selection::caret(at.clone());
        s.run("text.insert", &json!({"text": text})).unwrap();
    }
    s.sel = Selection::caret(source.clone());
    s.run("para.numbering", &json!({})).unwrap();
    let mut app = WordApp::new(s, Services::default());
    app.run("view.commentsPane", json!({"value": false})).unwrap();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1440.0, 900.0)).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, app: &mut WordApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app,
    );
    steps(&mut h, 6);
    (h, source, other, body)
}

fn numbering(h: &H, at: &Pos) -> Option<NumRef> {
    h.state().session.doc.para_at(at).and_then(|p| p.props.numbering)
}

/// Dragging the armed painter across a cell's whole text (which starts in the cell's selection
/// area and takes the cell) paints it, as across body text; it used to leave the painter armed
/// and the cell as it was.
#[test]
fn painting_from_a_numbered_cell_onto_a_cell_and_body_text() {
    for target in ["cell", "body"] {
        let (mut h, source, other, body) = window();
        let want = numbering(&h, &source);
        assert!(want.is_some_and(|n| n.num != 0), "the source cell is numbered");
        let to = if target == "cell" { other } else { body };
        let p = screen(&mut h, &Pos { off: 3, ..source.clone() });
        h.event(Event::PointerMoved(p));
        press(&mut h, p, true);
        h.step();
        press(&mut h, p, false);
        steps(&mut h, 3);
        {
            use egui_kittest::kittest::Queryable;
            h.get_by_label("Format Painter").click();
        }
        steps(&mut h, 3);
        assert!(h.state().session.painter.is_some(), "armed");
        let len = h.state().session.doc.para_at(&to).map_or(0, |p| p.len());
        let (a, b) = (screen(&mut h, &to), screen(&mut h, &Pos { off: len, ..to.clone() }));
        drag(&mut h, a, b);
        assert_eq!(numbering(&h, &to), want, "the {target} takes the cell's numbering");
        assert!(h.state().session.painter.is_none(), "the painter is used up ({target})");
    }
}
