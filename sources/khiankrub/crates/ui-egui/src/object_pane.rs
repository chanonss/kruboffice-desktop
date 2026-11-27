//! The Shape, Chart, Picture and SmartArt panes (owner, 2026-11-15): like the Table pane
//! ([`crate::table_pane`]), a selected shape, chart, picture or SmartArt graphic opens a
//! right-hand pane with every command that used to be on its Shape Format, Chart Design, Picture
//! Format or SmartArt Design tab, in folding sections, at the shared pane width. Each opens by
//! itself while its object is selected, closes when the selection moves on, and its × hides it
//! until then.

use egui::{Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::json;

use crate::WordApp;
use crate::colors::Target;
use crate::options::MeasureExt;
use crate::ribbon::{mi, mi_check, rotate_items, rotation_field, size_box, wrap_items};
use crate::table_pane::{GAP, close_pane, col_w, faces, field, follow, grid, ruled, section, tile, tile_menu, wide, wide_menu};
use crate::theme::Tokens;

const SHAPE_CLOSED: &str = "shape_pane_closed";
const CHART_CLOSED: &str = "chart_pane_closed";
const PICTURE_CLOSED: &str = "picture_pane_closed";
const SMART_ART_CLOSED: &str = "smart_art_design_pane_closed";

/// Hide the Picture pane until the picture is no longer selected (its ×).
pub fn close_picture(ctx: &egui::Context) {
    close_pane(ctx, PICTURE_CLOSED);
}

/// Whether the Picture pane shows: a picture is selected.
pub fn picture_wanted(app: &WordApp, ctx: &egui::Context) -> bool {
    follow(ctx, PICTURE_CLOSED, crate::ribbon::has_picture_selected(&app.session))
}

/// Whether the SmartArt pane shows: a SmartArt graphic is selected.
pub fn smart_art_wanted(app: &WordApp, ctx: &egui::Context) -> bool {
    follow(ctx, SMART_ART_CLOSED, crate::ribbon::has_smart_art_selected(&app.session))
}

/// Whether the Shape pane shows: a shape (text box, WordArt, drawing canvas…) is selected.
pub fn shape_wanted(app: &WordApp, ctx: &egui::Context) -> bool {
    follow(ctx, SHAPE_CLOSED, crate::ribbon::has_shape_selected(&app.session))
}

/// Whether the Chart pane shows: a chart is selected.
pub fn chart_wanted(app: &WordApp, ctx: &egui::Context) -> bool {
    follow(ctx, CHART_CLOSED, crate::ribbon::has_chart_selected(&app.session))
}

/// The selected object's width and height boxes (points, shown in the Options unit), and below
/// them, for a shape, its rotation.
fn size(app: &mut WordApp, ui: &mut Ui, min_pt: f32, rotation: bool) {
    let (w0, h0) = wordcraft_engine::cmd::objects::selected(&app.session).and_then(|(_, o)| o.frame().map(|(w, h, _)| (w, h))).unwrap_or((0.0, 0.0));
    egui::Grid::new("object_pane_size").num_columns(2).spacing(vec2(8.0, 4.0)).show(ui, |ui| {
        for (label, key, v0) in [("Width:", "width", w0), ("Height:", "height", h0)] {
            ui.label(egui::RichText::new(tl!(label)).small());
            let (r, v) = size_box(ui, v0, min_pt, 2000.0);
            if r.changed() {
                if r.dragged() && !r.drag_started() {
                    app.session.join_next_undo();
                }
                let _ = app.run("picture.size", json!({key: v, "lockAspect": false}));
            }
            ui.end_row();
        }
    });
    if rotation {
        ui.add_space(2.0);
        ui.horizontal(|ui| rotation_field(ui, app));
    }
}

/// The selected shape's look, as the pane shows it.
struct ShapeNow {
    kind: wordcraft_doc::para::ShapeKind,
    fill: Option<wordcraft_doc::Rgb>,
    stroke: Option<wordcraft_doc::Rgb>,
    stroke_width: f32,
    effects: wordcraft_doc::effects::ShapeEffects,
    extra: wordcraft_doc::connector::ShapeExtra,
}

fn shape_now(app: &WordApp) -> Option<ShapeNow> {
    match wordcraft_engine::cmd::objects::selected(&app.session)? {
        (_, wordcraft_doc::para::InlineObject::Shape { kind, fill, stroke, stroke_width, effects, extra, .. }) => {
            Some(ShapeNow { kind, fill, stroke, stroke_width, effects, extra })
        }
        _ => None,
    }
}

/// A colour box that runs `id` with `{"color": RRGGBB}` when a colour is picked.
fn color(ui: &mut Ui, app: &mut WordApp, theme: &[wordcraft_doc::Rgb], value: Option<wordcraft_doc::Rgb>, id: &str) {
    let mut c = value.map(|c| c.hex()).unwrap_or_default();
    let was = c.clone();
    crate::dialogs::color_field(ui, theme, &mut c, "No Color", "");
    if c != was {
        let _ = app.run(id, json!({"color": wordcraft_doc::Rgb::parse(&c).map(|c| c.hex())}));
    }
}

/// What KhianKrub stores for a shape: no stacking order between shapes yet, and no vertical align.
const SHAPE_CAPS: krub_icons::shape_pane::Caps =
    krub_icons::shape_pane::Caps { arrange: false, align_vertical: false, ..krub_icons::shape_pane::Caps::ALL };

/// The shared sections (Area, Line, Effect, Position and Size) from `krub_icons::shape_pane`,
/// drawn with KhianKrub's colour boxes, number boxes, buttons and Shadow menu; their edits run
/// KhianKrub's commands. KhianKrub has no stacking order between shapes yet (no Arrange) and
/// aligns left, centre or right only.
fn shared_sections(app: &mut WordApp, ui: &mut Ui, theme: &[wordcraft_doc::Rgb], now: &ShapeNow) {
    use krub_icons::shape_pane::{self as sp, Dash, Edit, Values};
    use wordcraft_doc::connector::LineDash;
    let unit = crate::options::unit();
    let k = unit.pt_per_unit();
    let ratio_key = egui::Id::new("shape_pane_keep_ratio");
    let keep = ui.ctx().data(|d| d.get_temp::<bool>(ratio_key).unwrap_or(false));
    let (w0, h0, rot) = wordcraft_engine::cmd::objects::selected(&app.session)
        .and_then(|(_, o)| o.frame().map(|(w, h, f)| (w, h, f.spin().deg)))
        .unwrap_or((0.0, 0.0, 0.0));
    let fx = now.effects.sanitized();
    let glow = fx.glow.unwrap_or_default();
    let dash = match now.extra.dash {
        LineDash::Solid => Dash::Solid,
        LineDash::Dot => Dash::Dotted,
        LineDash::Dash => Dash::Dashed,
        LineDash::LongDash => Dash::LongDash,
        LineDash::DashDot => Dash::DashDot,
    };
    let values = Values {
        open: now.kind.is_open(),
        has_fill: now.fill.is_some(),
        fill_transparency: now.extra.fill_transparency,
        has_line: now.stroke.is_some(),
        dash,
        thickness: now.stroke_width,
        line_transparency: now.extra.line_transparency,
        arrow_start: now.extra.arrow_start,
        arrow_end: now.extra.arrow_end,
        has_glow: fx.glow.is_some_and(|g| g.size > 0.0),
        glow_radius: fx.glow.map_or(0.0, |g| g.size),
        glow_transparency: glow.transparency,
        soft_edge: fx.soft_edge.unwrap_or(0.0),
        width: w0 / k,
        height: h0 / k,
        size_suffix: String::new(),
        size_max: 2000.0 / k,
        rotation: rot,
        keep_ratio: keep,
        // While the corner handle is dragged, the radius it would drop (owner, 2026-11-24).
        corner: (now.kind == wordcraft_doc::para::ShapeKind::RoundedRectangle)
            .then(|| crate::objects::corner_dragged(app).unwrap_or_else(|| wordcraft_doc::connector::corner_radius(now.extra.corner, w0, h0)) / k),
        corner_max: w0.min(h0).max(0.0) / 2.0 / k,
    };
    let caps = SHAPE_CAPS;
    let tr = |s: &str| tl!(s).to_string();
    let mut host = Host { app: &mut *app, theme, now, unit };
    let edits = sp::sections(ui, &values, caps, &tr, &mut host);
    let (glow_c, glow_t, glow_r) = (glow.color.hex(), glow.transparency, glow.size);
    for e in edits {
        let (id, params) = match e {
            Edit::FillTransparency(t) => ("shape.fill", json!({"transparency": t})),
            Edit::LineNone => ("shape.outline", json!({"color": null})),
            Edit::LineDash(d) => {
                let d = match d {
                    Dash::Solid => "solid",
                    Dash::Dotted => "dot",
                    Dash::Dashed => "dash",
                    Dash::LongDash => "longDash",
                    Dash::DashDot => "dashDot",
                };
                ("shape.outline", json!({"color": now.stroke.unwrap_or(wordcraft_doc::Rgb::BLACK).hex(), "dash": d}))
            }
            Edit::Thickness(w) => ("shape.outline", json!({"width": w})),
            Edit::LineTransparency(t) => ("shape.outline", json!({"transparency": t})),
            Edit::ArrowStart(b) => ("shape.outline", json!({"arrowStart": b})),
            Edit::ArrowEnd(b) => ("shape.outline", json!({"arrowEnd": b})),
            Edit::GlowRadius(r) if r <= 0.0 => ("shape.effects", json!({"glow": null})),
            Edit::GlowRadius(r) => ("shape.effects", json!({"glow": {"size": r, "color": glow_c, "transparency": glow_t}})),
            Edit::GlowTransparency(t) => ("shape.effects", json!({"glow": {"size": glow_r, "color": glow_c, "transparency": t}})),
            Edit::SoftEdge(r) if r <= 0.0 => ("shape.effects", json!({"softEdge": null})),
            Edit::SoftEdge(r) => ("shape.effects", json!({"softEdge": r})),
            Edit::Width(v) => ("picture.size", json!({"width": v * k, "lockAspect": keep})),
            Edit::Height(v) => ("picture.size", json!({"height": v * k, "lockAspect": keep})),
            Edit::KeepRatio(b) => {
                ui.ctx().data_mut(|d| d.insert_temp(ratio_key, b));
                continue;
            }
            Edit::Rotation(deg) => ("arrange.rotation", json!({"degrees": deg})),
            Edit::CornerRadius(r) => ("shape.cornerRadius", json!({"radius": r * k})),
            Edit::FlipVertical => ("arrange.rotate", json!({"direction": "flipVertical"})),
            Edit::FlipHorizontal => ("arrange.rotate", json!({"direction": "flipHorizontal"})),
            Edit::Align(edge) if matches!(edge, "left" | "center" | "right") => ("arrange.align", json!({"value": edge})),
            // Vertical alignment and stacking order: not offered (`caps`).
            Edit::Align(_) | Edit::Order(_) => continue,
        };
        let _ = app.run(id, params);
    }
}

/// A corner radius of `pt` points as the Shape pane's Corner radius box shows it: two decimals in
/// File › Options' unit ("0.85 cm", `0.90"`).
pub(crate) fn corner_number(pt: f32) -> String {
    let unit = crate::options::unit();
    let s = format!("{:.2}", wordcraft_geom::finite(pt) / unit.pt_per_unit());
    let sep = crate::options::decimal();
    let s = if sep == '.' { s } else { s.replace('.', &sep.to_string()) };
    format!("{s}{}", unit.suffix())
}

/// KhianKrub's side of the shared sections.
struct Host<'a> {
    app: &'a mut WordApp,
    theme: &'a [wordcraft_doc::Rgb],
    now: &'a ShapeNow,
    unit: wordcraft_geom::Unit,
}

