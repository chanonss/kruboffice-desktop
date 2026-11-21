//! Draw Table and Eraser as canvas modes (Insert › Tables, Table Layout › Draw). The buttons
//! toggle a pen or an eraser; while one is on, presses on the page draw (or erase) instead of
//! moving the caret, a preview follows the pointer, and each finished stroke or click runs
//! `table.draw` / `table.eraser` with page coordinates (one undo step each). Esc or the button
//! again turns it off. The geometry decisions are the engine's (`cmd::table_draw`).

use egui::{Pos2, Rect, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};
use wordcraft_engine::cmd::table_draw::{self, Stroke as Kind};
use wordcraft_layout::DocLayout;

use crate::WordApp;
use crate::theme::Tokens;

/// Which table tool is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableTool {
    Draw,
    Erase,
    /// The border painter (owner, 2026-11-17): a click or drag along a table's lines gives each
    /// the Table pane's pen (`table.paintBorder`).
    Paint,
}

impl TableTool {
    fn id(self) -> &'static str {
        match self {
            TableTool::Draw => "table.draw",
            TableTool::Erase => "table.eraser",
            TableTool::Paint => "table.paintBorder",
        }
    }
}

/// The line the border painter draws with: its style (OOXML name), weight (points) and color
/// (`RRGGBB`, `None` automatic). Chosen in the Table pane; kept while the app runs.
#[derive(Clone, Debug, PartialEq)]
pub struct BorderPen {
    pub style: String,
    pub width: f32,
    pub color: Option<String>,
    /// The line buttons set the whole table's lines, not the selected cells' edges.
    pub whole_table: bool,
}

impl Default for BorderPen {
    fn default() -> Self {
        BorderPen { style: "single".into(), width: 0.5, color: None, whole_table: false }
    }
}

impl BorderPen {
    /// The pen as `table.borders` / `table.paintBorder` parameters, merged into `v`.
    pub fn params(&self, mut v: Value) -> Value {
        v["style"] = json!(self.style);
        v["width"] = json!(self.width);
        if let Some(c) = &self.color {
            v["color"] = json!(c);
        }
        v
    }
}

/// A stroke in progress: its page and its start and current end (page points).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PenStroke {
    page: usize,
    from: (f32, f32),
    to: (f32, f32),
}

/// The ribbon buttons (no page coordinates): turn the tool on, or off when it already is.
/// `{"on": bool}` sets it. With coordinates the command goes to the engine instead (`None`).
pub fn toggle(app: &mut WordApp, id: &str, p: &Value) -> Option<Result<Value, String>> {
    let tool = match id {
        "table.draw" => TableTool::Draw,
        "table.eraser" => TableTool::Erase,
        // Border Painter (Table Design › Borders) paints lines on the page with the pen.
        "table.paintBorder" | "table.borderPainter" => TableTool::Paint,
        _ => return None,
    };
    if p.get("page").is_some() {
        return None;
    }
    let on = p.get("on").and_then(Value::as_bool).unwrap_or(app.canvas.table_tool != Some(tool));
    app.canvas.table_tool = on.then_some(tool);
    app.canvas.table_stroke = None;
    if on {
        app.canvas.want_focus = true;
        app.status(match tool {
            TableTool::Draw => tl!("Drag a rectangle to draw a table, or a line across cells to split them. Esc stops."),
            TableTool::Erase => tl!("Click a border between two cells to merge them. Esc stops."),
            TableTool::Paint => tl!("Click or drag along a table's lines to give them the pen's line; again to take it off. Esc stops."),
        });
    }
    Some(Ok(json!({"tool": app.canvas.table_tool.map(TableTool::id)})))
}

/// Turn the tool off (Esc).
pub fn stop(app: &mut WordApp) -> bool {
    let was = app.canvas.table_tool.is_some();
    app.canvas.table_tool = None;
    app.canvas.table_stroke = None;
    was
}

