//! Right-hand task panes: Comments, Watch Window, Selection Pane, Format Chart.

use egui::{Color32, Ui, vec2};
use serde_json::{Value as Json, json};

use gridcraft_engine::model::{ObjKind, ObjRef};

use crate::SheetApp;
use crate::theme::{self, Tokens};

pub fn show(app: &mut SheetApp, ui: &mut Ui) {
    // The Comments pane is engine state (`review.showComments`, so agents can drive it); the other
    // panes are UI state. One pane shows at a time: opening another one closes Comments.
    if app.grid.pane.as_deref() == Some("comments") {
        app.grid.pane = None;
        app.run_or_alert("review.showComments", json!({"on": true}));
    } else if app.grid.pane.is_some() && app.session.comments_pane {
        app.run_or_alert("review.showComments", json!({"on": false}));
    }
    dock(app, ui);
}

/// A dock state key for the window being drawn: each window has its own dock.
fn key(ctx: &egui::Context, name: impl std::hash::Hash + std::fmt::Debug) -> egui::Id {
    egui::Id::unique((ctx.viewport_id(), name))
}

/// A pane's title: the task panes (Comments, Watch Window…) and the object panes that follow the
/// selection (Table, Chart, Picture, Shape).
fn title(id: &str) -> &'static str {
    match id {
        "comments" => "Comments",
        "watch" => "Watch Window",
        "selection" => "Selection",
        "formatChart" => "Format Chart Area",
        "obj:table" => "Table",
        "obj:chart" => "Chart",
        "obj:picture" => "Picture",
        "obj:shape" => "Shape",
        "obj:idle" => "Format",
        _ => "",
    }
}

/// The strip button's icon for a pane.
fn strip_icon(id: &str) -> crate::icons::Icon {
    use crate::icons::Icon;
    match id {
        "obj:table" => Icon::Table,
        "obj:chart" | "formatChart" => Icon::Chart,
        "obj:picture" => Icon::Picture,
        "obj:shape" | "obj:idle" => Icon::Shapes,
        "comments" => Icon::Comment,
        "watch" => Icon::ZoomSelection,
        _ => Icon::SheetList,
    }
}

/// The width of the strip of pane buttons on the dock's outer edge.
pub const STRIP_W: f32 = 36.0;

/// Whether an object pane shows: while the selection asks for it, unless its × closed it (until
/// the selection no longer asks). While a mouse button is held, and on the frame it is released,
/// it stays as it was, so a press doesn't move the sheet under the pointer.
fn follow(ctx: &egui::Context, id: &'static str, active: bool) -> bool {
    let (closed, shown) = (key(ctx, ("pane_closed", id)), key(ctx, ("pane_shown", id)));
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

/// Hide the object pane `id` (`"table"`, `"chart"`…) until the selection no longer asks for it;
/// the dock doesn't stay open for it either.
pub fn close_object_pane(ctx: &egui::Context, id: &str) {
    let (closed, sticky) = (key(ctx, ("pane_closed", id)), key(ctx, "dock_sticky"));
    ctx.data_mut(|d| {
        d.insert_temp(closed, true);
        d.insert_temp(sticky, false);
    });
}

/// The panes open now: the object panes the selection asks for, the placeholder that keeps the
/// dock open once it has been used, then the task pane.
pub fn open_panes(app: &SheetApp, ctx: &egui::Context) -> Vec<&'static str> {
    let wanted = crate::ribbon::object_panes(app);
    let mut v: Vec<&'static str> = ["table", "chart", "picture", "shape"]
        .into_iter()
        .filter(|id| follow(ctx, id, wanted.contains(id)))
        .map(|id| match id {
            "table" => "obj:table",
            "chart" => "obj:chart",
            "picture" => "obj:picture",
            _ => "obj:shape",
        })
        .collect();
    let task = app.grid.pane.as_deref().or_else(|| app.session.comments_pane.then_some("comments"));
    let task = task.and_then(|p| ["comments", "watch", "selection", "formatChart"].into_iter().find(|k| *k == p));
    // Once an object pane has been open the dock stays open (owner, 2026-11-16): with nothing
    // selected it shows a placeholder at the same width, so the sheet doesn't move each time the
    // selection comes and goes. Its × closes the dock until an object is chosen again.
    let sticky = key(ctx, "dock_sticky");
    if !v.is_empty() {
        ctx.data_mut(|d| d.insert_temp(sticky, true));
    } else if task.is_none() && ctx.data(|d| d.get_temp::<bool>(sticky).unwrap_or(false)) {
        v.push("obj:idle");
    }
    v.extend(task);
    v
}

