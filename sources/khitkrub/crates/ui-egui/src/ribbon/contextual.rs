use super::controls::{act, anchor_keytip, control_keytip, keytip, menu_items, pane_heading, section};
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
    pane_heading(ui, "Properties");
    crate::ribbon_layout::column(ui, |ui| {
        let label = ui.label(egui::RichText::new(crate::i18n::current(ui.ctx()).tr("Table Name:")).small()).id;
        let field = egui::Id::unique(("gridcraft.table_name", &tname));
        if let Some(name) = crate::widgets::committed_text_labelled(ui, field, &tname, Some(120.0), Some(label))
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
    section(ui, "Tools");
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
    section(ui, "Slicers");
    if big_button(ui, Icon::Filter, "Insert\nSlicer", "Insert Slicer", false).clicked() {
        act(app, "table.insertSlicer", json!({"table": tname}));
    }
    section(ui, "Table Style Options");
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
            // In a side pane the row wraps to its width (the Thai names are long).
            let wrap = crate::widgets::pane_mode();
            let row_ui = |ui: &mut Ui| {
                for (label, cmd, v) in items {
                    let mut on = v;
                    if ui.checkbox(&mut on, crate::i18n::current(ui.ctx()).tr(label)).changed() {
                        act(app, cmd, json!({"table": tname, "on": on}));
                    }
                }
            };
            if wrap {
                ui.horizontal_wrapped(row_ui);
            } else {
                ui.horizontal(row_ui);
            }
        }
    });
    section(ui, "Table Styles");
    table_style_strip(app, ui, &tname, &style);
}

/// The first page of Excel's Table Styles gallery, drawn in the ribbon. The More arrow opens the
/// full gallery (Light, Medium, Dark) as a popup.
const RIBBON_TABLE_STYLES: [&str; 7] =
    ["TableStyleLight1", "TableStyleLight2", "TableStyleLight3", "TableStyleLight4", "TableStyleLight5", "TableStyleLight6", "TableStyleLight7"];

fn table_style_strip(app: &mut SheetApp, ui: &mut Ui, tname: &str, current: &str) {
    let t = Tokens::get(ui.ctx());
    ui.add_space(12.0);
    // In a side pane the tiles wrap to its width.
    let wrap = crate::widgets::pane_mode();
    let strip = |ui: &mut Ui| {
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
    };
    if wrap {
        ui.horizontal_wrapped(strip);
    } else {
        ui.horizontal(strip);
    }
}

pub(super) fn chart_design(app: &mut SheetApp, ui: &mut Ui) {
    let Some(id) = app.selected_chart() else { return };
    pane_heading(ui, "Chart Elements and Data");
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
            let lang = crate::i18n::current(ui.ctx());
            if krub_icons::chart_icons::item(ui, krub_icons::chart_icons::id_for(k, s), lang.tr(label), false, 180.0).clicked() {
                act(app, "chart.set", json!({"chart": id, "type": k, "subtype": s}));
                ui.close();
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
    section(ui, "Size");
    size_group(app, ui);
    section(ui, "Delete");
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
                // Showing a size never changes it: a picture a file made wider than the box's limit
                // keeps its size until someone types or drags a new one.
                let r = ui.add(
                    egui::DragValue::new(&mut v)
                        .speed(0.01)
                        .range(0.01..=10000.0 / per)
                        .clamp_existing_to_range(false)
                        .fixed_decimals(2)
                        .suffix(suffix),
                );
                if r.changed() {
                    act(app, "object.move", json!({"kind": kind, "id": o.id, key: v * per}));
                }
            });
        }
    });
}

/// Picture Format and Shape Format: the Size group (more groups can join it here).
pub(super) fn object_format(app: &mut SheetApp, ui: &mut Ui) {
    pane_heading(ui, "Size");
    size_group(app, ui);
}

/// The shadows the shape pane's Shadow menu offers (the first removes it).
const SHADOWS: [(&str, Option<gridcraft_engine::model::ShapeShadow>); 4] = [
    ("No Shadow", None),
    ("Outer shadow", Some(gridcraft_engine::model::ShapeShadow { dist: 3.0, dir: 45.0, blur: 4.0, transparency: 60.0 })),
    ("Bottom shadow", Some(gridcraft_engine::model::ShapeShadow { dist: 4.0, dir: 90.0, blur: 4.0, transparency: 60.0 })),
    ("Centered shadow", Some(gridcraft_engine::model::ShapeShadow { dist: 0.0, dir: 0.0, blur: 8.0, transparency: 60.0 })),
];