impl krub_icons::shape_pane::Host for Host<'_> {
    fn color(&mut self, ui: &mut Ui, slot: krub_icons::shape_pane::Slot, enabled: bool) {
        use krub_icons::shape_pane::Slot;
        ui.add_enabled_ui(enabled, |ui| match slot {
            Slot::Fill => color(ui, self.app, self.theme, self.now.fill, "shape.fill"),
            Slot::Line => color(ui, self.app, self.theme, self.now.stroke, "shape.outline"),
            Slot::Glow => {
                let glow = self.now.effects.sanitized().glow.unwrap_or_default();
                let mut c = glow.color.hex();
                let was = c.clone();
                crate::dialogs::color_field(ui, self.theme, &mut c, "Automatic", "auto");
                if c != was
                    && let Some(rgb) = wordcraft_doc::Rgb::parse(&c)
                {
                    let _ =
                        self.app.run("shape.effects", json!({"glow": {"size": glow.size, "color": rgb.hex(), "transparency": glow.transparency}}));
                }
            }
        });
    }

    fn shadow(&mut self, ui: &mut Ui) {
        let full = ui.available_width().floor();
        wide_menu(ui, self.app, "shapeEffects", "Shadow", "shape.effects", full, |ui, app| {
            mi(ui, app, "No Shadow", "shape.effects", json!({"shadow": null}));
            ui.separator();
            for (id, label) in wordcraft_doc::effects::SHADOW_PRESETS {
                mi(ui, app, label, "shape.effects", json!({"shadow": id}));
            }
        });
    }

    fn button(&mut self, ui: &mut Ui, icon: &str, label: &str, _name: &str, width: f32) -> bool {
        let icon = match icon {
            "flipVertical" => "flip-v",
            "flipHorizontal" => "flip-h",
            other => other,
        };
        crate::widgets::pane_button(ui, self.app, icon, label, "", None, false, false, width).clicked()
    }

    fn number(&mut self, ui: &mut Ui, n: krub_icons::shape_pane::Number) -> Option<f32> {
        let mut v = n.value;
        let size = matches!(n.key, "width" | "height");
        // The corner radius is a length too, in the same unit.
        let measured = size || n.key == "corner";
        // A line may be flat (no height, or no width): its size boxes go down to 0 and drawing
        // them never writes a size back (the engine keeps other shapes at least 4 pt).
        let range = if size { 0.0..=*n.range.end() } else { n.range };
        let field = crate::numfield::NumField::new(&mut v).range(range).clamp_existing_to_range(!size).speed(f64::from(n.speed)).max_decimals(2);
        // Width and height in File › Options' unit, as everywhere else in KhianKrub.
        let field = if measured { field.measure(self.unit) } else { field.suffix(n.suffix) };
        let r = ui.add_enabled(n.enabled, field).labelled_by(n.label);
        if r.changed() && r.dragged() && !r.drag_started() {
            self.app.session.join_next_undo();
        }
        r.changed().then_some(v)
    }
}