/// The pane the dock shows now (`"obj:table"`, `"comments"`…), if any.
pub fn active_pane(ctx: &egui::Context) -> Option<&'static str> {
    let active = key(ctx, "dock_active");
    ctx.data(|d| d.get_temp::<Option<&'static str>>(active)).flatten()
}

/// The right-hand dock (owner, 2026-11-16, as in KhianKrub and PhutKrub): one pane at a time at
/// one width, with a strip of buttons on its outer edge for every open pane. A pane that opens
/// comes to the front, but an object pane never covers an open task pane (it waits in the
/// strip); a strip button brings its pane back; closing the front pane shows the one before it.
fn dock(app: &mut SheetApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let ctx = ui.ctx().clone();
    let (order_id, prev_id, active_id) = (key(&ctx, "dock_order"), key(&ctx, "dock_prev"), key(&ctx, "dock_active"));
    let open = open_panes(app, &ctx);
    let prev: Vec<&'static str> = ctx.data(|d| d.get_temp(prev_id)).unwrap_or_default();
    let mut order: Vec<&'static str> = ctx.data(|d| d.get_temp(order_id)).unwrap_or_default();
    let front = |order: &mut Vec<&'static str>, id: &'static str| {
        order.retain(|o| *o != id);
        order.insert(0, id);
    };
    let task_open = open.iter().any(|id| !id.starts_with("obj:"));
    for id in open.iter().rev().filter(|id| id.starts_with("obj:") && !prev.contains(id)) {
        if task_open {
            order.retain(|o| o != id);
            order.push(id);
        } else {
            front(&mut order, id);
        }
    }
    for id in open.iter().filter(|id| !id.starts_with("obj:") && !prev.contains(id)) {
        front(&mut order, id);
    }
    ctx.data_mut(|d| d.insert_temp(prev_id, open.clone()));
    let mut active = order.iter().copied().find(|id| open.contains(id)).or_else(|| open.first().copied());
    if open.is_empty() {
        ctx.data_mut(|d| {
            d.insert_temp(order_id, order);
            d.insert_temp::<Option<&'static str>>(active_id, None);
        });
        return;
    }
    egui::Panel::right("task_pane_strip")
        .exact_size(STRIP_W)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(t.ribbon).inner_margin(egui::Margin::symmetric(3, 8)).stroke(egui::Stroke::new(1.0, t.separator)))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            for id in &open {
                let on = active == Some(*id);
                let (r, resp) = ui.allocate_exact_size(vec2(30.0, 30.0), egui::Sense::click());
                let fill = if on {
                    t.selected
                } else if resp.hovered() {
                    t.hover
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect_filled(r, 5.0, fill);
                if on {
                    ui.painter().rect_stroke(r, 5.0, egui::Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
                }
                crate::widgets::paint_icon(ui, egui::Rect::from_center_size(r.center(), vec2(20.0, 20.0)), strip_icon(id), t.text);
                let lang = crate::i18n::current(ui.ctx());
                let name = lang.tr(title(id)).to_string();
                // Named apart from the pane's own heading: "Pane: Comments".
                let label = lang.tr("Pane: {}").replace("{}", &name);
                let resp = resp.on_hover_text(&name);
                resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, on, &label));
                if resp.clicked() {
                    active = Some(*id);
                }
            }
        });
    if let Some(id) = active {
        front(&mut order, id);
    }
    ctx.data_mut(|d| {
        d.insert_temp(order_id, order);
        d.insert_temp(active_id, active);
    });
    let Some(pane) = active else { return };
    // Capped at half the window so a pane can never cover the sheet, whatever its content asks for.
    let max = (ui.available_width() / 2.0).max(300.0);
    let panel = egui::Panel::right("task_pane").default_size(300.0).max_size(max).resizable(true);
    panel.frame(egui::Frame::NONE.fill(t.ribbon).inner_margin(10)).show(ui, |ui| {
        ui.horizontal(|ui| {
            let name = crate::i18n::current(ui.ctx()).tr(title(pane)).to_string();
            ui.label(egui::RichText::new(name).font(theme::ui_bold(15.0)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::widgets::icon_button(ui, crate::icons::Icon::Close, t.text_dim, "Close", vec2(22.0, 22.0)).clicked() {
                    match pane.strip_prefix("obj:") {
                        Some(obj) => close_object_pane(ui.ctx(), obj),
                        None => {
                            if pane == "comments" {
                                app.run_or_alert("review.showComments", json!({"on": false}));
                            }
                            app.grid.pane = None;
                        }
                    }
                }
            });
        });
        ui.separator();
        egui::ScrollArea::vertical().id_salt(("task_pane_scroll", pane)).show(ui, |ui| match pane {
            "comments" => comments(app, ui),
            "watch" => watch(app, ui),
            "selection" => selection(app, ui),
            "formatChart" => format_chart(app, ui),
            "obj:idle" => {
                let hint = crate::i18n::current(ui.ctx()).tr("Choose a table, chart, picture or shape on the sheet to see its commands here.");
                ui.label(egui::RichText::new(hint).color(t.text_dim));
            }
            obj => {
                if let Some(obj) = obj.strip_prefix("obj:") {
                    crate::ribbon::object_pane(app, ui, obj);
                }
            }
        });
    });
}

