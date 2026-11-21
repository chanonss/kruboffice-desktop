//! The Table pane (owner, 2026-11-15): while the caret is in a table, a side pane on the right
//! holds every table command that used to be on the Table Design and Table Layout tabs, in
//! sections (styles, borders and shading, rows and columns, merge and split, cell size,
//! alignment, data, table). It opens by itself when the caret enters a table and closes when it
//! leaves; its × hides it until the caret leaves the table. It has the shared pane width
//! ([`crate::panes::PANE_W`]). Owner, 2026-11-18: a segmented control splits it into Styles,
//! Borders, and Rows and Columns.

use egui::{Ui, vec2};
use serde_json::json;

use crate::WordApp;
use crate::colors::{Remember, Target};
use crate::ribbon::{border_items, cell_size_boxes, mi, pic_item, table_style_gallery};
use crate::widgets::{labeled_button, small};

const CLOSED: &str = "table_pane_closed";

/// Whether the pane shows: the caret is in a table and the pane wasn't closed in it.
pub fn wanted(app: &WordApp, ctx: &egui::Context) -> bool {
    follow(ctx, CLOSED, app.session.sel.focus.path.cell().is_some())
}

/// Whether a pane that follows the selection shows: while `active` (the caret in a table, a
/// shape selected…) unless its × closed it (`closed`, cleared once `active` ends). While a mouse
/// button is held, and on the frame it is released, it stays as it was, so a drag that changes
/// the selection doesn't move the page under the pointer.
pub(crate) fn follow(ctx: &egui::Context, closed: &str, active: bool) -> bool {
    let (closed, shown) = (egui::Id::new(closed), egui::Id::new((closed, "shown")));
    let held = ctx.input(|i| i.pointer.any_down() || i.pointer.any_released());
    let before = ctx.data(|d| d.get_temp::<bool>(shown).unwrap_or(false));
    let show = if held && before != active {
        before
    } else if active {
        !ctx.data(|d| d.get_temp::<bool>(closed).unwrap_or(false))
    } else {
        ctx.data_mut(|d| d.remove::<bool>(closed));
        false
    };
    ctx.data_mut(|d| d.insert_temp(shown, show));
    show
}

/// Hide the pane until the caret leaves the table (its ×).
pub fn close(ctx: &egui::Context) {
    close_pane(ctx, CLOSED);
}

/// Hide the pane keyed `closed` until what it follows ends ([`follow`]).
pub(crate) fn close_pane(ctx: &egui::Context, closed: &str) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(closed), true));
}

pub fn show(app: &mut WordApp, ui: &mut Ui) {
    if crate::panes::header(ui, "Table") {
        close(ui.ctx());
        return;
    }
    // Three sub-panels (owner, 2026-11-18, draft B), the same segmented control as the shape
    // pane's (`krub_icons::pane_ui`).
    let segments = [
        krub_icons::pane_ui::Segment { icon: "table", label: tl!("Styles").to_string() },
        krub_icons::pane_ui::Segment { icon: "borders", label: tl!("Borders").to_string() },
        krub_icons::pane_ui::Segment { icon: "insertBelow", label: tl!("Rows and Columns").to_string() },
    ];
    let tab = krub_icons::pane_ui::segmented(ui, egui::Id::new("table_pane_tab"), &segments, 0);
    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt(("table_pane_scroll", tab)).show(ui, |ui| match tab {
        0 => {
            section(ui, "Table Styles", |ui| styles(app, ui));
            section(ui, "Table", |ui| table(app, ui));
        }
        1 => section(ui, "Borders and Shading", |ui| borders(app, ui)),
        _ => {
            section(ui, "Rows & Columns", |ui| rows_columns(app, ui));
            section(ui, "Merge and Split", |ui| {
                buttons(ui, |ui| {
                    labeled(ui, app, "merge", "Merge Cells", "table.merge");
                    labeled(ui, app, "splitCells", "Split Cells", "table.split");
                    labeled(ui, app, "splitTable", "Split Table", "table.splitTable");
                });
            });
            section(ui, "Cell Size", |ui| cell_size(app, ui));
            section(ui, "Alignment", |ui| alignment(app, ui));
            section(ui, "Data", |ui| {
                buttons(ui, |ui| {
                    labeled(ui, app, "sort", "Sort", "table.sort");
                    labeled(ui, app, "formula", "Formula", "table.formula");
                    labeled(ui, app, "repeatHeader", "Repeat Header Rows", "table.repeatHeader");
                    labeled(ui, app, "toText", "Convert to Text", "table.toText");
                });
            });
        }
    });
}

