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
use crate::table_pane::{buttons, close_pane, follow, labeled, menu, section, toggle};
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

/// The shared sections (Area, Line, Effect, Position and Size) from `krub_icons::shape_pane`,
/// drawn with KhianKrub's colour boxes, number boxes, buttons and Shadow menu; their edits run
/// KhianKrub's commands. KhianKrub has no stacking order between shapes yet (no Arrange) and
/// aligns left, centre or right only.
fn shared_sections(app: &mut WordApp, ui: &mut Ui, theme: &[wordcraft_doc::Rgb], now: &ShapeNow) {
    use krub_icons::shape_pane::{self as sp, Caps, Dash, Edit, Values};
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
    };
    let caps = Caps { arrange: false, align_vertical: false, ..Caps::ALL };
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
            Edit::FlipVertical => ("arrange.rotate", json!({"direction": "flipVertical"})),
            Edit::FlipHorizontal => ("arrange.rotate", json!({"direction": "flipHorizontal"})),
            Edit::Align(edge) if matches!(edge, "left" | "center" | "right") => ("arrange.align", json!({"value": edge})),
            // Vertical alignment and stacking order: not offered (`caps`).
            Edit::Align(_) | Edit::Order(_) => continue,
        };
        let _ = app.run(id, params);
    }
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
        menu(ui, self.app, "shapeEffects", "Shadow", "shape.effects", |ui, app| {
            mi(ui, app, "No Shadow", "shape.effects", json!({"shadow": null}));
            ui.separator();
            for (id, label) in wordcraft_doc::effects::SHADOW_PRESETS {
                mi(ui, app, label, "shape.effects", json!({"shadow": id}));
            }
        });
    }

    fn button(&mut self, ui: &mut Ui, icon: &str, label: &str, _name: &str) -> bool {
        let icon = match icon {
            "flipVertical" => "flip-v",
            "flipHorizontal" => "flip-h",
            other => other,
        };
        crate::widgets::labeled_button(ui, self.app, icon, label, "", None, false, false, 0.0).clicked()
    }

    fn number(&mut self, ui: &mut Ui, n: krub_icons::shape_pane::Number) -> Option<f32> {
        let mut v = n.value;
        let field = crate::numfield::NumField::new(&mut v).range(n.range).speed(f64::from(n.speed)).max_decimals(2);
        // Width and height in File › Options' unit, as everywhere else in KhianKrub.
        let field = if matches!(n.key, "width" | "height") { field.measure(self.unit) } else { field.suffix(n.suffix) };
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
    let theme = app.session.doc.settings.theme_colors.clone();
    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("shape_pane_scroll").show(ui, |ui| {
        // The sections shared by the three apps (owner, 2026-11-16; LibreOffice's object
        // Properties as the reference): Area, Line, Effect, Position and Size.
        // One module draws them in all three apps (`krub_icons::shape_pane`).
        shared_sections(app, ui, &theme, &now);
        // KhianKrub's own: how the shape sits in the text (LibreOffice's toolbar row).
        section(ui, "Text Wrap and Anchor", |ui| {
            buttons(ui, |ui| {
                labeled(ui, app, "position", "Position", "arrange.position");
                menu(ui, app, "wrapText", "Wrap Text", "arrange.wrapText", wrap_items);
            });
        });
        section(ui, "Text", |ui| text(app, ui));
        section(ui, "WordArt Styles", |ui| {
            let outline = crate::wordart_menus::current_outline(app);
            buttons(ui, |ui| {
                menu(ui, app, "wordArt", "Styles", "", |ui, app| crate::wordart_menus::style_gallery(ui, app, "wordArt.style"));
                menu(ui, app, "textFill", "Text Fill", "", |ui, app| crate::wordart_menus::fill_menu(ui, app, &theme));
                menu(ui, app, "textOutline", "Text Outline", "", |ui, app| crate::wordart_menus::outline_menu(ui, app, &theme, outline));
                menu(ui, app, "transform", "Text Effects", "", |ui, app| crate::wordart_menus::effects_menu(ui, app, &theme, true));
            });
        });
        if wordcraft_engine::cmd::canvas::selected_canvas(&app.session).is_some() {
            section(ui, "Drawing Canvas", |ui| {
                buttons(ui, |ui| {
                    labeled(ui, app, "canvas", "Fit", "canvas.fit");
                    labeled(ui, app, "canvas", "Expand", "canvas.expand");
                    menu(ui, app, "canvas", "Scale Drawing", "", |ui, app| {
                        for pct in [50, 75, 125, 150, 200] {
                            mi(ui, app, &format!("{pct}%"), "canvas.scale", json!({"factor": pct as f32 / 100.0}));
                        }
                    });
                });
            });
        }
        section(ui, "Accessibility", |ui| alt_text(app, ui));
    });
}

