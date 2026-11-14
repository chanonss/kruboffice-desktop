use super::arrange::arrange_group;
use super::galleries::cref_param;
use super::{
    Color32, CornerRadius, Icon, Rect, Sense, ShapeKind, SlideApp, Stroke, Ui, Value, big, big_button, big_button_ex, color_split, group, json,
    menu_item, pos2, rows, run, scheme, small, theme, toggle, vec2, widgets,
};

pub(super) fn table_design(app: &mut SlideApp, ui: &mut Ui) {
    let tbl = app
        .session
        .active()
        .and_then(|d| d.selected_shapes().into_iter().find_map(|s| if let ShapeKind::Table(t) = &s.kind { Some(t.clone()) } else { None }));
    group(ui, |ui| {
        rows(ui, |ui| {
            let flags = tbl.as_ref().map(|t| (t.first_row, t.last_row, t.band_row, t.first_col, t.last_col, t.band_col)).unwrap_or_default();
            // Two rows, as the two-size ribbon's: the rows' options, then the columns', in aligned
            // columns with room between them (the Thai labels are long).
            egui::Grid::new("table_style_options").num_columns(3).spacing(egui::vec2(10.0, crate::ribbon_layout::ROW_GAP + 4.0)).show(ui, |ui| {
                for items in [
                    [("Header Row", "headerRow", flags.0), ("Total Row", "totalRow", flags.1), ("Banded Rows", "bandedRows", flags.2)],
                    [("First Column", "firstColumn", flags.3), ("Last Column", "lastColumn", flags.4), ("Banded Columns", "bandedColumns", flags.5)],
                ] {
                    for (label, key, on) in items {
                        let mut v = on;
                        if ui.checkbox(&mut v, tl!(label)).changed() {
                            run(app, "table.options", json!({ key: v }));
                        }
                    }
                    ui.end_row();
                }
            });
        });
    });
    group(ui, |ui| {
        let styles = deckcraft_model::table::table_styles();
        let quick: Vec<_> = styles.iter().filter(|(id, _)| id.starts_with("medium2") || id == "none" || id == "grid").take(8).collect();
        let cur = tbl.as_ref().map(|t| t.style.clone()).unwrap_or_default();
        let sel = quick.iter().position(|(id, _)| *id == cur);
        let sc = scheme(app);
        if let Some(i) = widgets::gallery(
            ui,
            quick.len(),
            vec2(48.0, 40.0),
            sel,
            |p, r, i| {
                if let Some((id, _)) = quick.get(i) {
                    paint_table_style(p, r, id, &sc);
                }
            },
            |i| quick.get(i).map(|x| x.1.clone()).unwrap_or_default(),
        ) && let Some((id, _)) = quick.get(i)
        {
            run(app, "table.style", json!({"style": id}));
        }
        let more = widgets::drop_button(ui, "", vec2(14.0, 40.0), true);
        egui::Popup::menu(&more).width(360.0).show(|ui| {
            ui.horizontal_wrapped(|ui| {
                for (id, label) in &styles {
                    let (r, resp) = ui.allocate_exact_size(vec2(48.0, 36.0), Sense::click());
                    paint_table_style(ui.painter(), r.shrink(2.0), id, &sc);
                    if resp.on_hover_text(label).clicked() {
                        run(app, "table.style", json!({"style": id}));
                        ui.close();
                    }
                }
            });
        });
    });
    group(ui, |ui| {
        let sc = scheme(app);
        let (m, a) = color_split(ui, Icon::Shading, tl!("Shading"), Color32::from_rgb(0x2E, 0x6F, 0xD8), true);
        if m.clicked() {
            run(app, "table.cellFill", json!({"color": "accent1"}));
        }
        widgets::color_popup(&a).show(|ui| {
            if let Some(c) = widgets::color_grid(ui, &sc, Some("No Fill")) {
                match c {
                    Some(c) => run(app, "table.cellFill", json!({"color": cref_param(&c)})),
                    None => run(app, "table.cellFill", json!({"none": true})),
                }
            }
        });
    });
    group(ui, |ui| {
        let r = big_button(ui, Icon::Borders, tl!("Borders"), true);
        egui::Popup::menu(&r).show(|ui| {
            let pen = app.table_pen.clone();
            // KhianKrub's order and pictures: the sides, then none/all/outside/inside, then the
            // inside lines and diagonals.
            let groups: [&[(&str, &str)]; 3] = [
                &[("Bottom Border", "bottom"), ("Top Border", "top"), ("Left Border", "left"), ("Right Border", "right")],
                &[("No Border", "none"), ("All Borders", "all"), ("Outside Borders", "outside"), ("Inside Borders", "inside")],
                &[
                    ("Inside Horizontal Border", "insideH"),
                    ("Inside Vertical Border", "insideV"),
                    ("Diagonal Down Border", "diagDown"),
                    ("Diagonal Up Border", "diagUp"),
                ],
            ];
            for (i, items) in groups.iter().enumerate() {
                if i > 0 {
                    ui.separator();
                }
                for (label, which) in items.iter() {
                    let mut p = pen.clone();
                    p["which"] = json!(which);
                    border_item(app, ui, label, which, p);
                }
            }
            ui.separator();
            ui.menu_button(tl!("Pen Weight"), |ui| {
                for w in [0.25, 0.5, 0.75, 1.0, 1.5, 2.25, 3.0, 4.5, 6.0] {
                    let cur = app.table_pen.get("width").and_then(Value::as_f64) == Some(w);
                    if ui.radio(cur, format!("{w} pt")).clicked() {
                        app.table_pen["width"] = json!(w);
                    }
                }
            });
            ui.menu_button(tl!("Pen Style"), |ui| {
                for (l, d) in [("Solid", "solid"), ("Dash", "dash"), ("Dot", "sysDot"), ("Dash Dot", "dashDot"), ("Long Dash", "lgDash")] {
                    let cur = app.table_pen.get("dash").and_then(Value::as_str).unwrap_or("solid") == d;
                    if ui.radio(cur, l).clicked() {
                        app.table_pen["dash"] = json!(d);
                    }
                }
            });
            ui.menu_button(tl!("Pen Color"), |ui| {
                let sc = scheme(app);
                if let Some(Some(c)) = widgets::color_grid(ui, &sc, None) {
                    app.table_pen["color"] = cref_param(&c);
                }
            });
        });
    });
    // Draw Borders: the pencil splits cells along a line, the eraser merges across a border.
    group(ui, |ui| {
        use deckcraft_engine::ToolKind;
        use deckcraft_engine::tools::{DRAW_TABLE, TABLE_ERASER};
        let cur = app.session.tool.kind.clone();
        for (icon, label, preset) in [(Icon::Pencil, "Draw Table", DRAW_TABLE), (Icon::Eraser, "Table Eraser", TABLE_ERASER)] {
            let on = matches!(&cur, ToolKind::Shape { preset: p } if p == preset);
            if big_button_ex(ui, icon, label, true, false, on).0.clicked() {
                app.session.set_tool(if on { ToolKind::Select } else { ToolKind::Shape { preset: preset.into() } });
            }
        }
    });
}