/// The page point under screen point `p` on page `page` (its screen rect from `rects`).
fn on_page(rects: &[Rect], layout: &DocLayout, scale: f32, page: usize, p: Pos2) -> Option<(f32, f32)> {
    let r = rects.get(page)?;
    let scale = layout.pages.get(page).filter(|pg| pg.w > 0.0).map_or(scale, |pg| r.width() / pg.w);
    (scale > 0.0).then(|| ((p.x - r.min.x) / scale, (p.y - r.min.y) / scale))
}

/// Mouse input while a tool is on; `false` (nothing done) when none is, so the caret logic runs.
pub fn pointer(app: &mut WordApp, ui: &Ui, resp: &egui::Response, rects: &[Rect], layout: &DocLayout, scale: f32) -> bool {
    let Some(tool) = app.canvas.table_tool else { return false };
    if !app.canvas.focused && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        stop(app);
        return true;
    }
    let latest = ui.input(|i| i.pointer.latest_pos());
    if tool == TableTool::Paint {
        return paint_pointer(app, ui, resp, rects, layout, scale, latest);
    }
    if ui.input(|i| i.pointer.primary_pressed()) && resp.contains_pointer() {
        if let Some(p) = latest
            && let Some((page, x, y)) = crate::canvas::page_at(rects, layout, scale, p)
        {
            app.canvas.table_stroke = Some(PenStroke { page, from: (x, y), to: (x, y) });
        }
        return true;
    }
    let Some(mut st) = app.canvas.table_stroke else { return true };
    if let Some(p) = latest
        && let Some(to) = on_page(rects, layout, scale, st.page, p)
    {
        st.to = to;
        app.canvas.table_stroke = Some(st);
    }
    if ui.input(|i| i.pointer.primary_down()) {
        return true;
    }
    app.canvas.table_stroke = None;
    let params = match tool {
        TableTool::Draw => {
            // A click without a drag draws nothing.
            if (st.to.0 - st.from.0).abs() < 2.0 && (st.to.1 - st.from.1).abs() < 2.0 {
                return true;
            }
            json!({"page": st.page, "x0": st.from.0, "y0": st.from.1, "x1": st.to.0, "y1": st.to.1})
        }
        TableTool::Erase => json!({"page": st.page, "x": st.to.0, "y": st.to.1}),
        TableTool::Paint => return true,
    };
    if let Err(e) = app.run(tool.id(), params) {
        app.status(e);
    }
    true
}

/// The border painter's mouse: the line under a press takes the pen at once, and so does each
/// other line the pointer then crosses while the button is held, all one undo step.
fn paint_pointer(app: &mut WordApp, ui: &Ui, resp: &egui::Response, rects: &[Rect], layout: &DocLayout, scale: f32, latest: Option<Pos2>) -> bool {
    let at = latest.and_then(|p| crate::canvas::page_at(rects, layout, scale, p));
    let target = at.and_then(|(page, x, y)| table_draw::line_target(layout, page, x, y).map(|t| (page, x, y, t)));
    let pressed = ui.input(|i| i.pointer.primary_pressed()) && resp.contains_pointer();
    let down = ui.input(|i| i.pointer.primary_down());
    if !(pressed || down && app.canvas.painting.is_some()) {
        if !down {
            app.canvas.painting = None;
        }
        return true;
    }
    let Some((page, x, y, t)) = target else { return true };
    if app.canvas.painting.as_ref().is_some_and(|last| *last == t) {
        return true;
    }
    if !pressed {
        app.session.join_next_undo();
    }
    let params = app.border_pen.params(json!({"page": page, "x": x, "y": y}));
    if let Err(e) = app.run("table.paintBorder", params) {
        app.status(e);
    }
    app.canvas.painting = Some(t);
    true
}

/// The pointer while a tool is on (a crosshair; the tool's icon is drawn beside it).
pub fn cursor(app: &WordApp, ui: &Ui, resp: &egui::Response) {
    if app.canvas.table_tool.is_some() && (resp.hovered() || app.canvas.table_stroke.is_some()) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
}