pub fn show_shape(app: &mut WordApp, ui: &mut Ui) {
    if crate::panes::header(ui, "Shape") {
        close_pane(ui.ctx(), SHAPE_CLOSED);
        return;
    }
    let Some(now) = shape_now(app) else { return };
    locked_banner(app, ui);
    let theme = app.session.doc.settings.theme_colors.clone();
    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("shape_pane_scroll").show(ui, |ui| {
        // The sections shared by the three apps (owner, 2026-11-16; LibreOffice's object
        // Properties as the reference): Area, Line, Effect, Position and Size.
        // One module draws them in all three apps (`krub_icons::shape_pane`).
        shared_sections(app, ui, &theme, &now);
        // KhianKrub's own sections, each under the sub-panel it belongs to (owner, 2026-11-18):
        // the text's look with Fill and Line; how the shape sits in the text, the canvas and
        // its alternative text with Size and Position.
        use krub_icons::shape_pane::Tab;
        let tab = krub_icons::shape_pane::current_tab(ui.ctx(), SHAPE_CAPS);
        if tab == Tab::SizePosition {
            section(ui, "Text Wrap and Anchor", |ui| {
                let w = col_w(ui, 2);
                grid(ui, |ui| {
                    wide(ui, app, "position", "Position", "arrange.position", w);
                    wide_menu(ui, app, "wrapText", "Wrap Text", "arrange.wrapText", w, wrap_items);
                    lock_button(app, ui, 2.0 * w + GAP);
                });
            });
        }
        if tab != Tab::FillLine {
            if tab == Tab::SizePosition {
                canvas_and_alt_text(app, ui);
            }
            return;
        }
        section(ui, "Text", |ui| text(app, ui));
        section(ui, "WordArt Styles", |ui| {
            let outline = crate::wordart_menus::current_outline(app);
            let w = col_w(ui, 2);
            grid(ui, |ui| {
                wide_menu(ui, app, "wordArt", "Styles", "", w, |ui, app| crate::wordart_menus::style_gallery(ui, app, "wordArt.style"));
                wide_menu(ui, app, "textFill", "Text Fill", "", w, |ui, app| crate::wordart_menus::fill_menu(ui, app, &theme));
                wide_menu(ui, app, "textOutline", "Text Outline", "", w, |ui, app| crate::wordart_menus::outline_menu(ui, app, &theme, outline));
                wide_menu(ui, app, "transform", "Text Effects", "", w, |ui, app| crate::wordart_menus::effects_menu(ui, app, &theme, true));
            });
        });
    });
}