/// Lists comments and replies, links each to its cell, and runs delete/resolve/reopen/reply commands.
fn comments(app: &mut SheetApp, ui: &mut Ui) {
    let Some(d) = app.session.active() else { return };
    let Some(sh) = d.wb.active() else { return };
    let list: Vec<(String, gridcraft_engine::model::Comment)> = sh.comments.iter().map(|(c, m)| (c.a1(), m.clone())).collect();
    if list.is_empty() {
        ui.label(egui::RichText::new("No comments on this sheet yet. Select a cell and choose New Comment.").italics());
    }
    let t = crate::theme::Tokens::get(ui.ctx());
    for (cell, c) in list {
        egui::Frame::NONE
            .fill(if c.resolved { t.window } else { t.menu_bg })
            .corner_radius(6.0)
            .inner_margin(8.0)
            .stroke(egui::Stroke::new(1.0, t.separator))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    if ui.link(egui::RichText::new(&cell).strong()).clicked() {
                        let _ = app.run("selection.set", json!({"cell": cell}));
                        app.grid.ensure_visible = true;
                    }
                    ui.label(egui::RichText::new(&c.author).color(t.text_dim));
                    if c.resolved {
                        ui.label(egui::RichText::new("Resolved").small().color(t.accent));
                    }
                });
                ui.label(&c.text);
                for (a, txt) in &c.replies {
                    ui.label(egui::RichText::new(format!("↳ {a}: {txt}")).color(t.text));
                }
                let key = egui::Id::unique(("reply", &cell));
                let mut draft: String = ui.ctx().data_mut(|m| m.get_temp(key)).unwrap_or_default();
                // Buttons first, right to left, so the reply field gets exactly the width they leave:
                // a field sized by a guess wider than the row pushes the resizable pane wider every frame.
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Delete").clicked() {
                            app.run_or_alert("review.deleteComment", json!({"cell": cell}));
                        }
                        if ui.small_button(if c.resolved { "Reopen" } else { "Resolve" }).clicked() {
                            app.run_or_alert("review.resolveComment", json!({"cell": cell, "resolved": !c.resolved}));
                        }
                        let send = ui.small_button("Send").clicked();
                        let r = ui.add(egui::TextEdit::singleline(&mut draft).hint_text("Reply…").desired_width(ui.available_width()));
                        if (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) || send) && !draft.is_empty() {
                            app.run_or_alert("review.replyComment", json!({"cell": cell, "text": draft}));
                            draft.clear();
                        }
                    });
                });
                ui.ctx().data_mut(|m| m.insert_temp(key, draft));
            });
        ui.add_space(6.0);
    }
}

