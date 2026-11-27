//! File › Print's preview draws the page's floating objects, as the printout and the PDF export
//! do. It used to draw only the cells: a sheet holding a picture previewed as a blank page.

use std::io::Cursor;

use egui::{Color32, Rect, Shape, TextureId};
use serde_json::json;

fn png() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(160, 100, image::Rgba([24, 150, 110, 255]));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}

/// The bounds of every textured mesh (a picture) the last frame drew.
fn pictures(h: &crate::common::Harness) -> Vec<Rect> {
    h.output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Mesh(mesh) if mesh.texture_id != TextureId::default() => {
                Some(mesh.vertices.iter().fold(Rect::NOTHING, |r, v| r.union(Rect::from_min_max(v.pos, v.pos))))
            }
            _ => None,
        })
        .collect()
}

/// The rectangles filled in `color` the last frame drew.
fn filled(h: &crate::common::Harness, color: Color32) -> Vec<Rect> {
    h.output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(r) if r.fill == color => Some(r.rect),
            _ => None,
        })
        .collect()
}

#[test]
fn print_preview_shows_pictures_and_shapes_where_the_page_has_them() {
    let mut h = crate::support::blank_harness(4);
    let app = h.state_mut();
    app.run("insert.picture", json!({"base64": gridcraft_engine::io::base64_encode(&png()), "at": "A1", "width": 480, "height": 300})).unwrap();
    app.run("insert.shape", json!({"kind": "rectangle", "at": "A20", "width": 120, "height": 40, "fill": "#C83232"})).unwrap();
    let row_19_top = app.session.active().unwrap().wb.active().unwrap().row_top(19) as f32;
    app.run("ui.backstage", json!({"page": "print"})).unwrap();
    h.run_steps(4);

    let pics = pictures(&h);
    assert_eq!(pics.len(), 1, "the preview draws the picture: {pics:?}");
    let pic = pics[0];
    // In the preview, right of the print settings, and in the picture's own proportions.
    assert!(pic.left() > 300.0, "{pic:?}");
    assert!((pic.width() / pic.height() - 480.0 / 300.0).abs() < 0.02, "{pic:?}");
    let per_px = pic.width() / 480.0;
    assert!(per_px > 0.1 && per_px < 1.0, "a page in miniature: {per_px}");

    let red = filled(&h, Color32::from_rgb(0xC8, 0x32, 0x32));
    assert_eq!(red.len(), 1, "the preview draws the shape: {red:?}");
    let shape = red[0];
    // Both anchored in column A, the shape at row 20: the page keeps their places and sizes.
    assert!((shape.left() - pic.left()).abs() < 0.5, "{shape:?} {pic:?}");
    assert!((shape.top() - pic.top() - row_19_top * per_px).abs() < 0.5, "{shape:?} {pic:?}");
    assert!((shape.width() - 120.0 * per_px).abs() < 0.5 && (shape.height() - 40.0 * per_px).abs() < 0.5, "{shape:?}");
}