/// Shape › Size and Position: the drawing canvas's commands (when one is selected) and the
/// shape's alternative text.
fn canvas_and_alt_text(app: &mut WordApp, ui: &mut Ui) {
    if wordcraft_engine::cmd::canvas::selected_canvas(&app.session).is_some() {
        section(ui, "Drawing Canvas", |ui| {
            let w = col_w(ui, 3);
            grid(ui, |ui| {
                tile(ui, app, "canvas", "Fit", "canvas.fit", w);
                tile(ui, app, "canvas", "Expand", "canvas.expand", w);
                tile_menu(ui, app, "canvas", "Scale Drawing", "", w, |ui, app| {
                    for pct in [50, 75, 125, 150, 200] {
                        mi(ui, app, &format!("{pct}%"), "canvas.scale", json!({"factor": pct as f32 / 100.0}));
                    }
                });
            });
        });
    }
    section(ui, "Accessibility", |ui| alt_text(app, ui));
}

/// Shape › Text: the text box's Text Direction and Align Text, and Create Link (Break Link once
/// it is linked to another box).
fn text(app: &mut WordApp, ui: &mut Ui) {
    use wordcraft_doc::props::{TextVert, VAlign};
    let target = wordcraft_engine::cmd::objects::text_box_target(&app.session);
    let body = target.and_then(|(_, id)| app.session.doc.parts.get(&id).map(|p| p.body)).unwrap_or_default();
    let w = col_w(ui, 3);
    grid(ui, |ui| {
        tile_menu(ui, app, "textDirection", "Text Direction", "", w, |ui, app| {
            for (label, vert) in [
                ("Horizontal", TextVert::Horz),
                ("Rotate all text 90°", TextVert::Vert),
                ("Rotate all text 270°", TextVert::Vert270),
                ("Stacked", TextVert::Stacked),
            ] {
                mi_check(ui, app, label, body.vert == vert, "shape.textDirection", json!({"dir": vert.ooxml()}));
            }
        });
        tile_menu(ui, app, "alignText", "Align Text", "", w, |ui, app| {
            for (label, anchor) in [("Top", VAlign::Top), ("Middle", VAlign::Center), ("Bottom", VAlign::Bottom)] {
                mi_check(ui, app, label, body.anchor == anchor, "shape.alignText", json!({"anchor": wordcraft_doc::props::anchor_ooxml(anchor)}));
            }
        });
        if body.next.is_some() {
            tile(ui, app, "textBoxUnlink", "Break Link", "shape.breakLink", w);
        } else {
            let waiting = app.canvas.link_from.is_some();
            crate::widgets::pane_tile(ui, app, "textBoxLink", "Create Link", "ui.linkTextBox", Some(json!({})), waiting, false, w);
        }
    });
}

/// The Chart pane (owner, 2026-11-19): the shared chart pane (`krub_icons::chart_pane`). Its
/// Chart sub-panel holds Chart Layouts, Data and Type and Size; Fill and Line, Effects and Text
/// format the part chosen in its list, through `chart.format`.
pub fn show_chart(app: &mut WordApp, ui: &mut Ui) {
    if crate::panes::header(ui, "Chart") {
        close_pane(ui.ctx(), CHART_CLOSED);
        return;
    }
    let Some(chart) = wordcraft_engine::cmd::chart::selected_chart(&app.session) else { return };
    let theme = app.session.doc.settings.theme_colors.clone();
    let parts = chart_parts(&chart);
    locked_banner(app, ui);
    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("chart_pane_scroll").show(ui, |ui| {
        let mut host = ChartHost { app: &mut *app, theme: &theme, chart: &chart };
        let edits = krub_icons::chart_pane::show(ui, &parts, &chart.format, &|s| tl!(s).to_string(), &mut host);
        for (part, e) in edits {
            let (key, value) = e.to_pair();
            crate::widgets::run_from_ribbon(app, "chart.format", json!({"part": krub_chart_format::part_id(part), "edit": key, "value": value}));
        }
    });
}

/// The parts of `c` for the pane's list: the chart and plot areas, the title and legend when
/// shown, the axes and gridlines of a chart with axes, then each series by name.
pub(crate) fn chart_parts(c: &wordcraft_doc::chart::ChartSpec) -> Vec<(krub_chart_format::Part, String)> {
    use krub_chart_format::Part;
    let mut v = vec![(Part::ChartArea, tl!("Chart Area").to_string()), (Part::PlotArea, tl!("Plot Area").to_string())];
    if c.title.is_some() {
        v.push((Part::Title, tl!("Chart Title").to_string()));
    }
    if c.legend.is_some() {
        v.push((Part::Legend, tl!("Legend").to_string()));
    }
    if !c.kind.is_round() {
        v.push((Part::CategoryAxis, tl!("Horizontal Axis").to_string()));
        v.push((Part::ValueAxis, tl!("Vertical Axis").to_string()));
        v.push((Part::Gridlines, tl!("Gridlines").to_string()));
    }
    for (i, s) in c.series.iter().enumerate().take(krub_chart_format::MAX_SERIES) {
        let name = if s.name.trim().is_empty() { crate::i18n::fmt(tl!("Series {n}"), &[("n", &(i + 1).to_string())]) } else { s.name.clone() };
        v.push((Part::Series(i), name));
    }
    v
}

struct ChartHost<'a> {
    app: &'a mut WordApp,
    theme: &'a [wordcraft_doc::Rgb],
    chart: &'a wordcraft_doc::chart::ChartSpec,
}