/// Adds the active cell, lists watch values and formulas, and removes watched entries.
fn watch(app: &mut SheetApp, ui: &mut Ui) {
    if ui.button("Add Watch (active cell)").clicked()
        && let Some(d) = app.session.active()
        && let Some(sh) = d.wb.active()
    {
        let r = format!("{}!{}", gridcraft_engine::formula::quote_sheet(&sh.name), d.selection.active.a1());
        let _ = app.session.run("formulas.watchWindow", json!({"add": r}));
    }
    let list = app.session.run("formulas.watchWindow", json!({})).unwrap_or(Json::Null);
    egui::Grid::new("watch_grid").striped(true).num_columns(4).show(ui, |ui| {
        ui.strong("Cell");
        ui.strong("Value");
        ui.strong("Formula");
        ui.label("");
        ui.end_row();
        for w in list.as_array().cloned().unwrap_or_default() {
            let cell = w["cell"].as_str().unwrap_or("").to_string();
            ui.label(&cell);
            ui.label(w["value"].as_str().unwrap_or(""));
            ui.label(w["formula"].as_str().unwrap_or(""));
            if crate::widgets::icon_button(ui, crate::icons::Icon::Close, Tokens::get(ui.ctx()).text_dim, "Delete", vec2(18.0, 18.0)).clicked() {
                let _ = app.session.run("formulas.watchWindow", json!({"remove": cell}));
            }
            ui.end_row();
        }
    });
}

/// Lists the sheet's objects and dispatches select, bring-forward, send-backward and delete.
fn selection(app: &mut SheetApp, ui: &mut Ui) {
    let list = app.session.run("arrange.selectionPane", json!({})).unwrap_or(Json::Null);
    let items = list.as_array().cloned().unwrap_or_default();
    if items.is_empty() {
        ui.label(egui::RichText::new("There are no shapes, pictures or charts on this sheet.").italics());
    }
    for it in items.iter().rev() {
        let kind = it["kind"].as_str().unwrap_or("").to_string();
        let id = it["id"].as_u64().unwrap_or(0) as u32;
        let obj = ObjKind::parse(&kind).map(|k| ObjRef::new(k, id));
        ui.horizontal(|ui| {
            let sel = obj.is_some() && app.selected_object == obj;
            if ui.selectable_label(sel, it["name"].as_str().unwrap_or("")).clicked() {
                app.selected_object = obj;
            }
            if ui.small_button("▲").on_hover_text("Bring Forward").clicked() {
                app.run_or_alert("arrange.bringForward", json!({"kind": kind, "id": id}));
            }
            if ui.small_button("▼").on_hover_text("Send Backward").clicked() {
                app.run_or_alert("arrange.sendBackward", json!({"kind": kind, "id": id}));
            }
            if crate::widgets::icon_button(ui, crate::icons::Icon::Close, Tokens::get(ui.ctx()).text_dim, "Delete", vec2(18.0, 18.0)).clicked() {
                app.run_or_alert("object.delete", json!({"kind": kind, "id": id}));
            }
        });
    }
}

