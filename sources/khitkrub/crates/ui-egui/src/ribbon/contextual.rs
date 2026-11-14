use super::controls::{act, anchor_keytip, control_keytip, keytip, menu_items, sep};
use super::galleries::{paint_table_swatch, table_gallery};
use super::{Color32, Icon, Sense, SheetApp, Stroke, StrokeKind, Tokens, Ui, big_button, icon_button, json, keytip_table, vec2};

/// Draws the Table Design tab: rename, resize, remove duplicates, convert to range, slicer, table options and the style strip.
pub(super) fn table_design(app: &mut SheetApp, ui: &mut Ui) {
    let Some((tname, header, totals, banded, banded_c, first, last, filter, style)) = app.session.active().and_then(|d| {
        let sh = d.wb.active()?;
        let t = sh.table_at(d.selection.active)?;
        Some((t.name.clone(), t.header_row, t.totals_row, t.banded_rows, t.banded_cols, t.first_col, t.last_col, t.filter_button, t.style.clone()))
    }) else {
        return;
    };
    crate::ribbon_layout::column(ui, |ui| {
        ui.label(egui::RichText::new("Table Name:").small());
        let field = egui::Id::unique(("gridcraft.table_name", &tname));
        if let Some(name) = crate::widgets::committed_text(ui, field, &tname, Some(120.0))
            && name != tname
        {
            act(app, "table.rename", json!({"table": tname, "name": name}));
        }
    });
    let __k = big_button(ui, Icon::Table, "Resize\nTable", "Resize Table", false);
    control_keytip(app, ui, &__k, "dialog:tableResize");
    if __k.clicked() {
        app.open_dialog("tableResize", json!({}));
    }
    sep(ui);
    let __k = big_button(ui, Icon::Duplicates, "Remove\nDuplicates", "Remove Duplicates", false);
    control_keytip(app, ui, &__k, "data.removeDuplicates");
    if __k.clicked() {
        act(app, "data.removeDuplicates", json!({}));
    }
    let __k = big_button(ui, Icon::Table, "Convert\nto Range", "Convert to Range", false);
    control_keytip(app, ui, &__k, "table.convertToRange");
    if __k.clicked() {
        act(app, "table.convertToRange", json!({"table": tname}));
    }
    sep(ui);
    if big_button(ui, Icon::Filter, "Insert\nSlicer", "Insert Slicer", false).clicked() {
        act(app, "table.insertSlicer", json!({"table": tname}));
    }
    sep(ui);
    // Two rows (two-size ribbon): the rows' options, then the columns'.
    crate::ribbon_layout::column(ui, |ui| {
        for (row, items) in [
            [
                ("Header Row", "table.headerRow", header && filter),
                ("Total Row", "table.totalRow", totals),
                ("Banded Rows", "table.bandedRows", banded),
            ],
            [
                ("First Column", "table.firstColumn", first),
                ("Last Column", "table.lastColumn", last),
                ("Banded Columns", "table.bandedColumns", banded_c),
            ],
        ]
        .into_iter()
        .enumerate()
        {
            if row > 0 {
                ui.add_space(crate::ribbon_layout::ROW_GAP + 4.0);
            }
            ui.horizontal(|ui| {
                for (label, cmd, v) in items {
                    let mut on = v;
                    if ui.checkbox(&mut on, label).changed() {
                        act(app, cmd, json!({"table": tname, "on": on}));
                    }
                }
            });
        }
    });
    sep(ui);
    table_style_strip(app, ui, &tname, &style);
}

/// The first page of Excel's Table Styles gallery, drawn in the ribbon. The More arrow opens the
/// full gallery (Light, Medium, Dark) as a popup.
const RIBBON_TABLE_STYLES: [&str; 7] =
    ["TableStyleLight1", "TableStyleLight2", "TableStyleLight3", "TableStyleLight4", "TableStyleLight5", "TableStyleLight6", "TableStyleLight7"];

fn table_style_strip(app: &mut SheetApp, ui: &mut Ui, tname: &str, current: &str) {
    let t = Tokens::get(ui.ctx());
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        // Borrow the workbook only while painting; the session must be free for `act` afterwards.
        let clicked = {
            let Some(d) = app.session.active() else { return };
            let wb = &d.wb;
            let mut clicked = None;
            for name in RIBBON_TABLE_STYLES {
                let (r, resp) = ui.allocate_exact_size(vec2(52.0, 40.0), Sense::click());
                paint_table_swatch(ui, r, wb, name);
                if name == current {
                    ui.painter().rect_stroke(r.expand(2.5), 2.0, Stroke::new(2.0, t.accent), StrokeKind::Outside);
                }
                if resp.hovered() {
                    ui.painter().rect_stroke(r.expand(1.0), 2.0, Stroke::new(2.0, Color32::from_rgb(0xE8, 0x7D, 0x2B)), StrokeKind::Outside);
                }
                if resp.on_hover_text(name).clicked() {
                    clicked = Some(name);
                }
            }
            clicked
        };
        if let Some(name) = clicked {
            act(app, "table.style", json!({"table": tname, "style": name}));
        }
        let more = icon_button(ui, Icon::Chevron, t.text_dim, "More Table Styles", vec2(18.0, 40.0));
        anchor_keytip(app, ui, &more, &["table.style"]);
        egui::Popup::menu(&more).show(|ui| table_gallery(app, ui, "table.style"));
    });
}