impl krub_icons::chart_pane::Host for ChartHost<'_> {
    fn color(&mut self, ui: &mut Ui, key: &'static str, current: Option<[u8; 3]>, enabled: bool) -> Option<[u8; 3]> {
        let mut picked = None;
        ui.add_enabled_ui(enabled, |ui| {
            ui.push_id(key, |ui| {
                let mut c = current.map_or_else(|| "auto".to_string(), |c| wordcraft_doc::Rgb(c[0], c[1], c[2]).hex());
                let was = c.clone();
                crate::dialogs::color_field(ui, self.theme, &mut c, "Automatic", "auto");
                if c != was
                    && let Some(rgb) = wordcraft_doc::Rgb::parse(&c)
                {
                    picked = Some([rgb.0, rgb.1, rgb.2]);
                }
            });
        });
        picked
    }

    fn number(&mut self, ui: &mut Ui, n: krub_icons::shape_pane::Number) -> Option<f32> {
        let mut v = n.value;
        let field = crate::numfield::NumField::new(&mut v).range(n.range).speed(f64::from(n.speed)).max_decimals(2).suffix(n.suffix);
        let r = ui.add_enabled(n.enabled, field).labelled_by(n.label);
        // A drag is one undo step.
        if r.changed() && r.dragged() && !r.drag_started() {
            self.app.session.join_next_undo();
        }
        r.changed().then_some(v)
    }

    fn chart(&mut self, ui: &mut Ui) {
        let app = &mut *self.app;
        let chart = self.chart;
        section(ui, "Chart Layouts", |ui| {
            let w = col_w(ui, 3);
            grid(ui, |ui| {
                tile_menu(ui, app, "chartTitle", "Chart Title", "", w, |ui, app| {
                    mi_check(ui, app, "None", chart.title.is_none(), "chart.title", json!({"text": null}));
                    let text = chart.title.clone().unwrap_or_else(|| tl!("Chart Title").to_string());
                    mi_check(ui, app, "Above Chart", chart.title.is_some(), "chart.title", json!({"text": text}));
                });
                tile_menu(ui, app, "legend", "Legend", "", w, |ui, app| {
                    mi_check(ui, app, "None", chart.legend.is_none(), "chart.legend", json!({"pos": "none"}));
                    for (label, pos) in [
                        ("Right", wordcraft_doc::chart::LegendPos::Right),
                        ("Top", wordcraft_doc::chart::LegendPos::Top),
                        ("Left", wordcraft_doc::chart::LegendPos::Left),
                        ("Bottom", wordcraft_doc::chart::LegendPos::Bottom),
                    ] {
                        mi_check(ui, app, label, chart.legend == Some(pos), "chart.legend", json!({"pos": pos.id()}));
                    }
                });
                tile_menu(ui, app, "dataLabels", "Data Labels", "", w, |ui, app| {
                    mi_check(ui, app, "None", !chart.data_labels, "chart.dataLabels", json!({"on": false}));
                    mi_check(ui, app, "Show", chart.data_labels, "chart.dataLabels", json!({"on": true}));
                });
            });
        });
        section(ui, "Data and Type", |ui| {
            let w = col_w(ui, 2);
            grid(ui, |ui| {
                wide(ui, app, "chartData", "Edit Data", "chart.editData", w);
                wide(ui, app, "changeChartType", "Change Chart Type", "chart.type", w);
                lock_button(app, ui, 2.0 * w + GAP);
            });
        });
        section(ui, "Size", |ui| size(app, ui, 36.0, false));
    }
}

/// The gold line on top of an object's pane while it is locked, with its Unlock button (owner,
/// 2026-11-23). The pane's other buttons are greyed by `widgets::enabled`.
fn locked_banner(app: &mut WordApp, ui: &mut Ui) {
    if wordcraft_engine::cmd::objects::selection_locked(&app.session)
        && krub_icons::object_lock::pane_banner(ui, tl!("Locked: it can't be moved, resized or edited."), tl!("Unlock"))
    {
        let _ = app.run("shape.lock", json!({"locked": false}));
    }
}

/// The Lock button `w` wide: on (gold, closed padlock) while the object is locked.
fn lock_button(app: &mut WordApp, ui: &mut Ui, w: f32) {
    let locked = wordcraft_engine::cmd::objects::selection_locked(&app.session);
    let icon = if locked { "lock" } else { "unlock" };
    crate::widgets::pane_button(ui, app, icon, "Lock", "shape.lock", Some(json!({"locked": !locked})), locked, false, w);
}

/// Picture › Arrange in two even columns: Position, Wrap Text, Rotate and Align, then Group and
/// Lock (owner, 2026-11-22; Lock 2026-11-23).
fn arrange(app: &mut WordApp, ui: &mut Ui) {
    let w = col_w(ui, 2);
    grid(ui, |ui| {
        wide(ui, app, "position", "Position", "arrange.position", w);
        wide_menu(ui, app, "wrapText", "Wrap Text", "arrange.wrapText", w, wrap_items);
        wide_menu(ui, app, "rotate", "Rotate", "arrange.rotate", w, rotate_items);
        wide(ui, app, "align", "Align", "arrange.align", w);
        wide_menu(ui, app, "group", "Group", "arrange.group", w, |ui, app| {
            mi(ui, app, "Group", "arrange.group", json!({}));
            mi(ui, app, "Ungroup", "arrange.ungroup", json!({}));
        });
        lock_button(app, ui, w);
    });
}

/// Accessibility: Alt Text opens the Alt Text pane.
fn alt_text(app: &mut WordApp, ui: &mut Ui) {
    let full = ui.available_width().floor();
    crate::widgets::pane_button(ui, app, "altText", "Alt Text", "view.altTextPane", Some(json!({"value": true})), false, false, full);
}

pub fn show_picture(app: &mut WordApp, ui: &mut Ui) {
    if crate::panes::header(ui, "Picture") {
        close_pane(ui.ctx(), PICTURE_CLOSED);
        return;
    }
    locked_banner(app, ui);
    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("picture_pane_scroll").show(ui, |ui| {
        // Even grids with the ribbon's button face, under hairline headings (owner, 2026-11-22).
        faces(ui, |ui| {
            ruled(ui, "Adjust", |ui| picture_adjust(app, ui));
            ruled(ui, "Picture Styles", |ui| {
                let w = col_w(ui, 2);
                grid(ui, |ui| {
                    wide_menu(ui, app, "picStyles", "Styles", "picture.style", w, |ui, app| {
                        for (l, s) in [
                            ("Simple Frame", "simpleFrame"),
                            ("Thick Frame", "thickFrame"),
                            ("Rounded", "rounded"),
                            ("Soft Edge", "softEdge"),
                            ("Shadow", "shadow"),
                        ] {
                            mi(ui, app, l, "picture.style", json!({"style": s}));
                        }
                    });
                    wide_menu(ui, app, "picBorder", "Border", "picture.border", w, |ui, app| {
                        for (l, w) in [("Thin", 2), ("Medium", 4), ("Thick", 8)] {
                            mi(ui, app, l, "picture.border", json!({"width": w}));
                        }
                    });
                });
            });
            ruled(ui, "Arrange", |ui| arrange(app, ui));
            ruled(ui, "Size", |ui| picture_size(app, ui));
            ruled(ui, "Crop", |ui| picture_crop(app, ui));
            ruled(ui, "Accessibility", |ui| {
                let w = ui.available_width().floor();
                crate::widgets::pane_button(ui, app, "altText", "Alt Text", "view.altTextPane", Some(json!({"value": true})), false, false, w);
            });
        });
    });
}