/// A section under a shaded heading that folds it, open at first (`krub_icons::pane_ui`).
pub(crate) fn section(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    krub_icons::pane_ui::section(ui, egui::Id::new(("side_pane_section", title)), tl!(title), true, add);
}

/// Buttons that wrap onto as many rows as the pane's width needs.
pub(crate) fn buttons(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        add(ui);
    });
}

/// A labeled button that runs `id` with no parameters.
pub(crate) fn labeled(ui: &mut Ui, app: &mut WordApp, icon: &str, label: &str, id: &str) {
    labeled_button(ui, app, icon, label, id, Some(json!({})), false, false, 0.0);
}

/// A labeled toggle that runs `id`.
pub(crate) fn toggle(ui: &mut Ui, app: &mut WordApp, icon: &str, label: &str, id: &str, on: bool) {
    labeled_button(ui, app, icon, label, id, Some(json!({})), on, false, 0.0);
}

/// A labeled button that opens the menu `menu` (its items run the commands); `id` decides
/// whether it is enabled.
pub(crate) fn menu(ui: &mut Ui, app: &mut WordApp, icon: &str, label: &str, id: &str, menu: impl FnOnce(&mut Ui, &mut WordApp)) {
    let resp = labeled_button(ui, app, icon, label, id, None, false, true, 0.0);
    if !crate::widgets::menu_enabled(app, id) {
        return;
    }
    egui::Popup::menu(&resp).show(|ui| {
        ui.set_min_width(200.0);
        menu(ui, app);
    });
}

fn styles(app: &mut WordApp, ui: &mut Ui) {
    let story = app.session.sel.focus.story;
    let table = app.session.sel.focus.path.cell().and_then(|(tp, _, _)| app.session.doc.table(story, &tp));
    let look = table.map(|t| t.props.look);
    let current_id = table.and_then(|t| t.props.style.clone());
    // The document's own styles first, so a new one shows without scrolling.
    let mut styles: Vec<(bool, String, String)> = app
        .session
        .doc
        .styles
        .styles
        .iter()
        .filter(|s| s.kind == wordcraft_doc::StyleKind::Table && !s.hidden)
        .map(|s| (s.builtin, s.id.clone(), crate::i18n::style_name(s)))
        .collect();
    styles.sort_by_key(|(builtin, _, _)| *builtin);
    let current = current_id.as_ref().and_then(|id| {
        app.session
            .doc
            .styles
            .get(id)
            .filter(|st| st.kind == wordcraft_doc::StyleKind::Table)
            .map(wordcraft_engine::cmd::table_style::is_builtin_table_style)
    });
    // As many tiles as the pane's width holds (a tile and its gap are 65 px; the arrows 22).
    let visible = (((ui.available_width() - 22.0) / 65.0).floor() as usize).max(1);
    ui.horizontal(|ui| table_style_gallery(ui, app, &styles, current_id.as_deref(), current, visible));
    let Some(l) = look else { return };
    ui.add_space(4.0);
    egui::Grid::new("table_pane_look").num_columns(2).spacing(vec2(10.0, 4.0)).show(ui, |ui| {
        for row in [
            [("Header Row", "headerRow", l.header_row), ("First Column", "firstColumn", l.first_column)],
            [("Total Row", "totalRow", l.total_row), ("Last Column", "lastColumn", l.last_column)],
            [("Banded Rows", "bandedRows", l.banded_rows), ("Banded Columns", "bandedColumns", l.banded_columns)],
        ] {
            for (label, key, val) in row {
                let mut v = val;
                if ui.checkbox(&mut v, tl!(label)).changed() {
                    let _ = app.run("table.look", json!({ key: v }));
                }
            }
            ui.end_row();
        }
    });
}

fn borders(app: &mut WordApp, ui: &mut Ui) {
    buttons(ui, |ui| {
        menu(ui, app, "shading", "Shading", "table.shading", |ui, app| {
            mi(ui, app, "No Color", "table.shading", json!({"color": null}));
            let theme = app.session.doc.settings.theme_colors.clone();
            crate::colors::menu(ui, app, &theme, Target::cmd("table.shading").remember(Remember::Shading));
        });
        menu(ui, app, "borders", "Borders", "table.borders", border_items);
    });
    border_pen(app, ui);
}