/// Draws the chart format pane: title, type, legend, elements/axis titles, series colors, palettes/layouts and size.
fn format_chart(app: &mut SheetApp, ui: &mut Ui) {
    let Some(id) = app.selected_chart() else {
        ui.label(egui::RichText::new("Select a chart to format it.").italics());
        return;
    };
    let Some(chart) = app.session.active().and_then(|d| d.wb.active().and_then(|s| s.charts.iter().find(|c| c.id == id).cloned())) else {
        ui.label("The chart no longer exists.");
        return;
    };
    let theme = app.session.active().map(|d| d.wb.theme.clone()).unwrap_or_default();
    let lang = crate::i18n::current(ui.ctx());
    ui.collapsing("Chart Title", |ui| {
        let field = egui::Id::unique(("gridcraft.chart_title", id));
        if let Some(title) = crate::widgets::committed_text(ui, field, chart.title.as_deref().unwrap_or(""), None) {
            app.run_or_alert("chart.set", json!({"chart": id, "title": title}));
        }
    });
    ui.collapsing("Chart Type", |ui| {
        ui.horizontal_wrapped(|ui| {
            for (l, k, s) in [
                ("Column", "column", ""),
                ("Stacked Column", "column", "stacked"),
                ("100% Stacked Column", "column", "stacked100"),
                ("Bar", "bar", ""),
                ("Stacked Bar", "bar", "stacked"),
                ("100% Stacked Bar", "bar", "stacked100"),
                ("Line", "line", "markers"),
                ("Stacked Line", "line", "stacked"),
                ("Area", "area", ""),
                ("Stacked Area", "area", "stacked"),
                ("100% Stacked Area", "area", "stacked100"),
                ("Pie", "pie", ""),
                ("Doughnut", "doughnut", ""),
                ("Scatter", "scatter", ""),
                ("Radar", "radar", ""),
            ] {
                let cur = chart.kind == gridcraft_engine::cmd::insert::chart_kind(k, s);
                if ui.selectable_label(cur, l).clicked() {
                    app.run_or_alert("chart.set", json!({"chart": id, "type": k, "subtype": s}));
                }
            }
        });
        if matches!(chart.kind, gridcraft_chart::ChartKind::Pie | gridcraft_chart::ChartKind::Doughnut) {
            let draft = egui::Id::unique(("gridcraft.chart_first_slice_draft", id));
            let current = u32::from(chart.first_slice_angle);
            if let Some(angle) = chart_slider(ui, draft, current, 0..=359, lang.tr("First slice angle")) {
                app.run_or_alert("chart.set", json!({"chart": id, "firstSliceAngle": angle}));
            }
        }
        if chart.kind == gridcraft_chart::ChartKind::Doughnut {
            let draft = egui::Id::unique(("gridcraft.chart_hole_size_draft", id));
            let current = u32::from(chart.hole_size);
            if let Some(hole) = chart_slider(ui, draft, current, 10..=90, lang.tr("Doughnut hole size")) {
                app.run_or_alert("chart.set", json!({"chart": id, "holeSize": hole}));
            }
        }
    });
    ui.collapsing("Legend", |ui| {
        ui.horizontal(|ui| {
            for (l, v) in [("None", "none"), ("Bottom", "bottom"), ("Top", "top"), ("Left", "left"), ("Right", "right")] {
                let cur = format!("{:?}", chart.legend).eq_ignore_ascii_case(v);
                if ui.selectable_label(cur, l).clicked() {
                    app.run_or_alert("chart.set", json!({"chart": id, "legend": v}));
                }
            }
        });
    });
    ui.collapsing("Elements", |ui| {
        let mut dl = chart.data_labels;
        if ui.checkbox(&mut dl, "Data Labels").changed() {
            app.run_or_alert("chart.set", json!({"chart": id, "dataLabels": dl}));
        }
        let mut gl = chart.gridlines;
        if ui.checkbox(&mut gl, "Gridlines").changed() {
            app.run_or_alert("chart.set", json!({"chart": id, "gridlines": gl}));
        }
        ui.horizontal(|ui| {
            ui.label("Horizontal axis title:");
            let field = egui::Id::unique(("gridcraft.chart_x_title", id));
            if let Some(xt) = crate::widgets::committed_text(ui, field, chart.x_title.as_deref().unwrap_or(""), None) {
                app.run_or_alert("chart.set", json!({"chart": id, "xTitle": if xt.is_empty() { Json::Null } else { json!(xt) }}));
            }
        });
        ui.horizontal(|ui| {
            ui.label("Vertical axis title:");
            let field = egui::Id::unique(("gridcraft.chart_y_title", id));
            if let Some(yt) = crate::widgets::committed_text(ui, field, chart.y_title.as_deref().unwrap_or(""), None) {
                app.run_or_alert("chart.set", json!({"chart": id, "yTitle": if yt.is_empty() { Json::Null } else { json!(yt) }}));
            }
        });
    });
    ui.collapsing("Data Table", |ui| {
        let supported = chart.supports_data_table();
        let mut on = chart.data_table.is_some();
        if ui.add_enabled(supported, egui::Checkbox::new(&mut on, "Show Data Table")).changed() {
            app.run_or_alert("chart.set", json!({"chart": id, "dataTable": on}));
        }
        if !supported {
            ui.label("Available for column, bar, line, area and compatible combo charts.");
        }
        if let Some(table) = &chart.data_table {
            for (label, key, mut value) in [
                ("Show Legend Keys", "dataTableKeys", table.show_keys),
                ("Horizontal Borders", "dataTableHorizontalBorder", table.horizontal_border),
                ("Vertical Borders", "dataTableVerticalBorder", table.vertical_border),
                ("Outline", "dataTableOutline", table.outline),
            ] {
                if ui.checkbox(&mut value, label).changed() {
                    app.run_or_alert("chart.set", json!({"chart": id, key: value}));
                }
            }
        }
    });
    ui.collapsing("Series", |ui| {
        let data = app.session.active().map(|d| gridcraft_chart::resolve(&d.wb, d.wb.active_sheet(), &chart));
        for (i, s) in chart.series.iter().enumerate() {
            let name = data.as_ref().and_then(|d| d.series.get(i)).map(|x| x.name.clone()).unwrap_or_else(|| format!("Series {}", i + 1));
            let col =
                data.as_ref().and_then(|d| d.series.get(i)).map(|x| Color32::from_rgb(x.color[0], x.color[1], x.color[2])).unwrap_or(Color32::GRAY);
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), egui::Sense::hover());
                ui.painter().rect_filled(r, 2.0, col);
                ui.label(&name);
                ui.label(egui::RichText::new(&s.values).small().color(crate::theme::Tokens::get(ui.ctx()).text_dim));
                ui.menu_button("Fill…", |ui| {
                    if let Some(c) = crate::widgets::color_palette_fixed(ui, &theme.colors, "Automatic") {
                        app.run_or_alert(
                            "chart.formatSelection",
                            json!({"chart": id, "series": i, "color": if c == "none" { Json::Null } else { json!(c) }}),
                        );
                        ui.close();
                    }
                });
            });
        }
    });
    ui.collapsing("Colors & Layout", |ui| {
        ui.horizontal_wrapped(|ui| {
            for p in ["colorful", "monochrome", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6"] {
                if ui.button(p).clicked() {
                    app.run_or_alert("chart.changeColors", json!({"chart": id, "palette": p}));
                }
            }
        });
        ui.horizontal(|ui| {
            for n in 1..=6 {
                if ui.button(format!("Layout {n}")).clicked() {
                    app.run_or_alert("chart.quickLayout", json!({"chart": id, "layout": n}));
                }
            }
        });
    });
    ui.collapsing("Size", |ui| {
        // Keep the value being typed or dragged between frames; reloading it from the chart every frame threw the edit away.
        let draft = egui::Id::unique(("gridcraft.chart_size_draft", id));
        let current = (chart.anchor.width, chart.anchor.height);
        let (mut w, mut h) = ui.data_mut(|d| d.get_temp::<(f32, f32)>(draft)).unwrap_or(current);
        // A file can make a chart larger than the 4,000 points a field otherwise reaches. The
        // range keeps such a size, so the field shows it instead of clamping it to 4,000.
        let range = |v: f32| 40.0..=v.max(4000.0);
        let a = ui.add(egui::DragValue::new(&mut w).prefix("Width ").range(range(current.0)));
        let b = ui.add(egui::DragValue::new(&mut h).prefix("Height ").range(range(current.1)));
        if a.drag_stopped() || a.lost_focus() || b.drag_stopped() || b.lost_focus() {
            ui.data_mut(|d| d.remove::<(f32, f32)>(draft));
            if !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                // Only an edited dimension is sent: the other keeps what the chart has, however
                // large a file made it.
                let mut params = serde_json::Map::new();
                params.insert("chart".into(), json!(id));
                if w != current.0 {
                    params.insert("width".into(), json!(w));
                }
                if h != current.1 {
                    params.insert("height".into(), json!(h));
                }
                if params.len() > 1 {
                    app.run_or_alert("chart.set", Json::Object(params));
                }
            }
        } else if a.dragged() || a.has_focus() || b.dragged() || b.has_focus() {
            ui.data_mut(|d| d.insert_temp(draft, (w, h)));
        } else {
            ui.data_mut(|d| d.remove::<(f32, f32)>(draft));
        }
    });
}

/// A chart slider that keeps the value being dragged in a per-chart draft and submits it once
/// through `chart.set` when the interaction ends, like the Size controls. Escape cancels, and a
/// value the chart already has is not sent again.
fn chart_slider(ui: &mut Ui, draft: egui::Id, value: u32, range: std::ops::RangeInclusive<u32>, label: &str) -> Option<u32> {
    let mut current = ui.data_mut(|d| d.get_temp::<u32>(draft)).unwrap_or(value);
    let r = ui.add(egui::Slider::new(&mut current, range).text(label));
    if r.drag_stopped() || r.clicked() || r.lost_focus() {
        ui.data_mut(|d| d.remove::<u32>(draft));
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) || current == value { None } else { Some(current) }
    } else if r.dragged() || r.has_focus() {
        ui.data_mut(|d| d.insert_temp(draft, current));
        None
    } else {
        ui.data_mut(|d| d.remove::<u32>(draft));
        None
    }
}