pub fn paint_table_style(p: &egui::Painter, r: Rect, id: &str, sc: &deckcraft_color::ColorScheme) {
    let slot = match id.rsplit('-').next().unwrap_or("accent1") {
        "accent2" => deckcraft_color::SchemeSlot::Accent2,
        "accent3" => deckcraft_color::SchemeSlot::Accent3,
        "accent4" => deckcraft_color::SchemeSlot::Accent4,
        "accent5" => deckcraft_color::SchemeSlot::Accent5,
        "accent6" => deckcraft_color::SchemeSlot::Accent6,
        "tx1" => deckcraft_color::SchemeSlot::Dk1,
        _ => deckcraft_color::SchemeSlot::Accent1,
    };
    let a = theme::to_color32(sc.get(slot));
    let kind = id.split('-').next().unwrap_or("");
    for row in 0..4 {
        let y = r.min.y + row as f32 * r.height() / 4.0;
        let rr = Rect::from_min_size(pos2(r.min.x, y), vec2(r.width(), r.height() / 4.0));
        let c = match (kind, row) {
            ("none", _) | ("grid", _) => Color32::WHITE,
            ("light1", 0) | ("light2", _) => Color32::WHITE,
            ("dark1", 0) => Color32::BLACK,
            ("dark1", _) => a.lerp_to_gamma(Color32::BLACK, 0.3),
            (_, 0) => a,
            (_, k) if k % 2 == 1 => a.lerp_to_gamma(Color32::WHITE, 0.6),
            _ => a.lerp_to_gamma(Color32::WHITE, 0.8),
        };
        p.rect_filled(rr, CornerRadius::ZERO, c);
        p.line_segment([rr.left_bottom(), rr.right_bottom()], Stroke::new(0.5, if kind == "grid" { Color32::GRAY } else { Color32::WHITE }));
    }
    if kind == "grid" || kind == "light2" {
        p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(0.8, if kind == "grid" { Color32::GRAY } else { a }), egui::StrokeKind::Inside);
    }
}