/// Picture › Adjust: corrections, colour, icon colour and transparency, then Change and Reset, as
/// six tiles in three even columns.
fn picture_adjust(app: &mut WordApp, ui: &mut Ui) {
    let w = col_w(ui, 3);
    grid(ui, |ui| {
        tile_menu(ui, app, "picCorrections", "Corrections", "picture.corrections", w, |ui, app| {
            for (l, b, c) in
                [("Brighter +20%", 20.0, 0.0), ("Darker −20%", -20.0, 0.0), ("More Contrast +20%", 0.0, 20.0), ("Less Contrast −20%", 0.0, -20.0)]
            {
                mi(ui, app, l, "picture.corrections", json!({"brightness": b, "contrast": c}));
            }
            ui.separator();
            mi(ui, app, "Sharpen", "picture.corrections", json!({"sharpen": 40.0}));
            mi(ui, app, "Soften", "picture.corrections", json!({"sharpen": -40.0}));
        });
        tile_menu(ui, app, "picColor", "Color", "picture.color", w, |ui, app| {
            for (l, v) in [("Full Saturation 100%", 100.0), ("Muted 50%", 50.0), ("Gray 0%", 0.0), ("Vivid 200%", 200.0)] {
                mi(ui, app, l, "picture.color", json!({"mode": "saturation", "saturation": v}));
            }
            ui.separator();
            mi(ui, app, "Grayscale", "picture.color", json!({"mode": "grayscale"}));
            mi(ui, app, "Sepia", "picture.color", json!({"mode": "sepia"}));
            mi(ui, app, "Washout", "picture.color", json!({"mode": "washout"}));
        });
        tile_menu(ui, app, "icons", "Icon Color", "picture.iconColor", w, |ui, app| {
            let theme = app.session.doc.settings.theme_colors.clone();
            crate::colors::menu(ui, app, &theme, Target::cmd("picture.iconColor"));
        });
        tile_menu(ui, app, "picTransparency", "Transparency", "picture.transparency", w, |ui, app| {
            for v in [0.0, 15.0, 30.0, 50.0, 65.0, 80.0, 95.0] {
                mi(ui, app, &format!("{v:.0}%"), "picture.transparency", json!({"percent": v}));
            }
            ui.separator();
            // An exact value, dragged with a live preview: absolute from the original pixels, one
            // undo step for the whole drag. The current value isn't stored anywhere, so the field
            // reopens where it was last left for this picture.
            let (media, off) = match wordcraft_engine::cmd::objects::selected(&app.session) {
                Some((pos, wordcraft_doc::para::InlineObject::Image { media, .. })) => (media, pos.off),
                _ => (String::new(), 0),
            };
            let key = egui::Id::new(("picture_transparency", media, off));
            let mut v = ui.data(|d| d.get_temp::<f64>(key)).unwrap_or(0.0);
            let r = ui.add(crate::numfield::NumField::new(&mut v).speed(1.0).range(0.0..=100.0).suffix("%"));
            if r.changed() {
                ui.data_mut(|d| d.insert_temp(key, v));
                if r.dragged() && !r.drag_started() {
                    app.session.join_next_undo();
                }
                let _ = app.run("picture.transparency", json!({"percent": v}));
            }
        });
        tile(ui, app, "picChange", "Change Picture", "ui.changePicture", w);
        tile(ui, app, "picReset", "Reset Picture", "picture.reset", w);
    });
}

/// Picture › Size: width and height (`picture.size` keeps the aspect ratio) and rotation, side by
/// side, each named under its box.
fn picture_size(app: &mut WordApp, ui: &mut Ui) {
    let (w0, h0, deg0) = match wordcraft_engine::cmd::objects::selected(&app.session) {
        Some((_, o @ wordcraft_doc::para::InlineObject::Image { w, h, .. })) => (w, h, o.frame().map_or(0.0, |(_, _, f)| f.spin().deg)),
        _ => (0.0, 0.0, 0.0),
    };
    let unit = crate::options::unit();
    let k = unit.pt_per_unit();
    let w = col_w(ui, 3);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        for (label, key, v0) in [("Width", "width", w0), ("Height", "height", h0)] {
            let mut v = v0 / k;
            let r =
                field(ui, w, label, crate::numfield::NumField::new(&mut v).speed(1.0 / k).range(4.0 / k..=2000.0 / k).measure(unit).max_decimals(2));
            if r.changed() {
                if r.dragged() && !r.drag_started() {
                    app.session.join_next_undo();
                }
                let _ = app.run("picture.size", json!({key: v * k}));
            }
        }
        let mut deg = deg0;
        let r = field(ui, w, "Rotation", crate::numfield::NumField::new(&mut deg).speed(1.0).range(-360.0..=360.0).max_decimals(1).suffix("°"));
        if r.changed() {
            if r.dragged() && !r.drag_started() {
                app.session.join_next_undo();
            }
            let _ = app.run("arrange.rotation", json!({"degrees": deg}));
        }
    });
}