/// Shape › Text: the text box's Text Direction and Align Text, and Create Link (Break Link once
/// it is linked to another box).
fn text(app: &mut WordApp, ui: &mut Ui) {
    use wordcraft_doc::props::{TextVert, VAlign};
    let target = wordcraft_engine::cmd::objects::text_box_target(&app.session);
    let body = target.and_then(|(_, id)| app.session.doc.parts.get(&id).map(|p| p.body)).unwrap_or_default();
    buttons(ui, |ui| {
        menu(ui, app, "textDirection", "Text Direction", "", |ui, app| {
            for (label, vert) in [
                ("Horizontal", TextVert::Horz),
                ("Rotate all text 90°", TextVert::Vert),
                ("Rotate all text 270°", TextVert::Vert270),
                ("Stacked", TextVert::Stacked),
            ] {
                mi_check(ui, app, label, body.vert == vert, "shape.textDirection", json!({"dir": vert.ooxml()}));
            }
        });
        menu(ui, app, "alignText", "Align Text", "", |ui, app| {
            for (label, anchor) in [("Top", VAlign::Top), ("Middle", VAlign::Center), ("Bottom", VAlign::Bottom)] {
                mi_check(ui, app, label, body.anchor == anchor, "shape.alignText", json!({"anchor": wordcraft_doc::props::anchor_ooxml(anchor)}));
            }
        });
        if body.next.is_some() {
            labeled(ui, app, "textBoxUnlink", "Break Link", "shape.breakLink");
        } else {
            let waiting = app.canvas.link_from.is_some();
            toggle(ui, app, "textBoxLink", "Create Link", "ui.linkTextBox", waiting);
        }
    });
}