/// A preview tile of `shadow` under a small card; its name shows on hover.
fn shadow_tile(ui: &mut Ui, label: &str, selected: bool, shadow: Option<gridcraft_engine::model::ShapeShadow>) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(44.0, 36.0), Sense::click());
    let name = label.to_string();
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, selected, &name));
    let p = ui.painter();
    p.rect_filled(rect, 5.0, if resp.hovered() { t.hover } else { t.input_bg });
    // A light sheet behind the card, so the dark shadow shows in the dark theme too.
    p.rect_filled(rect.shrink(4.0), 3.0, Color32::from_gray(246));
    let card = egui::Rect::from_center_size(rect.center() - vec2(1.0, 1.0), vec2(20.0, 14.0));
    if let Some(sh) = shadow {
        let a = sh.dir.to_radians();
        let at = card.translate(vec2(a.cos(), a.sin()) * sh.dist * 1.2);
        for i in (0..=4).rev() {
            let k = i as f32 / 4.0;
            p.rect_filled(at.expand(sh.blur / 2.0 * k), 3.0, Color32::from_black_alpha(45));
        }
    }
    p.rect_filled(card, 2.0, t.accent);
    p.rect_stroke(
        rect,
        5.0,
        Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { t.accent } else { t.input_border }),
        egui::StrokeKind::Inside,
    );
    resp.on_hover_text(label)
}

/// KhitKrub's colour boxes, buttons and number boxes for the shared object pane.
struct KhitKrub<'a> {
    app: &'a mut SheetApp,
    /// The shape the colour boxes format; `None` for a picture.
    shape: Option<u32>,
}

impl krub_icons::shape_pane::Host for KhitKrub<'_> {
    fn color(&mut self, ui: &mut Ui, slot: krub_icons::shape_pane::Slot, enabled: bool) {
        use krub_icons::shape_pane::Slot;
        let Some(id) = self.shape else { return };
        let lang = crate::i18n::current(ui.ctx());
        let (key, none, name) = match slot {
            Slot::Fill => ("fill", "No Fill", "Fill color"),
            Slot::Line => ("line", "No Outline", "Line color"),
            // No "none" for a glow: a radius of 0 removes it.
            Slot::Glow => ("glowColor", "", "Glow color"),
        };
        let r = ui.add_enabled(enabled, egui::Button::new(lang.tr("Color")).min_size(vec2(80.0, 22.0)));
        let name = lang.tr(name).to_string();
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, &name));
        let theme = super::controls::theme_colors(self.app);
        let app = &mut *self.app;
        egui::Popup::menu(&r).show(|ui| {
            if let Some(c) = crate::widgets::color_palette(ui, &theme, none) {
                if c == "none" && slot == Slot::Glow {
                    return;
                }
                let v = if c == "none" { json!(null) } else { json!(c) };
                act(app, "shape.format", json!({"id": id, key: v}));
                ui.close();
            }
        });
    }

    fn shadow(&mut self, ui: &mut Ui) {
        let Some(id) = self.shape else { return };
        let lang = crate::i18n::current(ui.ctx());
        let current = self
            .app
            .session
            .active()
            .and_then(|d| d.wb.active())
            .and_then(|sh| sh.shapes.iter().find(|x| x.id == id))
            .and_then(|sp| sp.format.shadow);
        let r = ui.button(format!("{} ⏷", lang.tr("Shadow")));
        let app = &mut *self.app;
        egui::Popup::menu(&r).show(|ui| {
            ui.horizontal(|ui| {
                for (label, shadow) in SHADOWS {
                    let on = current.map(|c| (c.dist, c.dir, c.blur)) == shadow.map(|c| (c.dist, c.dir, c.blur));
                    if shadow_tile(ui, lang.tr(label), on, shadow).clicked() {
                        act(app, "shape.format", json!({"id": id, "shadow": shadow.map(|c| json!({"dist": c.dist, "dir": c.dir, "blur": c.blur}))}));
                        ui.close();
                    }
                }
            });
        });
    }

    fn number(&mut self, ui: &mut Ui, n: krub_icons::shape_pane::Number) -> Option<f32> {
        // Showing a value never changes it: a file can carry an object larger than the box's limit.
        let mut v = n.value;
        let r = ui
            .add_enabled(
                n.enabled,
                egui::DragValue::new(&mut v).range(n.range).clamp_existing_to_range(false).speed(n.speed).max_decimals(2).suffix(n.suffix),
            )
            .labelled_by(n.label);
        // Enter commits the box. The keys typed with it were the box's: none reaches the grid
        // (Enter would move the active cell, Cmd+A select the sheet, text start a cell edit).
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            ui.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Key { .. } | egui::Event::Text(_) | egui::Event::Ime(_))));
            crate::grid::hold_typing(ui.ctx());
        }
        r.changed().then_some(v)
    }

    fn button(&mut self, ui: &mut Ui, icon: &str, label: &str, name: &str) -> bool {
        let t = Tokens::get(ui.ctx());
        let icon = match icon {
            "flipVertical" => "flip-v",
            "flipHorizontal" => "flip-h",
            "bringToFront" | "bringForward" => "bringForward",
            "sendBackward" => "sendBackward",
            "sendToBack" => "send-to-back",
            other => other,
        };
        let galley = ui.painter().layout_no_wrap(label.to_string(), crate::theme::ui_font(11.5), t.text);
        let (r, resp) = ui.allocate_exact_size(vec2(28.0 + galley.size().x + 8.0, 28.0), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(r, 4.0, t.hover);
        }
        let ic = egui::Rect::from_center_size(egui::pos2(r.left() + 14.0, r.center().y), vec2(18.0, 18.0));
        krub_icons::paint_named(ui.painter(), ic, icon, t.text, true, false);
        ui.painter().galley(egui::pos2(r.left() + 28.0, r.center().y - galley.size().y / 2.0), galley, t.text);
        let name = name.to_string();
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
        resp.clicked()
    }
}