/// The line styles the pen offers: OOXML name and label.
const PEN_STYLES: [(&str, &str); 6] = [
    ("single", "Solid line"),
    ("dashed", "Dashed line"),
    ("dotted", "Dotted line"),
    ("double", "Double line"),
    ("dotDash", "Dash-dot line"),
    ("thick", "Thick line"),
];
/// The pen's Line style (owner, 2026-11-19: "show the real line, it says more than words"):
/// the box and its menu show each style drawn as a line (`line_looks`).
fn pen_style(app: &mut WordApp, ui: &mut Ui) {
    crate::line_looks::names(ui, egui::Id::new("tp_pen_style"), 120.0, &mut app.border_pen.style, &PEN_STYLES);
}

/// The weights the pen offers, points.
const PEN_WEIGHTS: [f32; 9] = [0.25, 0.5, 0.75, 1.0, 1.5, 2.25, 3.0, 4.5, 6.0];

/// Owner, 2026-11-17: a pen (line style, weight, color) for any of a table's lines. The line
/// buttons and the small table below set the selected cells' edges, or with Whole table the
/// table's own lines; Border Painter paints single lines on the page with it.
fn border_pen(app: &mut WordApp, ui: &mut Ui) {
    let enabled = crate::widgets::enabled(app, "table.borders");
    ui.add_enabled_ui(enabled, |ui| {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(tl!("Pen")).small().weak());
        egui::Grid::new("tp_pen").num_columns(2).spacing(vec2(8.0, 4.0)).show(ui, |ui| {
            ui.label(tl!("Line style:"));
            pen_style(app, ui);
            ui.end_row();
            ui.label(tl!("Line weight:"));
            egui::ComboBox::from_id_salt("tp_pen_width").selected_text(format!("{} pt", app.border_pen.width)).width(120.0).show_ui(ui, |ui| {
                for w in PEN_WEIGHTS {
                    if ui.selectable_label((app.border_pen.width - w).abs() < 1e-3, format!("{w} pt")).clicked() {
                        app.border_pen.width = w;
                    }
                }
            });
            ui.end_row();
            ui.label(tl!("Line color:"));
            pen_color(app, ui);
            ui.end_row();
        });
        // On a line of its own, the two choices sharing the pane's width: beside the label
        // they were wider than the pane and pushed it past its left edge.
        ui.add_space(2.0);
        ui.label(tl!("Apply to:"));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let half = ((ui.available_width() - 4.0) / 2.0).max(0.0);
            for (whole, label) in [(false, "Selected cells"), (true, "Whole table")] {
                let button = egui::Button::selectable(app.border_pen.whole_table == whole, tl!(label)).truncate();
                if ui.add_sized(vec2(half, 22.0), button).clicked() {
                    app.border_pen.whole_table = whole;
                }
            }
        });
        ui.add_space(4.0);
        if let Some(kind) = pen_preview(app, ui) {
            apply_pen(app, kind);
        }
        buttons(ui, |ui| {
            for (kind, tip) in [
                ("top", "Top Border"),
                ("bottom", "Bottom Border"),
                ("left", "Left Border"),
                ("right", "Right Border"),
                ("insideH", "Inside Horizontal Border"),
                ("insideV", "Inside Vertical Border"),
                ("outside", "Outside Borders"),
                ("all", "All Borders"),
                ("none", "No Border"),
                ("diagDown", "Diagonal Down Border"),
                ("diagUp", "Diagonal Up Border"),
            ] {
                if line_button(ui, kind, tl!(tip)).clicked() {
                    apply_pen(app, kind);
                }
            }
        });
        ui.add_space(2.0);
        let on = app.canvas.table_tool == Some(crate::table_pen::TableTool::Paint);
        buttons(ui, |ui| toggle(ui, app, "borderPainter", "Border Painter", "table.borderPainter", on));
    });
}

/// A line button: the Borders menu's picture of `kind`, framed while hovered.
fn line_button(ui: &mut Ui, kind: &str, tip: &str) -> egui::Response {
    let t = crate::theme::Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(28.0, 24.0), egui::Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, 4.0, t.hover);
    }
    let pic = egui::Rect::from_center_size(r.center(), vec2(20.0, 17.0));
    crate::ribbon::layout_picture(ui.painter(), pic, &format!("tb_{kind}"), t.icon, t.accent);
    resp.on_hover_text(tip)
}