pub(super) fn table_layout(app: &mut SlideApp, ui: &mut Ui) {
    group(ui, |ui| {
        let r = big_button(ui, Icon::DeleteTable, tl!("Delete"), true);
        egui::Popup::menu(&r).show(|ui| {
            menu_item(app, ui, Icon::DeleteTable, tl!("Delete Columns"), "table.deleteColumn", json!({}));
            menu_item(app, ui, Icon::DeleteTable, tl!("Delete Rows"), "table.deleteRow", json!({}));
            menu_item(app, ui, Icon::DeleteTable, tl!("Delete Table"), "edit.delete", json!({}));
        });
        big(app, ui, Icon::InsertAbove, tl!("Insert\nAbove"), "table.insertRowAbove", json!({}));
        big(app, ui, Icon::InsertBelow, tl!("Insert\nBelow"), "table.insertRowBelow", json!({}));
        big(app, ui, Icon::InsertLeft, tl!("Insert\nLeft"), "table.insertColumnLeft", json!({}));
        big(app, ui, Icon::InsertRight, tl!("Insert\nRight"), "table.insertColumnRight", json!({}));
    });
    group(ui, |ui| {
        big(app, ui, Icon::MergeCells, tl!("Merge\nCells"), "table.merge", json!({}));
        big(app, ui, Icon::SplitCells, tl!("Split\nCells"), "table.split", json!({}));
    });
    group(ui, |ui| {
        rows(ui, |ui| {
            small(app, ui, Icon::DistributeV, tl!("Distribute Rows"), "table.distributeRows", json!({}));
            small(app, ui, Icon::DistributeH, tl!("Distribute Columns"), "table.distributeColumns", json!({}));
        });
    });
    group(ui, |ui| {
        rows(ui, |ui| {
            ui.horizontal(|ui| {
                toggle(app, ui, Icon::AlignLeft, tl!("Align Left"), "format.alignLeft", false);
                toggle(app, ui, Icon::AlignCenter, tl!("Center"), "format.alignCenter", false);
                toggle(app, ui, Icon::AlignRight, tl!("Align Right"), "format.alignRight", false);
            });
            ui.horizontal(|ui| {
                for (l, a, icon) in
                    [("Top", "top", Icon::AlignObjTop), ("Middle", "middle", Icon::AlignObjMiddle), ("Bottom", "bottom", Icon::AlignObjBottom)]
                {
                    let tip = crate::i18n::fmt(tl!("Align text to the {l} of the cell"), &[("l", tl!(l))]);
                    if widgets::icon_toggle(ui, icon, &tip, true, false).clicked() {
                        run(app, "table.cellAlign", json!({"anchor": a}));
                    }
                }
                ui.menu_button(tl!("Cell Margins"), |ui| {
                    for (l, preset) in [("Normal", "normal"), ("None", "none"), ("Narrow", "narrow"), ("Wide", "wide")] {
                        if ui.button(tl!(l)).clicked() {
                            run(app, "table.cellMargins", json!({"preset": preset}));
                            ui.close();
                        }
                    }
                });
            });
        });
    });
    arrange_group(app, ui);
}

/// A Borders menu item with KhianKrub's picture of the borders it sets: a 2 × 2 table whose set
/// borders are solid and the rest dotted (the shared `krub-icons` `tb_*` pictures).
fn border_item(app: &mut SlideApp, ui: &mut Ui, label: &str, which: &str, p: Value) {
    let en = super::enabled(app, "table.borders");
    let atom = egui::IdSalt::new(("border picture", which));
    let resp = ui
        .add_enabled_ui(en, |ui| egui::Button::new((egui::Atom::custom(atom, vec2(24.0, 20.0)), tl!(label))).min_size(vec2(220.0, 28.0)).atom_ui(ui))
        .inner;
    if let Some(r) = resp.rect(atom) {
        let t = theme::Tokens::of(ui);
        let (ink, accent) = if en { (t.text, t.accent) } else { (t.text_faint, t.text_faint) };
        krub_icons::menu::paint(ui.painter(), r, &format!("tb_{which}"), ink, accent);
    }
    if resp.response.clicked() {
        run(app, "table.borders", p);
        ui.close();
    }
}
