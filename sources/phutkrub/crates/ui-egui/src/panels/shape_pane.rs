//! The Shape pane (owner, 2026-11-16): the object pane shared by the three KrubOffice apps,
//! [`krub_icons::shape_pane`] (Area, Line, Effect, Position and Size, with the same names and
//! order everywhere), then PhutKrub's own sections from the former Shape Format tab (Insert
//! Shapes, Quick Styles, WordArt Styles, Accessibility, More Formatting).
//!
//! PhutKrub stores every shared row.

use deckcraft_model::{Fill, Shape};
use egui::{Ui, vec2};
use krub_icons::shape_pane::{Caps, Dash, Edit, Host, Number, Slot, Values};
use serde_json::json;

use super::format_shape::{fill_transparency_to_apply, solid_fill_transparency_percent};
use super::{cref_param, effect_presets, run};
use crate::SlideApp;
use crate::icons::Icon;
use crate::widgets;

/// What PhutKrub can store of the shared rows.
pub(crate) const CAPS: Caps = Caps::ALL;

/// The id under which Keep ratio is remembered.
fn keep_id() -> egui::Id {
    egui::Id::unique("shape_pane_keep_ratio")
}

/// PhutKrub's colour boxes, shadow menu and buttons for the shared pane.
struct PhutKrub<'a> {
    app: &'a mut SlideApp,
    effects: deckcraft_model::style::Effects,
}

impl Host for PhutKrub<'_> {
    fn color(&mut self, ui: &mut Ui, slot: Slot, enabled: bool) {
        let sc = super::scheme(self.app);
        let (none, name) = match slot {
            Slot::Fill => (Some("No Fill"), "Fill color"),
            Slot::Line => (Some("No Outline"), "Line color"),
            Slot::Glow => (None, "Glow color"),
        };
        let r = ui.add_enabled_ui(enabled, |ui| widgets::drop_button(ui, tl!("Color"), vec2(80.0, 22.0), enabled)).inner;
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, tl!(name)));
        widgets::color_popup(&r).show(|ui| {
            if let Some(c) = widgets::color_grid(ui, &sc, none.map(|l| tl!(l))) {
                let (id, p) = match (slot, c) {
                    (Slot::Fill, Some(c)) => ("shape.fill", json!({"color": cref_param(&c)})),
                    (Slot::Fill, None) => ("shape.fill", json!({"none": true})),
                    (Slot::Line, Some(c)) => ("shape.line", json!({"color": cref_param(&c)})),
                    (Slot::Line, None) => ("shape.line", json!({"none": true})),
                    (Slot::Glow, c) => ("shape.effects", json!({"glowProps": {"color": c.as_ref().map(cref_param)}})),
                };
                run(self.app, id, p);
            }
        });
    }

    fn shadow(&mut self, ui: &mut Ui) {
        let enabled = crate::ribbon::enabled(self.app, "shape.effects");
        let (main, arrow) = widgets::small_button_ex(ui, Icon::ShapeEffects, "Shadow", enabled, false, true);
        if let Some(arrow) = arrow {
            let current = effect_presets::shadow_preset_id(&self.effects);
            let app = &mut *self.app;
            widgets::split_menu(&main, &arrow, |ui| {
                if ui.button(tl!("No Shadow")).clicked() {
                    run(app, "shape.effects", json!({"shadow": "none"}));
                    ui.close();
                }
                ui.separator();
                if let Some(v) = effect_presets::shadow_row(ui, current) {
                    run(app, "shape.effects", json!({"shadow": v}));
                    ui.close();
                }
            });
        }
    }

    fn button(&mut self, ui: &mut Ui, icon: &str, label: &str, name: &str) -> bool {
        let (icon, id) = match icon {
            "flipVertical" => (Icon::FlipV, "arrange.flipVertical"),
            "flipHorizontal" => (Icon::FlipH, "arrange.flipHorizontal"),
            "alignLeft" => (Icon::AlignObjLeft, "arrange.align"),
            "alignCenter" => (Icon::AlignObjCenter, "arrange.align"),
            "alignRight" => (Icon::AlignObjRight, "arrange.align"),
            "alignTop" => (Icon::AlignObjTop, "arrange.align"),
            "alignMiddle" => (Icon::AlignObjMiddle, "arrange.align"),
            "alignBottom" => (Icon::AlignObjBottom, "arrange.align"),
            "bringToFront" => (Icon::BringToFront, "arrange.bringToFront"),
            "bringForward" => (Icon::BringForward, "arrange.bringForward"),
            "sendBackward" => (Icon::SendBackward, "arrange.sendBackward"),
            _ => (Icon::SendToBack, "arrange.sendToBack"),
        };
        let enabled = crate::ribbon::enabled(self.app, id);
        widgets::small_button_named(ui, icon, label, name, enabled, false, false).0.clicked()
    }

    fn number(&mut self, ui: &mut Ui, n: Number) -> Option<f32> {
        // PhutKrub's own number box: type a value (another unit converts) or step it.
        let (lo, hi) = (*n.range.start(), *n.range.end());
        let id = format!("shape_pane_{}", n.key);
        ui.add_enabled_ui(n.enabled, |ui| widgets::spinner(ui, &id, n.name, f64::from(n.value), f64::from(n.speed), n.suffix, 80.0))
            .inner
            .map(|v| (v as f32).clamp(lo, hi))
    }
}