/// The pen's color: a swatch that opens Automatic and the palette.
fn pen_color(app: &mut WordApp, ui: &mut Ui) {
    let t = crate::theme::Tokens::get(ui.ctx());
    let fill = app.border_pen.color.as_deref().and_then(wordcraft_doc::Rgb::parse).map_or(t.text, |c| egui::Color32::from_rgb(c.0, c.1, c.2));
    let label = app.border_pen.color.clone().unwrap_or_else(|| tl!("Automatic").to_string());
    let resp = ui.add(egui::Button::new(egui::RichText::new(format!("■ {label}")).color(fill)).min_size(vec2(120.0, 0.0)));
    egui::Popup::menu(&resp).show(|ui| {
        ui.set_min_width(200.0);
        if ui.button(tl!("Automatic")).clicked() {
            app.border_pen.color = None;
        }
        let theme = app.session.doc.settings.theme_colors.clone();
        if let Some(crate::colors::Pick::Color(hex)) = crate::colors::palette(ui, &theme) {
            app.border_pen.color = Some(hex.trim_start_matches('#').to_string());
            ui.close();
        }
    });
}

/// `table.borders` with the pen: the selected cells' `kind` edges, or the whole table's.
fn apply_pen(app: &mut WordApp, kind: &str) {
    let scope = if app.border_pen.whole_table && !kind.starts_with("diag") { "table" } else { "selection" };
    let params = app.border_pen.params(json!({"kind": kind, "scope": scope}));
    if let Err(e) = app.run("table.borders", params) {
        app.status(e);
    }
}

/// A small three-by-three table; a click on one of its lines (top, bottom, left, right, the
/// inside ones) is that line button. The line under the pointer shows in the pen.
fn pen_preview(app: &WordApp, ui: &mut Ui) -> Option<&'static str> {
    let t = crate::theme::Tokens::get(ui.ctx());
    let w = ui.available_width().min(220.0);
    let (r, resp) = ui.allocate_exact_size(vec2(w, 84.0), egui::Sense::click());
    let painter = ui.painter_at(r.expand(4.0));
    let inner = r.shrink(8.0);
    let xs: Vec<f32> = (0..4).map(|i| inner.left() + inner.width() * i as f32 / 3.0).collect();
    let ys: Vec<f32> = (0..4).map(|i| inner.top() + inner.height() * i as f32 / 3.0).collect();
    if !app.border_pen.whole_table {
        // The selected cells: the middle column, tinted.
        painter.rect_filled(egui::Rect::from_min_max(egui::pos2(xs[1], ys[0]), egui::pos2(xs[2], ys[3])), 0.0, t.selection);
    }
    let faint = egui::Stroke::new(1.0, t.border);
    let lines: Vec<(&'static str, egui::Pos2, egui::Pos2)> = {
        let mut v = Vec::new();
        for (i, y) in ys.iter().enumerate() {
            let kind = match i {
                0 => "top",
                3 => "bottom",
                _ => "insideH",
            };
            v.push((kind, egui::pos2(xs[0], *y), egui::pos2(xs[3], *y)));
        }
        for (i, x) in xs.iter().enumerate() {
            let kind = match i {
                0 => "left",
                3 => "right",
                _ => "insideV",
            };
            v.push((kind, egui::pos2(*x, ys[0]), egui::pos2(*x, ys[3])));
        }
        v
    };
    let hover = resp.hover_pos();
    let near = |a: egui::Pos2, b: egui::Pos2, p: egui::Pos2| {
        let (lo, hi) = (a.min(b), a.max(b));
        let d = if a.x == b.x { (p.x - a.x).abs() } else { (p.y - a.y).abs() };
        d <= 5.0 && p.x >= lo.x - 5.0 && p.x <= hi.x + 5.0 && p.y >= lo.y - 5.0 && p.y <= hi.y + 5.0
    };
    let hot = hover.and_then(|p| lines.iter().find(|(_, a, b)| near(*a, *b, p)).map(|(k, ..)| *k));
    let pen_color = app.border_pen.color.as_deref().and_then(wordcraft_doc::Rgb::parse).map_or(t.text, |c| egui::Color32::from_rgb(c.0, c.1, c.2));
    let pen = egui::Stroke::new((app.border_pen.width * 1.5).clamp(1.0, 6.0), pen_color);
    for (kind, a, b) in &lines {
        if Some(*kind) == hot {
            pen_line(&painter, *a, *b, pen, &app.border_pen.style);
        } else {
            painter.line_segment([*a, *b], faint);
        }
    }
    let resp = resp.on_hover_text(tl!("Click a line to give it the pen's line"));
    if hot.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if resp.clicked() { hot } else { None }
}