pub fn show_chart(app: &mut WordApp, ui: &mut Ui) {
    if crate::panes::header(ui, "Chart") {
        close_pane(ui.ctx(), CHART_CLOSED);
        return;
    }
    let Some(chart) = wordcraft_engine::cmd::chart::selected_chart(&app.session) else { return };
    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("chart_pane_scroll").show(ui, |ui| {
        section(ui, "Chart Layouts", |ui| {
            buttons(ui, |ui| {
                menu(ui, app, "chartTitle", "Chart Title", "", |ui, app| {
                    mi_check(ui, app, "None", chart.title.is_none(), "chart.title", json!({"text": null}));
                    let text = chart.title.clone().unwrap_or_else(|| tl!("Chart Title").to_string());
                    mi_check(ui, app, "Above Chart", chart.title.is_some(), "chart.title", json!({"text": text}));
                });
                menu(ui, app, "legend", "Legend", "", |ui, app| {
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
                menu(ui, app, "dataLabels", "Data Labels", "", |ui, app| {
                    mi_check(ui, app, "None", !chart.data_labels, "chart.dataLabels", json!({"on": false}));
                    mi_check(ui, app, "Show", chart.data_labels, "chart.dataLabels", json!({"on": true}));
                });
            });
        });
        section(ui, "Data and Type", |ui| {
            buttons(ui, |ui| {
                labeled(ui, app, "chartData", "Edit Data", "chart.editData");
                labeled(ui, app, "changeChartType", "Change Chart Type", "chart.type");
            });
        });
        section(ui, "Size", |ui| size(app, ui, 36.0, false));
    });
}

/// Arrange: Position, Wrap Text, Rotate and Align, and for a picture Group.
fn arrange(app: &mut WordApp, ui: &mut Ui, grouping: bool) {
    buttons(ui, |ui| {
        labeled(ui, app, "position", "Position", "arrange.position");
        menu(ui, app, "wrapText", "Wrap Text", "arrange.wrapText", wrap_items);
        menu(ui, app, "rotate", "Rotate", "arrange.rotate", rotate_items);
        labeled(ui, app, "align", "Align", "arrange.align");
        if grouping {
            menu(ui, app, "group", "Group", "arrange.group", |ui, app| {
                mi(ui, app, "Group", "arrange.group", json!({}));
                mi(ui, app, "Ungroup", "arrange.ungroup", json!({}));
            });
        }
    });
}

/// Accessibility: Alt Text opens the Alt Text pane.
fn alt_text(app: &mut WordApp, ui: &mut Ui) {
    buttons(ui, |ui| {
        crate::widgets::labeled_button(ui, app, "altText", "Alt Text", "view.altTextPane", Some(json!({"value": true})), false, false, 0.0);
    });
}

pub fn show_picture(app: &mut WordApp, ui: &mut Ui) {
    if crate::panes::header(ui, "Picture") {
        close_pane(ui.ctx(), PICTURE_CLOSED);
        return;
    }
    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("picture_pane_scroll").show(ui, |ui| {
        section(ui, "Adjust", |ui| picture_adjust(app, ui));
        section(ui, "Picture Styles", |ui| {
            buttons(ui, |ui| {
                menu(ui, app, "picStyles", "Styles", "picture.style", |ui, app| {
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
                menu(ui, app, "picBorder", "Border", "picture.border", |ui, app| {
                    for (l, w) in [("Thin", 2), ("Medium", 4), ("Thick", 8)] {
                        mi(ui, app, l, "picture.border", json!({"width": w}));
                    }
                });
            });
        });
        section(ui, "Arrange", |ui| arrange(app, ui, true));
        section(ui, "Size", |ui| picture_size(app, ui));
        section(ui, "Accessibility", |ui| alt_text(app, ui));
    });
}

/// Picture › Adjust: corrections, colour, icon colour, transparency, and Change and Reset.
fn picture_adjust(app: &mut WordApp, ui: &mut Ui) {
    buttons(ui, |ui| {
        menu(ui, app, "picCorrections", "Corrections", "picture.corrections", |ui, app| {
            for (l, b, c) in
                [("Brighter +20%", 20.0, 0.0), ("Darker −20%", -20.0, 0.0), ("More Contrast +20%", 0.0, 20.0), ("Less Contrast −20%", 0.0, -20.0)]
            {
                mi(ui, app, l, "picture.corrections", json!({"brightness": b, "contrast": c}));
            }
            ui.separator();
            mi(ui, app, "Sharpen", "picture.corrections", json!({"sharpen": 40.0}));
            mi(ui, app, "Soften", "picture.corrections", json!({"sharpen": -40.0}));
        });
        menu(ui, app, "picColor", "Color", "picture.color", |ui, app| {
            for (l, v) in [("Full Saturation 100%", 100.0), ("Muted 50%", 50.0), ("Gray 0%", 0.0), ("Vivid 200%", 200.0)] {
                mi(ui, app, l, "picture.color", json!({"mode": "saturation", "saturation": v}));
            }
            ui.separator();
            mi(ui, app, "Grayscale", "picture.color", json!({"mode": "grayscale"}));
            mi(ui, app, "Sepia", "picture.color", json!({"mode": "sepia"}));
            mi(ui, app, "Washout", "picture.color", json!({"mode": "washout"}));
        });
        menu(ui, app, "icons", "Icon Color", "picture.iconColor", |ui, app| {
            let theme = app.session.doc.settings.theme_colors.clone();
            crate::colors::menu(ui, app, &theme, Target::cmd("picture.iconColor"));
        });
        menu(ui, app, "picTransparency", "Transparency", "picture.transparency", |ui, app| {
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
        labeled(ui, app, "picChange", "Change Picture", "ui.changePicture");
        labeled(ui, app, "picReset", "Reset Picture", "picture.reset");
    });
}

/// Picture › Size: width and height (`picture.size` keeps the aspect ratio), rotation, and Crop
/// with each side's crop.
fn picture_size(app: &mut WordApp, ui: &mut Ui) {
    let (w0, h0, crop0) = match wordcraft_engine::cmd::objects::selected(&app.session) {
        Some((_, wordcraft_doc::para::InlineObject::Image { w, h, crop, .. })) => (w, h, crop),
        _ => (0.0, 0.0, [0.0; 4]),
    };
    egui::Grid::new("picture_pane_size").num_columns(2).spacing(vec2(8.0, 4.0)).show(ui, |ui| {
        for (label, key, v0) in [("Width:", "width", w0), ("Height:", "height", h0)] {
            ui.label(egui::RichText::new(tl!(label)).small());
            let (r, v) = size_box(ui, v0, 4.0, 2000.0);
            if r.changed() {
                if r.dragged() && !r.drag_started() {
                    app.session.join_next_undo();
                }
                let _ = app.run("picture.size", json!({key: v}));
            }
            ui.end_row();
        }
    });
    ui.add_space(2.0);
    ui.horizontal(|ui| rotation_field(ui, app));
    ui.add_space(4.0);
    let cropping = crate::crop::active(app).is_some();
    buttons(ui, |ui| {
        toggle(ui, app, "crop", "Crop", "ui.crop", cropping);
        if ui.button(egui::RichText::new(tl!("Reset Crop")).small()).clicked() {
            let _ = app.run("picture.crop", json!({"left": 0.0, "top": 0.0, "right": 0.0, "bottom": 0.0}));
        }
    });
    egui::Grid::new("picture_pane_crop").num_columns(4).spacing(vec2(6.0, 4.0)).show(ui, |ui| {
        for (i, side) in ["L", "T", "R", "B"].iter().enumerate() {
            ui.label(egui::RichText::new(crate::i18n::tc("crop side", side)).small());
            let mut v = crop0[i] * 100.0;
            let r = ui.add(crate::numfield::NumField::new(&mut v).speed(0.5).range(0.0..=45.0).suffix("%"));
            if r.changed() {
                if r.dragged() && !r.drag_started() {
                    app.session.join_next_undo();
                }
                let mut c = crop0;
                c[i] = (v / 100.0).clamp(0.0, 0.45);
                let _ = app.run("picture.crop", json!({"left": c[0], "top": c[1], "right": c[2], "bottom": c[3]}));
            }
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
}

pub fn show_smart_art(app: &mut WordApp, ui: &mut Ui) {
    use wordcraft_doc::smart_art::{SmartArtColors, SmartArtLayout};
    if crate::panes::header(ui, "SmartArt") {
        close_pane(ui.ctx(), SMART_ART_CLOSED);
        return;
    }
    let Some(g) = wordcraft_engine::cmd::smart_art::selected_smart_art(&app.session) else { return };
    let theme = app.session.doc.settings.theme_colors.clone();
    // The Text Pane's current item, else the last.
    let current = app.smart_art_item.filter(|i| *i < g.items.len());
    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("smart_art_pane_scroll").show(ui, |ui| {
        section(ui, "Create Graphic", |ui| {
            let idx = current.map_or(json!({}), |i| json!({"index": i}));
            let after = current.map_or(json!({}), |i| json!({"after": i}));
            let text_pane = app.session.view.smart_art_pane;
            buttons(ui, |ui| {
                crate::widgets::labeled_button(ui, app, "addShape", "Add Shape", "smartArt.addShape", Some(after), false, false, 0.0);
                crate::widgets::labeled_button(ui, app, "promote", "Promote", "smartArt.promote", Some(idx.clone()), false, false, 0.0);
                crate::widgets::labeled_button(ui, app, "demote", "Demote", "smartArt.demote", Some(idx), false, false, 0.0);
                toggle(ui, app, "textPane", "Text Pane", "smartArt.textPane", text_pane);
            });
        });
        section(ui, "Layouts", |ui| {
            let t = Tokens::get(ui.ctx());
            let cols = (((ui.available_width() + 2.0) / 48.0).floor() as usize).max(1);
            egui::Grid::new("smart_art_pane_layouts").spacing(vec2(2.0, 2.0)).show(ui, |ui| {
                for (k, layout) in SmartArtLayout::ALL.into_iter().enumerate() {
                    let (r, resp) = ui.allocate_exact_size(vec2(46.0, 28.0), Sense::click());
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
        section(ui, "SmartArt Styles", |ui| {
            buttons(ui, |ui| {
                menu(ui, app, "smartArtColors", "Change Colors", "", |ui, app| {
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
                labeled(ui, app, "resetGraphic", "Reset Graphic", "smartArt.reset");
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
        assert!(h.query_by_label("Position and Size").is_some(), "the Shape pane is open");
        let tabs = crate::ribbon::contextual_tabs(&h.state().session);
        assert!(!tabs.contains(&"Shape Format"), "{tabs:?}");
        h.state_mut().run("select.collapse", json!({"end": true})).unwrap();
        h.state_mut().run("text.insert", json!({"text": "x"})).unwrap();
        h.run_steps(3);
        assert!(h.query_by_label("Position and Size").is_none(), "closed with nothing selected");
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
}