/// Picture › Crop: each side's crop in a two-by-two grid, then Crop and Reset Crop.
fn picture_crop(app: &mut WordApp, ui: &mut Ui) {
    let crop0 = match wordcraft_engine::cmd::objects::selected(&app.session) {
        Some((_, wordcraft_doc::para::InlineObject::Image { crop, .. })) => crop,
        _ => [0.0; 4],
    };
    let w = col_w(ui, 2);
    // crop0 is [left, top, right, bottom]; shown as top and bottom, then left and right.
    for pair in [[(1, "Top"), (3, "Bottom")], [(0, "Left"), (2, "Right")]] {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            for (i, label) in pair {
                let mut v = crop0.get(i).copied().unwrap_or(0.0) * 100.0;
                let r = field(ui, w, label, crate::numfield::NumField::new(&mut v).speed(0.5).range(0.0..=45.0).suffix("%"));
                if r.changed() {
                    if r.dragged() && !r.drag_started() {
                        app.session.join_next_undo();
                    }
                    let mut c = crop0;
                    if let Some(side) = c.get_mut(i) {
                        *side = (v / 100.0).clamp(0.0, 0.45);
                    }
                    let _ = app.run("picture.crop", json!({"left": c[0], "top": c[1], "right": c[2], "bottom": c[3]}));
                }
            }
        });
    }
    ui.add_space(2.0);
    let cropping = crate::crop::active(app).is_some();
    grid(ui, |ui| {
        crate::widgets::pane_button(ui, app, "crop", "Crop", "ui.crop", Some(json!({})), cropping, false, w);
        if crate::widgets::pane_button(ui, app, "picReset", "Reset Crop", "picture.crop", None, false, false, w).clicked() {
            let _ = app.run("picture.crop", json!({"left": 0.0, "top": 0.0, "right": 0.0, "bottom": 0.0}));
        }
    });
}