/// A line from `a` to `b` drawn in border style `style`.
fn pen_line(p: &egui::Painter, a: egui::Pos2, b: egui::Pos2, s: egui::Stroke, style: &str) {
    match style {
        "dashed" => {
            p.extend(egui::Shape::dashed_line(&[a, b], s, 6.0, 4.0));
        }
        "dotted" => {
            p.extend(egui::Shape::dotted_line(&[a, b], s.color, 4.0, s.width / 2.0 + 0.5));
        }
        "dotDash" => {
            p.extend(egui::Shape::dashed_line_with_offset(&[a, b], s, &[8.0, 2.0], &[3.0, 3.0], 0.0));
        }
        "double" => {
            let off = if a.x == b.x { vec2(1.8, 0.0) } else { vec2(0.0, 1.8) };
            let thin = egui::Stroke::new((s.width / 2.0).max(1.0), s.color);
            p.line_segment([a - off, b - off], thin);
            p.line_segment([a + off, b + off], thin);
        }
        _ => {
            p.line_segment([a, b], s);
        }
    }
}

fn rows_columns(app: &mut WordApp, ui: &mut Ui) {
    let half = ((ui.available_width() - 4.0) / 2.0).floor();
    egui::Grid::new("table_pane_insert").num_columns(2).spacing(vec2(4.0, 4.0)).show(ui, |ui| {
        for row in [
            [("insertAbove", "Insert Above", "table.insertRowAbove"), ("insertBelow", "Insert Below", "table.insertRowBelow")],
            [("insertLeft", "Insert Left", "table.insertColumnLeft"), ("insertRight", "Insert Right", "table.insertColumnRight")],
        ] {
            for (icon, label, id) in row {
                labeled_button(ui, app, icon, label, id, Some(json!({})), false, false, half);
            }
            ui.end_row();
        }
    });
    ui.add_space(4.0);
    buttons(ui, |ui| {
        menu(ui, app, "deleteTable", "Delete", "table.deleteTable", |ui, app| {
            pic_item(ui, app, "Delete Cells", None, "del_cell", "table.deleteCells", json!({}));
            pic_item(ui, app, "Delete Columns", None, "del_column", "table.deleteColumn", json!({}));
            pic_item(ui, app, "Delete Rows", None, "del_row", "table.deleteRow", json!({}));
            pic_item(ui, app, "Delete Table", None, "del_table", "table.deleteTable", json!({}));
        });
    });
}

fn cell_size(app: &mut WordApp, ui: &mut Ui) {
    cell_size_boxes(ui, app);
    ui.add_space(2.0);
    buttons(ui, |ui| {
        menu(ui, app, "autofit", "AutoFit", "table.autofit", |ui, app| {
            pic_item(ui, app, "AutoFit Contents", None, "fit_contents", "table.autofit", json!({"mode": "contents"}));
            pic_item(ui, app, "AutoFit Window", None, "fit_window", "table.autofit", json!({"mode": "window"}));
            pic_item(ui, app, "Fixed Column Width", None, "fit_fixed", "table.autofit", json!({"mode": "fixed"}));
        });
        labeled(ui, app, "distributeRows", "Distribute Rows", "table.distributeRows");
        labeled(ui, app, "distributeCols", "Distribute Columns", "table.distributeColumns");
    });
}

fn alignment(app: &mut WordApp, ui: &mut Ui) {
    ui.horizontal_top(|ui| {
        // The nine cell alignments, as on the page: top, centre and bottom rows.
        egui::Grid::new("table_pane_align").spacing(vec2(1.0, 1.0)).show(ui, |ui| {
            for row in [
                [("topLeft", "Align Top Left"), ("topCenter", "Align Top Center"), ("topRight", "Align Top Right")],
                [("centerLeft", "Align Center Left"), ("center", "Align Center"), ("centerRight", "Align Center Right")],
                [("bottomLeft", "Align Bottom Left"), ("bottomCenter", "Align Bottom Center"), ("bottomRight", "Align Bottom Right")],
            ] {
                for (v, tip) in row {
                    small(ui, app, &format!("cellAlign_{v}"), None, tip, "table.cellAlign", json!({"value": v}), false);
                }
                ui.end_row();
            }
        });
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            labeled(ui, app, "textDirection", "Text Direction", "table.textDirection");
            labeled(ui, app, "cellMargins", "Cell Margins", "table.cellMargins");
        });
    });
}