/// The shared pane's values for shape `sh`.
fn values(app: &SlideApp, ui: &Ui, sh: &Shape) -> Values {
    let l = sh.line.clone().unwrap_or_default();
    let fx = sh.effects.clone().unwrap_or_default();
    let dash = match &l.dash {
        Some(deckcraft_model::style::Dash::Preset { name }) => match name.as_str() {
            "dot" | "sysDot" => Dash::Dotted,
            "dash" | "sysDash" => Dash::Dashed,
            "lgDash" => Dash::LongDash,
            "dashDot" | "sysDashDot" | "lgDashDot" | "lgDashDotDot" | "sysDashDotDot" => Dash::DashDot,
            _ => Dash::Solid,
        },
        _ => Dash::Solid,
    };
    let x = app.session.active().map(|d| deckcraft_engine::cmd::xfrm_of(&d.doc, &d.selection, sh));
    let len = app.session.prefs.units;
    let ppu = len.points_per_unit();
    Values {
        open: sh.is_line(),
        has_fill: !matches!(sh.fill, Some(Fill::None)),
        fill_transparency: solid_fill_transparency_percent(sh.fill.as_ref()),
        has_line: !matches!(l.fill, Some(Fill::None)),
        dash,
        thickness: l.width.unwrap_or(0.75) as f32,
        line_transparency: match &l.fill {
            Some(Fill::Solid { color }) => ((1.0 - color.alpha()) * 100.0) as f32,
            _ => 0.0,
        },
        arrow_start: l.head.as_ref().is_some_and(|e| e.kind != "none"),
        arrow_end: l.tail.as_ref().is_some_and(|e| e.kind != "none"),
        has_glow: fx.glow.as_ref().is_some_and(|g| g.radius > 0.0),
        glow_radius: fx.glow.as_ref().map_or(0.0, |g| g.radius) as f32,
        glow_transparency: fx.glow.as_ref().map_or(0.0, |g| (1.0 - g.color.alpha()) * 100.0) as f32,
        soft_edge: fx.soft_edge.unwrap_or(0.0) as f32,
        width: x.map_or(0.0, |x| (x.w / ppu) as f32),
        height: x.map_or(0.0, |x| (x.h / ppu) as f32),
        size_suffix: len.suffix().to_string(),
        size_max: (20000.0 / ppu) as f32,
        rotation: x.map_or(0.0, |x| x.rot as f32),
        keep_ratio: ui.data(|d| d.get_temp(keep_id())).unwrap_or(false),
    }
}

