//! The Table pane (owner, 2026-11-15): while the caret is in a table, a side pane on the right
//! holds every table command that used to be on the Table Design and Table Layout tabs, in
//! sections (styles, borders and shading, rows and columns, merge and split, cell size,
//! alignment, data, table). It opens by itself when the caret enters a table and closes when it
//! leaves; its × hides it until the caret leaves the table. It has the shared pane width
//! ([`crate::panes::PANE_W`]).

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
    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("table_pane_scroll").show(ui, |ui| {
        section(ui, "Table Styles", |ui| styles(app, ui));
        section(ui, "Borders and Shading", |ui| borders(app, ui));
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
        section(ui, "Table", |ui| table(app, ui));
    });
}

/// A section: a heading that folds it, open at first.
pub(crate) fn section(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    egui::CollapsingHeader::new(egui::RichText::new(tl!(title)).font(crate::theme::semibold(12.5)))
        .id_salt(("side_pane_section", title))
        .default_open(true)
        .show(ui, add);
    ui.add_space(2.0);
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
        labeled(ui, app, "borderPainter", "Border Painter", "table.borderPainter");
    });
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
        assert!(h.query_by_label("Merge and Split").is_some(), "the pane is open in the table");
        assert!(!crate::ribbon::contextual_tabs(&h.state().session).contains(&"Table Design"));
        // A pane button runs its command.
        let rows = |h: &H| h.state().session.doc.body.iter().find_map(|b| b.as_table()).map(|t| t.rows.len());
        h.get_by_label("Insert Below").click();
        h.run_steps(3);
        assert_eq!(rows(&h), Some(3));
        close(&h.ctx);
        h.run_steps(3);
        assert!(h.query_by_label("Merge and Split").is_none(), "closed");
        h.state_mut().run("caret.docStart", json!({})).unwrap();
        h.run_steps(3);
        assert!(h.query_by_label("Merge and Split").is_none(), "outside a table");
        h.state_mut().run("caret.docEnd", json!({})).unwrap();
        h.state_mut().run("caret.up", json!({})).unwrap();
        h.run_steps(3);
        assert!(h.state().session.sel.focus.path.cell().is_some());
        assert!(h.query_by_label("Merge and Split").is_some(), "back in a table, it opens again");
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
        assert!(h.query_by_label("Merge and Split").is_none(), "no pane outside the table");
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
        assert!(h.query_by_label("Merge and Split").is_some(), "the pane opened");
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
        assert!(h.query_by_label("Merge and Split").is_none(), "one pane at a time");
        assert_eq!(width(&h, "right_dock"), table_w, "the width stays");
        // The strip has a button for each open pane; the table's brings it back.
        h.get_by_label(tl!("Table")).click();
        h.run_steps(3);
        assert_eq!(crate::panes::right_active(&h.ctx), Some("table"));
        assert!(h.query_by_label("Merge and Split").is_some());
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
}