pub fn show_smart_art(app: &mut WordApp, ui: &mut Ui) {
    use wordcraft_doc::smart_art::{SmartArtColors, SmartArtLayout};
    if crate::panes::header(ui, "Diagram") {
        close_pane(ui.ctx(), SMART_ART_CLOSED);
        return;
    }
    let Some(g) = wordcraft_engine::cmd::smart_art::selected_smart_art(&app.session) else { return };
    let theme = app.session.doc.settings.theme_colors.clone();
    // The Text Pane's current item, else the last.
    let current = app.smart_art_item.filter(|i| *i < g.items.len());
    locked_banner(app, ui);
    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("smart_art_pane_scroll").show(ui, |ui| {
        section(ui, "Create Graphic", |ui| {
            let idx = current.map_or(json!({}), |i| json!({"index": i}));
            let after = current.map_or(json!({}), |i| json!({"after": i}));
            let text_pane = app.session.view.smart_art_pane;
            let w = col_w(ui, 2);
            grid(ui, |ui| {
                crate::widgets::pane_button(ui, app, "addShape", "Add Shape", "smartArt.addShape", Some(after), false, false, w);
                crate::widgets::pane_button(ui, app, "textPane", "Text Pane", "smartArt.textPane", Some(json!({})), text_pane, false, w);
                crate::widgets::pane_button(ui, app, "promote", "Promote", "smartArt.promote", Some(idx.clone()), false, false, w);
                crate::widgets::pane_button(ui, app, "demote", "Demote", "smartArt.demote", Some(idx), false, false, w);
            });
        });
        section(ui, "Layouts", |ui| {
            let t = Tokens::get(ui.ctx());
            // Tiles about 46 px wide, stretched so the columns fill the pane evenly.
            let cols = (((ui.available_width() + 4.0) / 50.0).floor() as usize).max(1);
            let tw = ((ui.available_width() - 4.0 * (cols as f32 - 1.0)) / cols as f32).floor().max(28.0);
            egui::Grid::new("smart_art_pane_layouts").spacing(vec2(4.0, 4.0)).show(ui, |ui| {
                for (k, layout) in SmartArtLayout::ALL.into_iter().enumerate() {
                    let (r, resp) = ui.allocate_exact_size(vec2(tw, 28.0), Sense::click());
                    let on = g.layout == layout;
                    let fill = if on {
                        t.checked
                    } else if resp.hovered() {
                        t.hover
                    } else {
                        t.ribbon
                    };
                    ui.painter().rect(r, 3.0, fill, Stroke::new(1.0, if on { t.accent } else { t.border }), egui::StrokeKind::Inside);
                    crate::dialogs_smart_art::preview(ui.painter(), r.shrink(2.0), layout, g.colors, &theme, t.text_dim);
                    let resp = resp.on_hover_text(tl!(layout.label()));
                    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, tl!(layout.label())));
                    if resp.clicked() && !on {
                        let _ = app.run("smartArt.layout", json!({"layout": layout.id()}));
                    }
                    if k % cols == cols - 1 {
                        ui.end_row();
                    }
                }
            });
        });
        section(ui, "Diagram Styles", |ui| {
            let w = col_w(ui, 2);
            grid(ui, |ui| {
                wide_menu(ui, app, "smartArtColors", "Change Colors", "", w, |ui, app| {
                    for colors in SmartArtColors::ALL {
                        let on = g.colors == colors;
                        let resp = ui.add(egui::Button::new(crate::dialogs_smart_art::colors_label(colors)).selected(on).min_size(vec2(200.0, 24.0)));
                        let sw = Rect::from_min_size(pos2(resp.rect.right() - 74.0, resp.rect.center().y - 6.0), vec2(66.0, 12.0));
                        crate::dialogs_smart_art::swatches(ui.painter(), sw, colors, &theme);
                        if resp.clicked() {
                            let _ = app.run("smartArt.colors", json!({"variant": colors.id()}));
                            ui.close();
                        }
                    }
                });
                wide(ui, app, "resetGraphic", "Reset Graphic", "smartArt.reset", w);
                lock_button(app, ui, 2.0 * w + GAP);
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;
    use wordcraft_engine::Session;

    fn window(app: WordApp) -> egui_kittest::Harness<'static, WordApp> {
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

    /// A selected shape opens the Shape pane and a selected chart the Chart pane, in place of the
    /// Shape Format and Chart Design tabs; each closes when the selection moves on.
    #[test]
    fn shapes_and_charts_open_their_panes() {
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.run("insert.shape", json!({"kind": "rectangle"})).unwrap();
        let mut h = window(app);
        assert!(h.query_by_label("Size and Position").is_some(), "the Shape pane is open");
        let tabs = crate::ribbon::contextual_tabs(&h.state().session);
        assert!(!tabs.contains(&"Shape Format"), "{tabs:?}");
        h.state_mut().run("select.collapse", json!({"end": true})).unwrap();
        h.state_mut().run("text.insert", json!({"text": "x"})).unwrap();
        h.run_steps(3);
        assert!(h.query_by_label("Size and Position").is_none(), "closed with nothing selected");
        h.state_mut().run("insert.chart", json!({"type": "column"})).unwrap();
        h.run_steps(3);
        assert!(h.query_by_label("Chart Layouts").is_some(), "the Chart pane is open");
        assert!(!crate::ribbon::contextual_tabs(&h.state().session).contains(&"Chart Design"));
        close_pane(&h.ctx, CHART_CLOSED);
        h.run_steps(3);
        assert!(h.query_by_label("Chart Layouts").is_none(), "its × closes it");
    }

    /// A selected picture opens the Picture pane and a SmartArt graphic the SmartArt pane, in
    /// place of the Picture Format and SmartArt Design tabs.
    #[test]
    fn pictures_and_smart_art_open_their_panes() {
        let picture = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/app-icon/khiankrub-192.png");
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.run("insert.picture", json!({"path": picture})).unwrap();
        let mut h = window(app);
        assert!(h.query_by_label("Picture Styles").is_some(), "the Picture pane is open");
        assert!(!crate::ribbon::contextual_tabs(&h.state().session).contains(&"Picture Format"));
        h.state_mut().run("select.collapse", json!({"end": true})).unwrap();
        h.state_mut().run("text.insert", json!({"text": "x"})).unwrap();
        h.run_steps(3);
        assert!(h.query_by_label("Picture Styles").is_none(), "closed with nothing selected");
        h.state_mut().run("insert.smartArt", json!({"layout": "process"})).unwrap();
        h.run_steps(3);
        assert!(h.query_by_label("Create Graphic").is_some(), "the SmartArt pane is open");
        assert!(!crate::ribbon::contextual_tabs(&h.state().session).contains(&"SmartArt Design"));
        // A layout tile in the pane changes the layout.
        h.get_by_label(tl!(wordcraft_doc::smart_art::SmartArtLayout::Cycle.label())).click();
        h.run_steps(3);
        let g = wordcraft_engine::cmd::smart_art::selected_smart_art(&h.state().session).unwrap();
        assert_eq!(g.layout, wordcraft_doc::smart_art::SmartArtLayout::Cycle);
    }

    /// The Picture pane sets its buttons and boxes in even grids (owner, 2026-11-22): Adjust's
    /// six tiles in three columns, Arrange in two with Group across both, Size's three boxes in
    /// one row, all inside the pane.
    #[test]
    fn the_picture_pane_sits_in_even_grids() {
        let picture = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/app-icon/khiankrub-192.png");
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.run("insert.picture", json!({"path": picture})).unwrap();
        let h = window(app);
        let rect = |l: &str| h.get_by_label(tl!(l)).rect();
        let even = |labels: &[&str]| {
            let r: Vec<Rect> = labels.iter().map(|l| rect(l)).collect();
            for x in &r {
                assert!((x.width() - r[0].width()).abs() < 0.5 && (x.top() - r[0].top()).abs() < 0.5, "{labels:?} are one even row");
            }
            r
        };
        let top = even(&["Corrections", "Color", "Icon Color"]);
        let bottom = even(&["Transparency", "Change Picture", "Reset Picture"]);
        for (a, b) in top.iter().zip(&bottom) {
            assert!((a.left() - b.left()).abs() < 0.5, "Adjust's columns line up");
        }
        let arrange = even(&["Position", "Wrap Text"]);
        // Group and Lock share the last row (owner, 2026-11-23).
        let last = even(&["Group", "Lock"]);
        assert!(
            (last[0].left() - arrange[0].left()).abs() < 0.5 && (last[1].right() - arrange[1].right()).abs() < 0.5,
            "Group and Lock fill the columns"
        );
        even(&["Width", "Height", "Rotation"]);
        let dock = egui::PanelState::load(&h.ctx, egui::Id::new("right_dock")).map(|p| p.outer_rect).unwrap();
        for l in ["Reset Picture", "Group", "Rotation", "Reset Crop", "Alt Text"] {
            let r = rect(l);
            assert!(r.left() >= dock.left() && r.right() <= dock.right() + 0.5, "{l} at {r:?} is outside the pane {dock:?}");
        }
    }

    /// The Shape pane's WordArt buttons stay a two-by-two grid however long their names run
    /// (owner, 2026-11-22: a long Thai name widened its button and pushed the row out of line).
    #[test]
    fn shape_pane_buttons_keep_their_grid() {
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.run("insert.shape", json!({"kind": "rectangle"})).unwrap();
        let h = window(app);
        let rect = |l: &str| h.get_by_label(tl!(l)).rect();
        let (a, b, c, d) = (rect("Styles"), rect("Text Fill"), rect("Text Outline"), rect("Text Effects"));
        for x in [b, c, d] {
            assert!((x.width() - a.width()).abs() < 0.5, "one width");
        }
        assert!((a.top() - b.top()).abs() < 0.5 && (c.top() - d.top()).abs() < 0.5, "two rows");
        assert!((a.left() - c.left()).abs() < 0.5 && (b.left() - d.left()).abs() < 0.5, "two columns");
    }
}