/// Run an edit from the shared pane as PhutKrub's command.
fn apply(app: &mut SlideApp, ui: &Ui, sh: &Shape, v: &Values, edit: Edit) {
    let ppu = app.session.prefs.units.points_per_unit();
    let dash = |d: Dash| match d {
        Dash::Solid => "solid",
        Dash::Dotted => "sysDot",
        Dash::Dashed => "dash",
        Dash::LongDash => "lgDash",
        Dash::DashDot => "dashDot",
    };
    let arrow = |on: bool| if on { "triangle" } else { "none" };
    let to = {
        let selection = app.session.active().map(|d| d.selection.shapes.clone()).unwrap_or_default();
        if crate::ribbon::align_to_slide(app.align_to.as_ref(), &selection) { "slide" } else { "selection" }
    };
    let (id, p) = match edit {
        Edit::FillTransparency(t) => match fill_transparency_to_apply(t, solid_fill_transparency_percent(sh.fill.as_ref())) {
            Some(t) => ("shape.fill", json!({"transparency": t})),
            None => return,
        },
        Edit::LineNone => ("shape.line", json!({"none": true})),
        Edit::LineDash(d) if !v.has_line => ("shape.line", json!({"color": "tx1", "width": v.thickness, "dash": dash(d)})),
        Edit::LineDash(d) => ("shape.line", json!({"dash": dash(d)})),
        Edit::Thickness(w) => ("shape.line", json!({"width": w.max(0.0)})),
        Edit::LineTransparency(t) => ("shape.line", json!({"transparency": f64::from(t.clamp(0.0, 100.0)) / 100.0})),
        Edit::ArrowStart(on) => ("shape.line", json!({"head": arrow(on)})),
        Edit::ArrowEnd(on) => ("shape.line", json!({"tail": arrow(on)})),
        Edit::GlowRadius(r) if r <= 0.0 => ("shape.effects", json!({"glow": "none"})),
        Edit::GlowRadius(r) => ("shape.effects", json!({"glowProps": {"radius": r.min(150.0)}})),
        Edit::GlowTransparency(t) => ("shape.effects", json!({"glowProps": {"transparency": t / 100.0}})),
        Edit::SoftEdge(r) => ("shape.effects", json!({"softEdges": r.clamp(0.0, 100.0)})),
        Edit::Width(w) => ("shape.resize", json!({"w": f64::from(w) * ppu, "lockAspect": v.keep_ratio})),
        Edit::Height(h) => ("shape.resize", json!({"h": f64::from(h) * ppu, "lockAspect": v.keep_ratio})),
        Edit::KeepRatio(k) => {
            ui.ctx().data_mut(|d| d.insert_temp(keep_id(), k));
            return;
        }
        Edit::Rotation(r) => ("shape.rotate", json!({"deg": r})),
        Edit::FlipVertical => ("arrange.flipVertical", json!({})),
        Edit::FlipHorizontal => ("arrange.flipHorizontal", json!({})),
        Edit::Align(edge) => ("arrange.align", json!({"edge": edge, "to": to})),
        Edit::Order(op) => {
            run(app, &format!("arrange.{op}"), json!({}));
            return;
        }
    };
    run(app, id, p);
}

/// The Shape pane for the selected shapes (the first one's values are shown).
pub(crate) fn shape_pane(app: &mut SlideApp, ui: &mut Ui) {
    let Some(sh) = app.session.active().and_then(|d| d.selected_shapes().first().map(|s| (*s).clone())) else {
        return;
    };
    let v = values(app, ui, &sh);
    let tr = |s: &str| tl!(s).to_string();
    let edits = widgets::in_pane(|| {
        let mut host = PhutKrub { app: &mut *app, effects: sh.effects.clone().unwrap_or_default() };
        krub_icons::shape_pane::sections(ui, &v, CAPS, &tr, &mut host)
    });
    for edit in edits {
        apply(app, ui, &sh, &v, edit);
    }
    // PhutKrub's own: whether Align lines objects up with the slide or with each other (the
    // same choice as Arrange › Align on Home), then the former Shape Format tab's sections.
    let selection = app.session.active().map(|d| d.selection.shapes.clone()).unwrap_or_default();
    let to_slide = crate::ribbon::align_to_slide(app.align_to.as_ref(), &selection);
    ui.horizontal_wrapped(|ui| {
        for (label, slide) in [("Align to Slide", true), ("Align Selected Objects", false)] {
            if ui.radio(to_slide == slide, tl!(label)).clicked() {
                app.align_to = Some((selection.clone(), slide));
            }
        }
    });
    widgets::in_pane(|| crate::ribbon::shape_extras(app, ui));
}