fn table(app: &mut WordApp, ui: &mut Ui) {
    use crate::table_pen::TableTool;
    let tool = app.canvas.table_tool;
    let gridlines = app.session.view.table_gridlines;
    buttons(ui, |ui| {
        menu(ui, app, "selectTable", "Select", "table.selectTable", |ui, app| {
            pic_item(ui, app, "Select Cell", None, "sel_cell", "table.selectCell", json!({}));
            pic_item(ui, app, "Select Column", None, "sel_column", "table.selectColumn", json!({}));
            pic_item(ui, app, "Select Row", None, "sel_row", "table.selectRow", json!({}));
            pic_item(ui, app, "Select Table", None, "sel_table", "table.selectTable", json!({}));
        });
        toggle(ui, app, "gridlines", "View Gridlines", "table.viewGridlines", gridlines);
        toggle(ui, app, "drawTable", "Draw Table", "table.draw", tool == Some(TableTool::Draw));
        toggle(ui, app, "eraser", "Eraser", "table.eraser", tool == Some(TableTool::Erase));
        labeled(ui, app, "properties", "Table Properties", "table.properties");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;
    use wordcraft_engine::Session;

    type H = egui_kittest::Harness<'static, WordApp>;

    /// Show the pane's sub-panel `i` (0 Styles, 1 Borders, 2 Rows and Columns), as a click on
    /// its slot does.
    fn table_tab(h: &mut H, i: usize) {
        h.ctx.data_mut(|d| d.insert_persisted(egui::Id::new("table_pane_tab"), i));
        h.run_steps(3);
    }

    fn window(app: WordApp) -> H {
        let mut h = egui_kittest::Harness::builder().with_size(vec2(1440.0, 900.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        h.run_steps(4);
        h
    }

    /// The caret in a table opens the Table pane (no Table Design or Layout tab); its × hides it
    /// until the caret leaves the table, and it comes back on the next visit.
    #[test]
    fn the_pane_follows_the_caret_into_tables() {
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::from_text("Before")), crate::Services::default());
        app.run("caret.docEnd", json!({})).unwrap();
        app.run("insert.table", json!({"rows": 2, "cols": 2})).unwrap();
        let mut h = window(app);
        assert!(h.query_by_label("Rows and Columns").is_some(), "the pane is open in the table");
        assert!(!crate::ribbon::contextual_tabs(&h.state().session).contains(&"Table Design"));
        // A pane button runs its command.
        let rows = |h: &H| h.state().session.doc.body.iter().find_map(|b| b.as_table()).map(|t| t.rows.len());
        table_tab(&mut h, 2);
        h.get_by_label("Insert Below").click();
        h.run_steps(3);
        assert_eq!(rows(&h), Some(3));
        close(&h.ctx);
        h.run_steps(3);
        assert!(h.query_by_label("Rows and Columns").is_none(), "closed");
        h.state_mut().run("caret.docStart", json!({})).unwrap();
        h.run_steps(3);
        assert!(h.query_by_label("Rows and Columns").is_none(), "outside a table");
        h.state_mut().run("caret.docEnd", json!({})).unwrap();
        h.state_mut().run("caret.up", json!({})).unwrap();
        h.run_steps(3);
        assert!(h.state().session.sel.focus.path.cell().is_some());
        assert!(h.query_by_label("Rows and Columns").is_some(), "back in a table, it opens again");
    }

    /// Clicking into a table opens the pane, and the page keeps the keyboard: what is typed next
    /// goes into the cell (the page's widget id used to change when a pane opened).
    #[test]
    fn typing_right_after_clicking_into_a_table_goes_into_the_cell() {
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::from_text("Before")), crate::Services::default());
        app.run("caret.docEnd", json!({})).unwrap();
        app.run("insert.table", json!({"rows": 2, "cols": 2})).unwrap();
        app.run("caret.docStart", json!({})).unwrap();
        let mut h = window(app);
        assert!(h.query_by_label("Rows and Columns").is_none(), "no pane outside the table");
        let cell = h.state_mut().session.layout().pages[0]
            .items
            .iter()
            .find_map(|it| match it {
                wordcraft_layout::Placed::Cell { rect, row: 0, cell: 0, .. } => Some(*rect),
                _ => None,
            })
            .unwrap();
        let at = crate::canvas::page_to_screen(h.state_mut(), 0, cell.x + cell.w / 2.0, cell.y + cell.h / 2.0).unwrap();
        h.event(egui::Event::PointerMoved(at));
        h.step();
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() });
            h.step();
        }
        h.run_steps(3);
        assert!(h.query_by_label("Rows and Columns").is_some(), "the pane opened");
        h.event(egui::Event::Text("Hi".into()));
        h.run_steps(2);
        let text = h.state().session.doc.plain_text(wordcraft_doc::StoryRef::Body);
        assert!(text.contains("Hi"), "typed into the cell: {text:?}");
    }

    /// The right-hand panes share one dock (owner, 2026-11-15): with a table and Styles both
    /// open only one pane shows, at the shared width, with a strip button for each; the newest
    /// comes to the front, a strip button switches, and closing the front pane shows the other.
    #[test]
    fn right_panes_share_one_dock() {
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.run("insert.table", json!({"rows": 2, "cols": 2})).unwrap();
        let mut h = window(app);
        let width = |h: &H, id: &str| egui::PanelState::load(&h.ctx, egui::Id::new(id)).map(|p| p.outer_rect.width());
        let table_w = width(&h, "right_dock");
        assert_eq!(crate::panes::right_active(&h.ctx), Some("table"));
        h.state_mut().run("view.stylesPane", json!({"value": true})).unwrap();
        h.run_steps(3);
        assert_eq!(crate::panes::right_active(&h.ctx), Some("styles"), "the newest pane comes to the front");
        assert!(h.query_by_label("Rows and Columns").is_none(), "one pane at a time");
        assert_eq!(width(&h, "right_dock"), table_w, "the width stays");
        // The strip has a button for each open pane; the table's brings it back.
        h.get_by_label(tl!("Table")).click();
        h.run_steps(3);
        assert_eq!(crate::panes::right_active(&h.ctx), Some("table"));
        assert!(h.query_by_label("Rows and Columns").is_some());
        assert!(h.state().session.view.styles_pane, "Styles stays open behind it");
        // Closing the front pane shows the one before it.
        close(&h.ctx);
        h.run_steps(3);
        assert_eq!(crate::panes::right_active(&h.ctx), Some("styles"));
        // A dragged edge sets the width for every pane.
        h.ctx.data_mut(|d| d.insert_persisted(egui::Id::new("krub_side_pane_w"), 340.0_f32));
        h.run_steps(3);
        assert!(width(&h, "right_dock").is_some_and(|w| w > table_w.unwrap_or(0.0) + 40.0));
    }

    /// Everything in the Table pane fits its width (owner, 2026-11-18: the pen's Apply to row
    /// was wider than the pane, pushed its contents past the left edge and cut off "Table").
    #[test]
    fn the_table_pane_fits_the_shared_width() {
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.run("insert.table", json!({"rows": 2, "cols": 2})).unwrap();
        let mut h = window(app);
        assert_eq!(crate::panes::right_active(&h.ctx), Some("table"));
        let dock = egui::PanelState::load(&h.ctx, egui::Id::new("right_dock")).map(|p| p.outer_rect).unwrap();
        for label in ["Table Styles", "Rows and Columns"] {
            let r = h.get_by_label(tl!(label)).rect();
            assert!(r.left() >= dock.left() && r.right() <= dock.right() + 0.5, "{label} at {r:?} is outside the pane {dock:?}");
        }
        table_tab(&mut h, 1);
        for label in ["Borders and Shading", "Pen", "Line style:", "Apply to:", "Selected cells", "Whole table"] {
            let r = h.get_by_label(tl!(label)).rect();
            assert!(r.left() >= dock.left() && r.right() <= dock.right() + 0.5, "{label} at {r:?} is outside the pane {dock:?}");
        }
        // The widest row: the Apply to choices go under their label, not beside it, so a wider
        // interface font (the web's) can't push the row past the pane.
        let below = h.get_by_label(tl!("Apply to:")).rect().bottom();
        for label in ["Selected cells", "Whole table"] {
            assert!(h.get_by_label(tl!(label)).rect().top() >= below - 0.5, "{label} sits under Apply to:");
        }
    }

    /// The pen's Line style menu shows drawn lines (owner, 2026-11-19); each keeps its name for
    /// assistive technology, and picking one sets the pen.
    #[test]
    fn the_line_style_menu_picks_by_its_drawn_lines() {
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.run("insert.table", json!({"rows": 2, "cols": 2})).unwrap();
        let mut h = window(app);
        table_tab(&mut h, 1);
        h.get_by(|n| n.role() == egui::accesskit::Role::ComboBox && n.value().as_deref() == Some("Solid line")).click();
        h.run_steps(3);
        for label in ["Solid line", "Dashed line", "Dotted line", "Double line", "Dash-dot line", "Thick line"] {
            assert!(h.query_by_label(label).is_some(), "{label} is in the menu");
        }
        h.get_by_label("Dotted line").click();
        h.run_steps(3);
        assert_eq!(h.state().border_pen.style, "dotted");
        assert!(h.query_by_label("Dashed line").is_none(), "the menu closes");
        assert!(
            h.query_by(|n| n.role() == egui::accesskit::Role::ComboBox && n.value().as_deref() == Some("Dotted line")).is_some(),
            "the box names the new style"
        );
    }

    /// The dock is persistent (owner, 2026-11-17): Navigation shows while nothing is selected,
    /// an object's pane takes its place at the same width and leaving the object brings
    /// Navigation back; the chevron folds the dock to its strip, a selected object doesn't
    /// unfold it, and its strip button (or View › Navigation Pane) does.
    #[test]
    fn the_dock_stays_with_navigation_and_folds() {
        let app = WordApp::new(Session::new(wordcraft_doc::Document::from_text("Before")), crate::Services::default());
        let mut h = window(app);
        let width = |h: &H| egui::PanelState::load(&h.ctx, egui::Id::new("right_dock")).map(|p| p.outer_rect.width());
        let active = |h: &H| crate::panes::right_active(&h.ctx);
        assert_eq!(active(&h), Some("nav"), "Navigation is the home pane");
        assert!(h.state().session.view.nav_pane);
        let nav_w = width(&h);
        assert!(nav_w.is_some());
        h.state_mut().run("caret.docEnd", json!({})).unwrap();
        h.state_mut().run("insert.table", json!({"rows": 2, "cols": 2})).unwrap();
        h.run_steps(3);
        assert_eq!(active(&h), Some("table"));
        assert!(!h.state().session.view.nav_pane);
        assert_eq!(width(&h), nav_w, "the same place, the same width");
        assert!(h.state().session.sel.focus.path.cell().is_some());
        h.state_mut().run("caret.docStart", json!({})).unwrap();
        h.run_steps(3);
        assert_eq!(active(&h), Some("nav"), "out of the table, back to Navigation");
        // Fold it.
        h.get_by_label(tl!("Collapse Side Panel")).click();
        h.run_steps(3);
        assert!(crate::panes::right_collapsed(&h.ctx));
        assert!(!h.state().session.view.nav_pane, "View › Navigation Pane shows it folded");
        // Into the table: still folded, the Table button waits in the strip.
        h.state_mut().run("caret.docEnd", json!({})).unwrap();
        h.state_mut().run("caret.up", json!({})).unwrap();
        h.run_steps(3);
        assert!(crate::panes::right_collapsed(&h.ctx), "a selection doesn't unfold the dock");
        assert!(h.query_by_label("Rows and Columns").is_none());
        h.get_by_label(tl!("Table")).click();
        h.run_steps(3);
        assert!(!crate::panes::right_collapsed(&h.ctx));
        assert!(h.query_by_label("Rows and Columns").is_some());
        // View › Navigation Pane brings Navigation to the front; off folds the dock.
        h.state_mut().run("view.navigationPane", json!({"value": true})).unwrap();
        h.run_steps(3);
        assert_eq!(active(&h), Some("nav"));
        h.state_mut().run("view.navigationPane", json!({"value": false})).unwrap();
        h.run_steps(3);
        assert!(crate::panes::right_collapsed(&h.ctx));
        h.get_by_label(tl!("Expand Side Panel")).click();
        h.run_steps(3);
        assert!(!crate::panes::right_collapsed(&h.ctx));
        assert!(h.state().session.view.nav_pane);
        // The same from a script: ui.sidePanel.
        h.state_mut().run("ui.sidePanel", json!({"value": false})).unwrap();
        h.run_steps(3);
        assert!(crate::panes::right_collapsed(&h.ctx));
        h.state_mut().run("ui.sidePanel", json!({})).unwrap();
        h.run_steps(3);
        assert!(!crate::panes::right_collapsed(&h.ctx), "no value toggles");
    }
}