fn dashed_rect(painter: &egui::Painter, r: Rect, s: Stroke) {
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    painter.extend(egui::Shape::dashed_line(&pts, s, 5.0, 3.0));
}

/// The live preview: the rectangle or line being drawn, or the border the eraser would remove;
/// and the tool's icon beside the pointer.
pub fn paint(app: &WordApp, painter: &egui::Painter, t: &Tokens, layout: &DocLayout, pages: &[Rect], scale: f32) {
    let Some(tool) = app.canvas.table_tool else { return };
    let to_screen = |page: usize, x: f32, y: f32| -> Option<Pos2> {
        let r = pages.get(page)?;
        let sc = layout.pages.get(page).filter(|pg| pg.w > 0.0).map_or(scale, |pg| r.width() / pg.w);
        Some(pos2(r.min.x + x * sc, r.min.y + y * sc))
    };
    let ink = Stroke::new(1.5, t.accent);
    let hover = painter.ctx().input(|i| i.pointer.hover_pos());
    match (tool, app.canvas.table_stroke) {
        (TableTool::Draw, Some(st)) => match table_draw::stroke(layout, st.page, st.from.0, st.from.1, st.to.0, st.to.1) {
            Some(Kind::Rect(r)) => {
                if let (Some(a), Some(b)) = (to_screen(st.page, r.x, r.y), to_screen(st.page, r.right(), r.bottom())) {
                    painter.rect_stroke(Rect::from_two_pos(a, b), 0.0, ink, egui::StrokeKind::Middle);
                }
            }
            Some(Kind::Vertical { x, y0, y1 }) => {
                if let (Some(a), Some(b)) = (to_screen(st.page, x, y0), to_screen(st.page, x, y1)) {
                    painter.line_segment([a, b], ink);
                }
            }
            Some(Kind::Horizontal { y, x0, x1 }) => {
                if let (Some(a), Some(b)) = (to_screen(st.page, x0, y), to_screen(st.page, x1, y)) {
                    painter.line_segment([a, b], ink);
                }
            }
            // Not a table or a line yet: the drag's box, dashed and dim.
            None => {
                if let (Some(a), Some(b)) = (to_screen(st.page, st.from.0, st.from.1), to_screen(st.page, st.to.0, st.to.1)) {
                    dashed_rect(painter, Rect::from_two_pos(a, b), Stroke::new(1.0, t.text_dim));
                }
            }
        },
        (TableTool::Paint, _) => {
            if let Some(p) = hover
                && let Some((page, x, y)) = crate::canvas::page_at(pages, layout, scale, p)
                && let Some(e) = table_draw::line_target(layout, page, x, y)
                && let (Some(a), Some(b)) = (to_screen(page, e.from.0, e.from.1), to_screen(page, e.to.0, e.to.1))
            {
                let color =
                    app.border_pen.color.as_deref().and_then(wordcraft_doc::Rgb::parse).map_or(t.text, |c| egui::Color32::from_rgb(c.0, c.1, c.2));
                let sc = crate::table_resize::page_scale(pages, layout, scale, page).unwrap_or(scale);
                painter.line_segment([a, b], Stroke::new((app.border_pen.width * sc).max(2.0), color));
            }
        }
        (TableTool::Erase, _) => {
            if let Some(p) = hover
                && let Some((page, x, y)) = crate::canvas::page_at(pages, layout, scale, p)
                && let Some(e) = table_draw::erase_target(layout, page, x, y)
                && let (Some(a), Some(b)) = (to_screen(page, e.from.0, e.from.1), to_screen(page, e.to.0, e.to.1))
            {
                painter.line_segment([a, b], Stroke::new(3.0, t.red));
            }
        }
        _ => {}
    }
    if let Some(p) = hover.filter(|p| pages.iter().any(|r| r.contains(*p))) {
        let r = Rect::from_min_size(p + vec2(8.0, 6.0), vec2(16.0, 16.0));
        let icon = match tool {
            TableTool::Draw => "pencil",
            TableTool::Erase => "eraser",
            TableTool::Paint => "brush",
        };
        crate::icons::paint(painter, r, icon, t.icon, t.accent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Owner, 2026-11-17: Border Painter paints the line under a click with the Table pane's pen
    /// (style, weight, color) and takes it off with a second click; the button toggles it.
    #[test]
    fn border_painter_paints_one_line_with_the_pen() {
        use crate::table_resize::tests::{button, cell, screen, steps, table_window};
        use wordcraft_doc::props::{Border, BorderStyle};
        let mut h = table_window();
        assert_eq!(h.state_mut().run("table.borderPainter", json!({})).unwrap()["tool"], "table.paintBorder");
        h.state_mut().border_pen = BorderPen { style: "dashed".into(), width: 2.25, color: Some("C00000".into()), whole_table: false };
        let a = cell(&mut h, 0, 0);
        let at = screen(&mut h, a.right(), a.y + a.h / 2.0);
        let depth = h.state().session.undo_depth();
        let click = |h: &mut crate::table_resize::tests::H| {
            h.event(egui::Event::PointerMoved(at));
            h.step();
            button(h, at, true);
            h.step();
            button(h, at, false);
            steps(h, 2);
        };
        click(&mut h);
        let want = Border { style: BorderStyle::Dashed, width: 2.25, color: wordcraft_doc::Rgb::parse("C00000"), space: 0.0 };
        let side = |h: &crate::table_resize::tests::H, c: usize, right: bool| {
            let t = h.state().session.doc.body.iter().find_map(|b| b.as_table()).unwrap();
            t.rows[0].cells[c].props.borders.and_then(|b| if right { b.right } else { b.left })
        };
        assert_eq!((side(&h, 0, true), side(&h, 1, false)), (Some(want), Some(want)));
        assert_eq!(h.state().session.undo_depth(), depth + 1);
        click(&mut h);
        assert_eq!(side(&h, 0, true).map(|b| b.style), Some(BorderStyle::None), "a second click takes it off");
        h.state_mut().run("table.borderPainter", json!({})).unwrap();
        assert_eq!(h.state().canvas.table_tool, None);
    }

    /// The buttons toggle the tools (one at a time); coordinates go to the engine.
    #[test]
    fn buttons_toggle_the_pen_and_the_eraser() {
        let mut a = WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        assert_eq!(a.run("table.draw", json!({})).unwrap()["tool"], "table.draw");
        assert_eq!(a.canvas.table_tool, Some(TableTool::Draw));
        assert_eq!(a.run("table.eraser", json!({})).unwrap()["tool"], "table.eraser");
        assert_eq!(a.run("table.eraser", json!({})).unwrap()["tool"], Value::Null);
        assert_eq!(a.canvas.table_tool, None);
        a.run("table.draw", json!({"on": true})).unwrap();
        assert!(stop(&mut a) && a.canvas.table_tool.is_none());
        // With coordinates it is the engine's command (an error here: nothing to split).
        assert!(a.run("table.draw", json!({"page": 0, "x0": 1, "y0": 1, "x1": 2, "y1": 2})).is_err());
        assert!(a.canvas.table_tool.is_none());
    }

    /// Deleting a table or removing its last cells turns off the eraser.
    #[test]
    fn deleting_table_turns_off_the_eraser() {
        let mut a = WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        a.run("insert.table", json!({"rows": 2, "cols": 2})).unwrap();
        a.run("table.eraser", json!({"on": true})).unwrap();
        assert_eq!(a.canvas.table_tool, Some(TableTool::Erase));
        a.run("table.deleteTable", json!({})).unwrap();
        assert_eq!(a.canvas.table_tool, None);

        // Deleting the only row also removes the table and stops the eraser.
        a.run("insert.table", json!({"rows": 1, "cols": 1})).unwrap();
        a.run("table.eraser", json!({"on": true})).unwrap();
        assert_eq!(a.canvas.table_tool, Some(TableTool::Erase));
        a.run("table.deleteRow", json!({})).unwrap();
        assert_eq!(a.canvas.table_tool, None);
    }
}
