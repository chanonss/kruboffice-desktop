//! The Picture pane's Picture Styles (owner, 2026-11-16: the Picture Format tab became a pane):
//! the styles, Crop and Size fit in the pane, and More Picture Styles opens every style.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use deckcraft_engine::Session;
use deckcraft_engine::cmd::picture_style::STYLES;
use deckcraft_model::{Geom, ShapeId};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::json;

use crate::{Services, SlideApp};

/// A blank slide with one selected picture (so the Picture pane is open), in a `width` px window.
fn picture_format(width: f32) -> (Harness<'static, SlideApp>, ShapeId) {
    let mut s = Session::with_new();
    s.execute("slide.layout", &json!({"layout": "blank"})).unwrap();
    let pixels = (0..4).flat_map(|_| [40, 120, 200, 255]).collect();
    let png = deckcraft_render::Image { width: 2, height: 2, pixels }.to_png();
    let data = deckcraft_engine::cmd::base64_encode(&png);
    let id = s.execute("insert.picture", &json!({"data": data, "name": "a.png", "rect": [200, 100, 400, 250]})).unwrap()["id"].as_u64().unwrap();
    s.execute("edit.select", &json!({"ids": [id]})).unwrap();
    let app = SlideApp::new(s, Services::default());
    let mut h = Harness::builder().with_size(egui::vec2(width, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    (h, ShapeId(u32::try_from(id).unwrap()))
}

/// Picture Styles tiles in the ribbon row (the More menu is closed).
fn inline_styles(h: &Harness<'_, SlideApp>) -> usize {
    STYLES.iter().filter(|(_, label)| h.query_by_label(label).is_some()).count()
}

#[test]
fn crop_and_size_sit_in_the_picture_pane() {
    for width in [1280.0, 1440.0, 1920.0] {
        let (h, _) = picture_format(width);
        assert_eq!(crate::panes::active_pane(&h.ctx), Some("obj:picture"));
        assert!(inline_styles(&h) >= 2, "the pane shows some styles at {width} px");
        for label in ["Crop", "Height value", "Width value"] {
            let r = h.get_by_label(label).rect();
            assert!(r.max.x <= width, "{label} at {r:?} is past the window ({width} px)");
        }
    }
}

#[test]
fn more_picture_styles_lists_every_style_and_applies_the_one_clicked() {
    let (mut h, id) = picture_format(1440.0);
    let (_, last) = STYLES[STYLES.len() - 1];
    assert!(h.query_by_label(last).is_none(), "{last} shows only under More Picture Styles");
    let undo = h.state().session.active().unwrap().history.undo.len();

    h.get_by_label("More Picture Styles").click();
    h.run_steps(2);
    // The menu has every style, also the ones the row shows.
    for (_, label) in STYLES {
        assert!(h.query_all_by_label(label).count() >= 1, "{label} is missing from More Picture Styles");
    }
    h.get_by_label(last).click();
    h.run_steps(2);

    let doc = h.state().session.active().unwrap();
    assert!(matches!(&doc.shape(id).unwrap().geom, Geom::Preset { name, .. } if name == "ellipse"), "Metal Oval makes the picture an oval");
    assert_eq!(doc.history.undo.len(), undo + 1, "one undo step");
    assert!(h.query_by_label(last).is_none(), "the menu closes after a pick");
}
