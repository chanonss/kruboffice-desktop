//! Standard DrawingML shapes (owner, 2026-11-19: the shared Insert › Shapes menu) are saved by
//! their own names and read back as themselves; a block arrow is no longer read as a line.

use std::sync::Arc;

use gridcraft_model::{Anchor, AnchorMode, Color, Shape, ShapeKind, Sheet, Workbook};

use crate::{read_xlsx, write_xlsx};

#[test]
fn standard_shapes_survive_a_save() {
    let names = ["star24", "flowChartDecision", "wedgeRectCallout", "rightArrow", "cloud", "roundRect"];
    let mut sheet = Sheet::new("Shapes");
    for (i, name) in names.iter().enumerate() {
        sheet.shapes.push(Shape {
            id: i as u32 + 1,
            kind: ShapeKind::from_preset(name).unwrap(),
            anchor: Anchor { width: 96.0, height: 96.0, mode: AnchorMode::Absolute, ..Default::default() },
            fill: Color::Rgb(0x4472C4),
            line: Color::Rgb(0x2F528F),
            text: String::new(),
            format: Default::default(),
            math: Default::default(),
        });
    }
    let wb = Workbook { sheets: vec![Arc::new(sheet)], ..Default::default() };
    let (back, _) = read_xlsx(&write_xlsx(&wb).unwrap()).unwrap();
    let kinds: Vec<ShapeKind> = back.sheets[0].shapes.iter().map(|s| s.kind).collect();
    let want: Vec<ShapeKind> = names.iter().map(|n| ShapeKind::from_preset(n).unwrap()).collect();
    assert_eq!(kinds, want);
    assert!(matches!(kinds[3], ShapeKind::Preset(id) if id.name() == "rightArrow"), "a block arrow stays a block arrow");
    assert_eq!(kinds[5], ShapeKind::RoundedRectangle, "KhitKrub's own kinds stay themselves");
}

/// A rounded rectangle's own corner (`roundRect`'s `adj`) is written and read back; without one
/// the file has none (Excel's default), and a file's out-of-range value is kept within round ends.
#[test]
fn rounded_rectangle_corners_survive_a_save() {
    let mut sheet = Sheet::new("Corners");
    for (i, corner) in [Some(0), Some(8_000), Some(50_000), None].into_iter().enumerate() {
        sheet.shapes.push(Shape {
            id: i as u32 + 1,
            kind: ShapeKind::RoundedRectangle,
            anchor: Anchor { width: 120.0, height: 60.0, mode: AnchorMode::Absolute, ..Default::default() },
            fill: Color::Rgb(0x4472C4),
            line: Color::Rgb(0x2F528F),
            text: String::new(),
            format: gridcraft_model::ShapeFormat { corner, ..Default::default() },
            math: Default::default(),
        });
    }
    let wb = Workbook { sheets: vec![Arc::new(sheet)], ..Default::default() };
    let bytes = write_xlsx(&wb).unwrap();
    let (back, _) = read_xlsx(&bytes).unwrap();
    let corners: Vec<Option<u32>> = back.sheets[0].shapes.iter().map(|s| s.format.corner).collect();
    assert_eq!(corners, [Some(0), Some(8_000), Some(50_000), None]);
}