/// The Picture and Shape panes (owner, 2026-11-16): the object pane shared by the three
/// KrubOffice apps ([`krub_icons::shape_pane`]). A shape has Area, Line (with its transparency
/// and arrows) and Position and Size (with rotation, flip and stacking order); a picture has
/// Position and Size. Effects (shadow, glow, soft edges) and Align come with later engine work, so
/// they aren't shown (no buttons that would do nothing). Then KhitKrub's own Properties: how the
/// object moves and sizes with the cells.
pub(super) fn object_pane(app: &mut SheetApp, ui: &mut Ui) {
    use krub_icons::shape_pane::{Caps, Dash, Edit, Values};
    let Some(o) = app.selected_object else { return };
    let Some(sh) = app.session.active().and_then(|d| d.wb.active()) else { return };
    let Some(a) = sh.object_anchor(o) else { return };
    let shape = (o.kind == gridcraft_engine::model::ObjKind::Shape).then(|| sh.shapes.iter().find(|x| x.id == o.id).cloned()).flatten();
    let lang = crate::i18n::current(ui.ctx());
    let kind = match o.kind {
        gridcraft_engine::model::ObjKind::Chart => "chart",
        gridcraft_engine::model::ObjKind::Image => "image",
        gridcraft_engine::model::ObjKind::Slicer => "slicer",
        _ => "shape",
    };
    let metric = metric_sizes(app);
    let per = if metric { PX_PER_INCH / 2.54 } else { PX_PER_INCH };
    let keep_id = egui::Id::unique("object_pane_keep_ratio");
    let keep: bool = ui.data(|d| d.get_temp(keep_id)).unwrap_or(false);
    let fixed = |sp: &gridcraft_engine::model::Shape| {
        !matches!(
            sp.kind,
            gridcraft_engine::model::ShapeKind::Ink | gridcraft_engine::model::ShapeKind::Equation | gridcraft_engine::model::ShapeKind::Icon
        )
    };
    let caps = match &shape {
        Some(sp) if fixed(sp) => Caps {
            area: true,
            line: true,
            line_transparency: true,
            arrows: true,
            effects: true,
            rotation: true,
            flip: true,
            arrange: true,
            ..Caps::default()
        },
        Some(_) => Caps { arrange: true, ..Caps::default() },
        None => Caps::default(),
    };
    let mut v = Values {
        width: (f64::from(a.width) / per) as f32,
        height: (f64::from(a.height) / per) as f32,
        size_suffix: if metric { " cm" } else { " in" }.into(),
        size_max: (10000.0 / per) as f32,
        keep_ratio: keep,
        ..Values::default()
    };
    if let Some(sp) = &shape {
        use gridcraft_engine::model::{LineDash, ShapeKind};
        let f = &sp.format;
        v.open = matches!(sp.kind, ShapeKind::Line | ShapeKind::Arrow);
        v.has_fill = !f.no_fill;
        v.fill_transparency = f.fill_transparency;
        v.has_line = !f.no_line;
        v.dash = match f.line_dash {
            LineDash::Solid => Dash::Solid,
            LineDash::Dot | LineDash::SysDot => Dash::Dotted,
            LineDash::LgDash => Dash::LongDash,
            LineDash::DashDot | LineDash::SysDashDot | LineDash::LgDashDot | LineDash::LgDashDotDot | LineDash::SysDashDotDot => Dash::DashDot,
            _ => Dash::Dashed,
        };
        v.thickness = f.line_width.unwrap_or(0.75);
        v.line_transparency = f.line_transparency;
        v.arrow_start = f.arrow_start;
        v.arrow_end = f.arrow_end || (sp.kind == ShapeKind::Arrow && !f.arrow_start);
        v.rotation = f.rotation;
        v.has_glow = f.glow_radius > 0.0;
        v.glow_radius = f.glow_radius;
        v.glow_transparency = f.glow_transparency;
        v.soft_edge = f.soft_edge;
    }
    let tr = |s: &str| lang.tr(s).to_string();
    let edits = {
        let mut host = KhitKrub { app: &mut *app, shape: shape.as_ref().map(|s| s.id) };
        krub_icons::shape_pane::sections(ui, &v, caps, &tr, &mut host)
    };
    for edit in edits {
        let fmt = |p: serde_json::Value| -> serde_json::Value {
            let mut p = p;
            p["id"] = json!(o.id);
            p
        };
        let dash = |d: Dash| match d {
            Dash::Solid => "solid",
            Dash::Dotted => "sysDot",
            Dash::Dashed => "dash",
            Dash::LongDash => "lgDash",
            Dash::DashDot => "dashDot",
        };
        match edit {
            Edit::Width(w) | Edit::Height(w) => {
                let key = if matches!(edit, Edit::Width(_)) { "width" } else { "height" };
                let mut p = json!({"kind": kind, "id": o.id, key: f64::from(w) * per});
                // Keep ratio: the other side follows, in proportion.
                if keep && a.width > 0.0 && a.height > 0.0 {
                    let ratio = f64::from(a.height) / f64::from(a.width);
                    if key == "width" {
                        p["height"] = json!(f64::from(w) * per * ratio);
                    } else {
                        p["width"] = json!(f64::from(w) * per / ratio);
                    }
                }
                act(app, "object.move", p);
            }
            Edit::KeepRatio(k) => {
                ui.data_mut(|d| d.insert_temp(keep_id, k));
            }
            Edit::FillTransparency(t) => act(app, "shape.format", fmt(json!({"fillTransparency": t}))),
            Edit::LineNone => act(app, "shape.format", fmt(json!({"line": null}))),
            Edit::LineDash(d) => act(app, "shape.format", fmt(json!({"lineDash": dash(d)}))),
            Edit::Thickness(w) => act(app, "shape.format", fmt(json!({"lineWidth": w.clamp(0.25, 72.0)}))),
            Edit::LineTransparency(t) => act(app, "shape.format", fmt(json!({"lineTransparency": t}))),
            Edit::ArrowStart(on) => act(app, "shape.format", fmt(json!({"arrowStart": on}))),
            Edit::ArrowEnd(on) => act(app, "shape.format", fmt(json!({"arrowEnd": on}))),
            Edit::Rotation(r) => act(app, "shape.format", fmt(json!({"rotation": r}))),
            Edit::FlipVertical => act(app, "shape.format", fmt(json!({"flipV": !shape.as_ref().is_some_and(|s| s.format.flip_v)}))),
            Edit::FlipHorizontal => act(app, "shape.format", fmt(json!({"flipH": !shape.as_ref().is_some_and(|s| s.format.flip_h)}))),
            Edit::Order(op) => act(app, "object.order", json!({"kind": "shape", "id": o.id, "op": op})),
            Edit::GlowRadius(r) => act(app, "shape.format", fmt(json!({"glowRadius": r.clamp(0.0, 150.0)}))),
            Edit::GlowTransparency(t) => act(app, "shape.format", fmt(json!({"glowTransparency": t.clamp(0.0, 100.0)}))),
            Edit::SoftEdge(r) => act(app, "shape.format", fmt(json!({"softEdge": r.clamp(0.0, 100.0)}))),
            // Align isn't offered in KhitKrub yet.
            Edit::Align(_) => {}
        }
    }
    // KhitKrub's own: how the object moves and sizes with the cells, under Size and Position
    // with the shared heading (owner, 2026-11-18).
    if krub_icons::shape_pane::current_tab(ui.ctx(), caps) != krub_icons::shape_pane::Tab::SizePosition {
        return;
    }
    let mode = a.mode;
    krub_icons::pane_ui::section(ui, krub_icons::pane_ui::id("object_pane_properties"), lang.tr("Properties"), true, |ui| {
        use gridcraft_engine::model::AnchorMode;
        for (label, m, value) in [
            ("Move and size with cells", AnchorMode::MoveAndSize, "moveAndSize"),
            ("Move but don't size with cells", AnchorMode::MoveOnly, "moveOnly"),
            ("Don't move or size with cells", AnchorMode::Absolute, "absolute"),
        ] {
            if ui.radio(mode == m, lang.tr(label)).clicked() && mode != m {
                act(app, "object.setAnchorMode", json!({"kind": kind, "id": o.id, "mode": value}));
            }
        }
    });
}