pub(super) fn chart_design(app: &mut SheetApp, ui: &mut Ui) {
    let Some(id) = app.selected_chart() else { return };
    let ae = big_button(ui, Icon::Chart, "Add Chart\nElement", "Add Chart Element", true);
    for sequence in keytip_table::group_extension(app.keytips.prefix().unwrap_or_default(), &["chart.set"]) {
        keytip(app, ui, sequence, ae.rect.center_bottom());
    }
    egui::Popup::menu(&ae).show(|ui| {
        menu_items(
            app,
            ui,
            &[
                ("Legend: Bottom", "chart.set", json!({"chart": id, "legend": "bottom"})),
                ("Legend: Right", "chart.set", json!({"chart": id, "legend": "right"})),
                ("Legend: Top", "chart.set", json!({"chart": id, "legend": "top"})),
                ("Legend: None", "chart.set", json!({"chart": id, "legend": "none"})),
                ("-", "", json!(null)),
                ("Data Labels: Show", "chart.set", json!({"chart": id, "dataLabels": true})),
                ("Data Labels: None", "chart.set", json!({"chart": id, "dataLabels": false})),
                ("Data Table: With Legend Keys", "chart.addElement", json!({"chart": id, "element": "dataTable", "dataTableKeys": true})),
                ("Data Table: No Legend Keys", "chart.addElement", json!({"chart": id, "element": "dataTable", "dataTableKeys": false})),
                ("Data Table: None", "chart.addElement", json!({"chart": id, "element": "dataTable", "on": false})),
                ("Gridlines: Show", "chart.set", json!({"chart": id, "gridlines": true})),
                ("Gridlines: None", "chart.set", json!({"chart": id, "gridlines": false})),
            ],
        );
    });
    let __k = big_button(ui, Icon::Redo, "Switch\nRow/Column", "Switch Row/Column", false);
    control_keytip(app, ui, &__k, "chart.switchRowColumn");
    if __k.clicked() {
        act(app, "chart.switchRowColumn", json!({"chart": id}));
    }
    if big_button(ui, Icon::Table, "Select\nData", "Select Data", false).clicked() {
        app.open_dialog("selectData", json!({"chart": id}));
    }
    let ct = big_button(ui, Icon::ChartBar, "Change\nChart Type", "Change Chart Type", true);
    for sequence in keytip_table::group_extension(
        app.keytips.prefix().unwrap_or_default(),
        &["column", "bar", "line", "area", "pie", "doughnut", "scatter", "radar", "waterfall", "funnel", "treemap"],
    ) {
        keytip(app, ui, sequence, ct.rect.center_bottom());
    }
    egui::Popup::menu(&ct).show(|ui| {
        for (label, k, s) in [
            ("Clustered Column", "column", ""),
            ("Stacked Column", "column", "stacked"),
            ("Bar", "bar", ""),
            ("Line", "line", "markers"),
            ("Area", "area", ""),
            ("Pie", "pie", ""),
            ("Doughnut", "doughnut", ""),
            ("Scatter", "scatter", ""),
            ("Radar", "radar", ""),
            ("Waterfall", "waterfall", ""),
            ("Funnel", "funnel", ""),
            ("Treemap", "treemap", ""),
        ] {
            if ui.button(label).clicked() {
                act(app, "chart.set", json!({"chart": id, "type": k, "subtype": s}));
            }
        }
    });
    if big_button(ui, Icon::Settings, "Format\nPane", "Format Chart pane", false).clicked() {
        app.grid.pane = Some("formatChart".into());
    }
    let __k = big_button(ui, Icon::TextBox, "Chart\nTitle", "Edit chart title", false);
    control_keytip(app, ui, &__k, "chartTitle");
    if __k.clicked() {
        app.open_dialog("chartTitle", json!({"chart": id}));
    }
    sep(ui);
    size_group(app, ui);
    sep(ui);
    let __k = big_button(ui, Icon::Delete, "Delete\nChart", "Delete chart", false);
    control_keytip(app, ui, &__k, "chart.delete");
    if __k.clicked() {
        act(app, "chart.delete", json!({"chart": id}));
        app.selected_object = None;
    }
}

/// Sheet units are 96-dpi pixels; an inch is this many.
const PX_PER_INCH: f64 = 96.0;

/// Whether sizes show in centimetres (every language but English, as on the rulers).
fn metric_sizes(app: &SheetApp) -> bool {
    !matches!(app.ui.language, crate::i18n::Language::En)
}

/// The Size group: Height and Width boxes for the selected floating object, in inches or
/// centimetres. Typing a size sends `object.move` with the new size in sheet pixels.
pub(super) fn size_group(app: &mut SheetApp, ui: &mut Ui) {
    let Some(o) = app.selected_object else { return };
    let Some(a) = app.session.active().and_then(|d| d.wb.active()).and_then(|sh| sh.object_anchor(o)) else { return };
    let metric = metric_sizes(app);
    let per = if metric { PX_PER_INCH / 2.54 } else { PX_PER_INCH };
    let (suffix, kind) = (
        if metric { " cm" } else { " in" },
        match o.kind {
            gridcraft_engine::model::ObjKind::Chart => "chart",
            gridcraft_engine::model::ObjKind::Image => "image",
            gridcraft_engine::model::ObjKind::Slicer => "slicer",
            _ => "shape",
        },
    );
    crate::ribbon_layout::column(ui, |ui| {
        for (label, cur, key) in [("Height:", a.height, "height"), ("Width:", a.width, "width")] {
            let mut v = f64::from(cur) / per;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(app.ui.language.tr(label)).small());
                let r = ui.add(egui::DragValue::new(&mut v).speed(0.01).range(0.01..=10000.0 / per).fixed_decimals(2).suffix(suffix));
                if r.changed() {
                    act(app, "object.move", json!({"kind": kind, "id": o.id, key: v * per}));
                }
            });
        }
    });
}

/// Picture Format and Shape Format: the Size group (more groups can join it here).
pub(super) fn object_format(app: &mut SheetApp, ui: &mut Ui) {
    size_group(app, ui);
}
