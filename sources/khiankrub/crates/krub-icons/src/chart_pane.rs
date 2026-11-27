//! The chart pane, the same in every KrubOffice app (owner, 2026-11-19, from the approved draft
//! after Word's Format Chart Area pane): a list of the chart's parts at the top (Chart Area,
//! Plot Area, Chart Title, Legend, the axes, the gridlines, each series), then four sub-panels
//! in a segmented control:
//!
//! - **Chart**: the app's own sections (layouts, data and type, size, arrange), [`Host::chart`];
//! - **Fill and Line**: Fill (No fill, Solid, Automatic; colour, transparency) and Border (No
//!   line, Solid, Automatic; colour, transparency, width, compound and dash drawn as lines) and,
//!   for the chart area, Rounded corners;
//! - **Effects**: Shadow, Glow and Soft Edges;
//! - **Text**: Text Fill, Text Outline and the font size, for the parts that show text.
//!
//! The format itself is `krub_chart_format`; this returns the edits the person made, which the
//! app runs as its own command. 3-D Format and 3-D Rotation are left out: no app draws charts in
//! 3-D. The apps run different egui versions, so this uses only what they share.

use egui::Ui;
use krub_chart_format::{Color, Compound, Dash, Edit, Fill, Format, Formats, Kind, Part};

use crate::line_style::{Item, Look, combo};
use crate::shape_pane::Number;

/// What each app draws its own way.
pub trait Host {
    /// A colour box showing `current`; `Some(colour)` when one is picked. `key` is unique to the
    /// box ("fill", "border", "shadow", "glow", "text", "outline").
    fn color(&mut self, ui: &mut Ui, key: &'static str, current: Option<Color>, enabled: bool) -> Option<Color>;
    /// A number box; `Some(v)` when the person changed it.
    fn number(&mut self, ui: &mut Ui, n: Number) -> Option<f32> {
        crate::shape_pane::drag_number(ui, n)
    }
    /// The Chart sub-panel: the app's own sections.
    fn chart(&mut self, ui: &mut Ui);
}

/// The English names the pane shows (for the apps' translation tables and tests).
pub const NAMES: [&str; 44] = [
    "Chart",
    "Fill and Line",
    "Effects",
    "Text",
    "Format:",
    "Fill",
    "Border",
    "No fill",
    "No line",
    "Solid",
    "Automatic",
    "Color:",
    "Transparency:",
    "Width:",
    "Compound type:",
    "Dash type:",
    "Rounded corners",
    "Shadow",
    "Glow",
    "Soft Edges",
    "Blur:",
    "Angle:",
    "Distance:",
    "Size:",
    "Text Fill",
    "Text Outline",
    "Font",
    "Font size:",
    "Reset to Automatic",
    "Chart Area",
    "Plot Area",
    "Chart Title",
    "Legend",
    "Horizontal Axis",
    "Vertical Axis",
    "Gridlines",
    "Single",
    "Double",
    "Thick Thin",
    "Thin Thick",
    "Triple",
    "Dotted",
    "Dashed",
    "Long dash",
];

/// The sub-panels, in the segmented control's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Chart,
    FillLine,
    Effects,
    Text,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Chart, Tab::FillLine, Tab::Effects, Tab::Text];

    fn icon(self) -> &'static str {
        match self {
            Tab::Chart => "chart",
            Tab::FillLine => "shape-fill",
            Tab::Effects => "effects",
            Tab::Text => "fontcolor",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Tab::Chart => "Chart",
            Tab::FillLine => "Fill and Line",
            Tab::Effects => "Effects",
            Tab::Text => "Text",
        }
    }
}

fn tab_id() -> egui::Id {
    crate::pane_ui::id("krub_chart_tab")
}

fn part_id() -> egui::Id {
    crate::pane_ui::id("krub_chart_part")
}

/// The sub-panel shown now.
pub fn current_tab(ctx: &egui::Context) -> Tab {
    let i = ctx.data_mut(|d| *d.get_persisted_mut_or(tab_id(), 0usize));
    Tab::ALL.get(i).copied().unwrap_or(Tab::Chart)
}

/// Shows sub-panel `tab` next time (as a click on its slot does).
pub fn set_tab(ctx: &egui::Context, tab: Tab) {
    let i = Tab::ALL.iter().position(|t| *t == tab).unwrap_or(0);
    ctx.data_mut(|d| d.insert_persisted(tab_id(), i));
}

/// The part chosen in the list, among `parts`.
pub fn current_part(ctx: &egui::Context, parts: &[(Part, String)]) -> Part {
    let chosen = ctx.data_mut(|d| d.get_persisted::<String>(part_id())).and_then(|s| krub_chart_format::part_from_id(&s));
    match chosen {
        Some(p) if parts.iter().any(|(q, _)| *q == p) => p,
        _ => parts.first().map_or(Part::ChartArea, |(p, _)| *p),
    }
}

/// Chooses `part` in the list (as picking it does).
pub fn set_part(ctx: &egui::Context, part: Part) {
    ctx.data_mut(|d| d.insert_persisted(part_id(), krub_chart_format::part_id(part)));
}

fn section(ui: &mut Ui, key: &str, title: String, add: impl FnOnce(&mut Ui)) {
    crate::pane_ui::section(ui, crate::pane_ui::id(("krub_chart_section", key)), &title, true, add);
}

fn rows(id: &str) -> egui::Grid {
    egui::Grid::new(id).num_columns(2).spacing(egui::vec2(8.0, crate::pane_ui::ROW_GAP)).min_col_width(crate::pane_ui::LABEL_W)
}

/// A row of choices (No fill, Solid, Automatic) as small selectable buttons.
fn kinds(ui: &mut Ui, current: Kind, none: &str, tr: &dyn Fn(&str) -> String) -> Option<Kind> {
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (k, name) in [(Kind::None, none), (Kind::Solid, "Solid"), (Kind::Auto, "Automatic")] {
            if ui.add(egui::Button::selectable(current == k, tr(name))).clicked() && current != k {
                picked = Some(k);
            }
        }
    });
    picked
}

/// The segmented control, the list of parts and the chosen sub-panel. `parts` are the chart's
/// parts with their (translated) names, the chart area first. Returns this frame's edits.
pub fn show(ui: &mut Ui, parts: &[(Part, String)], formats: &Formats, tr: &dyn Fn(&str) -> String, host: &mut dyn Host) -> Vec<(Part, Edit)> {
    let mut edits = Vec::new();
    let segments: Vec<crate::pane_ui::Segment> = Tab::ALL.iter().map(|t| crate::pane_ui::Segment { icon: t.icon(), label: tr(t.name()) }).collect();
    let tab = Tab::ALL.get(crate::pane_ui::segmented(ui, tab_id(), &segments, 0)).copied().unwrap_or(Tab::Chart);
    if tab == Tab::Chart {
        host.chart(ui);
        return edits;
    }
    let part = current_part(ui.ctx(), parts);
    ui.horizontal(|ui| {
        let label = ui.label(tr("Format:")).id;
        let items: Vec<Item<Part>> = parts.iter().map(|(p, n)| Item::new(*p, None, n.as_str())).collect();
        let (resp, picked) = combo(ui, crate::pane_ui::id("krub_chart_parts"), 170.0, Some(part), &items);
        let _ = resp.labelled_by(label);
        if let Some(p) = picked {
            set_part(ui.ctx(), p);
        }
    });
    let part = current_part(ui.ctx(), parts);
    let f = formats.get(part);
    let mut out = |e: Edit| edits.push((part, e));
    match tab {
        Tab::Chart => {}
        Tab::FillLine => {
            if part.has_fill() {
                ui.push_id("krub_chart_fill", |ui| section(ui, "fill", tr("Fill"), |ui| fill(ui, &f, tr, host, &mut out)));
            }
            let title = if part.has_fill() { tr("Border") } else { tr("Line") };
            ui.push_id("krub_chart_border", |ui| section(ui, "border", title, |ui| border(ui, &f, tr, host, &mut out)));
            if part == Part::ChartArea {
                let mut on = formats.rounded_corners;
                if ui.checkbox(&mut on, tr("Rounded corners")).changed() {
                    out(Edit::RoundedCorners(on));
                }
            }
        }
        Tab::Effects => {
            if part.has_effects() {
                ui.push_id("krub_chart_shadow", |ui| section(ui, "shadow", tr("Shadow"), |ui| shadow(ui, &f, tr, host, &mut out)));
                ui.push_id("krub_chart_glow", |ui| section(ui, "glow", tr("Glow"), |ui| glow(ui, &f, tr, host, &mut out)));
                ui.push_id("krub_chart_soft", |ui| section(ui, "soft", tr("Soft Edges"), |ui| soft(ui, &f, tr, host, &mut out)));
            } else {
                ui.label(egui::RichText::new(tr("Gridlines have no effects.")).weak());
            }
        }
        Tab::Text => {
            if part.has_text() {
                ui.push_id("krub_chart_text_fill", |ui| section(ui, "text_fill", tr("Text Fill"), |ui| text_fill(ui, &f, tr, host, &mut out)));
                ui.push_id("krub_chart_outline", |ui| section(ui, "outline", tr("Text Outline"), |ui| outline(ui, &f, tr, host, &mut out)));
                ui.push_id("krub_chart_font", |ui| section(ui, "font", tr("Font"), |ui| font(ui, &f, tr, host, &mut out)));
            } else {
                ui.label(egui::RichText::new(tr("This part shows no text.")).weak());
            }
        }
    }
    if !f.is_auto() {
        ui.add_space(4.0);
        // Across the pane, like the other panes' last actions (owner, 2026-11-22).
        let w = ui.available_width().floor();
        if ui.add_sized(egui::vec2(w, 28.0), egui::Button::new(tr("Reset to Automatic"))).clicked() {
            out(Edit::Reset);
        }
    }
    edits
}

/// A number row: label, then the app's box. `Some(v)` when changed.
#[allow(clippy::too_many_arguments)]
fn number(
    ui: &mut Ui,
    host: &mut dyn Host,
    tr: &dyn Fn(&str) -> String,
    key: &'static str,
    label: &str,
    value: f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
    enabled: bool,
) -> Option<f32> {
    let id = ui.label(tr(label)).id;
    let name = tr(label.trim_end_matches(':'));
    let speed = if suffix == "%" || suffix == "°" { 1.0 } else { 0.25 };
    let r = host.number(ui, Number { key, label: id, name: &name, value, range, speed, suffix, enabled });
    ui.end_row();
    r
}

fn color_row(
    ui: &mut Ui,
    host: &mut dyn Host,
    tr: &dyn Fn(&str) -> String,
    key: &'static str,
    current: Option<Color>,
    enabled: bool,
) -> Option<Color> {
    ui.label(tr("Color:"));
    let r = host.color(ui, key, current, enabled);
    ui.end_row();
    r
}

fn fill(ui: &mut Ui, f: &Format, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, out: &mut dyn FnMut(Edit)) {
    if let Some(k) = kinds(ui, f.fill.kind(), "No fill", tr) {
        out(Edit::Fill(k));
    }
    let solid = f.fill.solid();
    rows("krub_chart_fill_rows").show(ui, |ui| {
        if let Some(c) = color_row(ui, host, tr, "fill", solid.map(|(c, _)| c), true) {
            out(Edit::FillColor(c));
        }
        let t = match f.fill {
            Fill::Solid { transparency, .. } => transparency,
            _ => 0.0,
        };
        if let Some(v) = number(ui, host, tr, "fill_transparency", "Transparency:", t, 0.0..=100.0, "%", solid.is_some()) {
            out(Edit::FillTransparency(v));
        }
    });
}

fn compound_look(c: Compound) -> Look {
    match c {
        Compound::Single => Look::Solid,
        Compound::Double => Look::Double,
        Compound::ThickThin => Look::ThickThin,
        Compound::ThinThick => Look::ThinThick,
        Compound::Triple => Look::ThickBetweenThin,
    }
}

fn dash_look(d: Dash) -> Look {
    match d {
        Dash::Solid => Look::Solid,
        Dash::Dotted => Look::Dotted,
        Dash::Dashed => Look::Dashed,
        Dash::LongDash => Look::LongDash,
        Dash::DashDot => Look::DashDot,
    }
}

fn border(ui: &mut Ui, f: &Format, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, out: &mut dyn FnMut(Edit)) {
    if let Some(k) = kinds(ui, f.border.kind(), "No line", tr) {
        out(Edit::Border(k));
    }
    let line = f.border.line().copied();
    let on = line.is_some();
    let l = line.unwrap_or_default();
    rows("krub_chart_border_rows").show(ui, |ui| {
        if let Some(c) = color_row(ui, host, tr, "border", line.map(|l| l.color), true) {
            out(Edit::BorderColor(c));
        }
        if let Some(v) = number(ui, host, tr, "border_transparency", "Transparency:", l.transparency, 0.0..=100.0, "%", on) {
            out(Edit::BorderTransparency(v));
        }
        if let Some(v) = number(ui, host, tr, "border_width", "Width:", l.width, 0.0..=20.0, " pt", true) {
            out(Edit::BorderWidth(v));
        }
        let label = ui.label(tr("Compound type:")).id;
        let names: Vec<String> = Compound::ALL.iter().map(|c| tr(c.name())).collect();
        let items: Vec<Item<Compound>> = Compound::ALL.iter().zip(&names).map(|(c, n)| Item::new(*c, Some(compound_look(*c)), n.as_str())).collect();
        let (resp, picked) = combo(ui, crate::pane_ui::id("krub_chart_compound"), 110.0, on.then_some(l.compound), &items);
        let _ = resp.labelled_by(label);
        if let Some(c) = picked {
            out(Edit::Compound(c));
        }
        ui.end_row();
        let label = ui.label(tr("Dash type:")).id;
        let names: Vec<String> = Dash::ALL.iter().map(|d| tr(d.name())).collect();
        let items: Vec<Item<Dash>> = Dash::ALL.iter().zip(&names).map(|(d, n)| Item::new(*d, Some(dash_look(*d)), n.as_str())).collect();
        let (resp, picked) = combo(ui, crate::pane_ui::id("krub_chart_dash"), 110.0, on.then_some(l.dash), &items);
        let _ = resp.labelled_by(label);
        if let Some(d) = picked {
            out(Edit::Dash(d));
        }
        ui.end_row();
    });
}

fn shadow(ui: &mut Ui, f: &Format, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, out: &mut dyn FnMut(Edit)) {
    let mut on = f.shadow.is_some();
    if ui.checkbox(&mut on, tr("Shadow")).changed() {
        out(Edit::Shadow(on));
    }
    let s = f.shadow.unwrap_or_default();
    rows("krub_chart_shadow_rows").show(ui, |ui| {
        if let Some(c) = color_row(ui, host, tr, "shadow", Some(s.color), on) {
            out(Edit::ShadowColor(c));
        }
        if let Some(v) = number(ui, host, tr, "shadow_transparency", "Transparency:", s.transparency, 0.0..=100.0, "%", on) {
            out(Edit::ShadowTransparency(v));
        }
        if let Some(v) = number(ui, host, tr, "shadow_blur", "Blur:", s.blur, 0.0..=100.0, " pt", on) {
            out(Edit::ShadowBlur(v));
        }
        if let Some(v) = number(ui, host, tr, "shadow_angle", "Angle:", s.angle, 0.0..=359.0, "°", on) {
            out(Edit::ShadowAngle(v));
        }
        if let Some(v) = number(ui, host, tr, "shadow_distance", "Distance:", s.distance, 0.0..=200.0, " pt", on) {
            out(Edit::ShadowDistance(v));
        }
    });
}

fn glow(ui: &mut Ui, f: &Format, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, out: &mut dyn FnMut(Edit)) {
    let on = f.glow.is_some();
    let g = f.glow.unwrap_or_default();
    rows("krub_chart_glow_rows").show(ui, |ui| {
        if let Some(v) = number(ui, host, tr, "glow_size", "Size:", if on { g.size } else { 0.0 }, 0.0..=150.0, " pt", true) {
            out(Edit::GlowSize(v));
        }
        if let Some(c) = color_row(ui, host, tr, "glow", Some(g.color), on) {
            out(Edit::GlowColor(c));
        }
        if let Some(v) = number(ui, host, tr, "glow_transparency", "Transparency:", g.transparency, 0.0..=100.0, "%", on) {
            out(Edit::GlowTransparency(v));
        }
    });
}

fn soft(ui: &mut Ui, f: &Format, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, out: &mut dyn FnMut(Edit)) {
    rows("krub_chart_soft_rows").show(ui, |ui| {
        if let Some(v) = number(ui, host, tr, "soft_edge", "Size:", f.soft_edge, 0.0..=100.0, " pt", true) {
            out(Edit::SoftEdge(v));
        }
    });
}

fn text_fill(ui: &mut Ui, f: &Format, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, out: &mut dyn FnMut(Edit)) {
    if let Some(k) = kinds(ui, f.text.fill.kind(), "No fill", tr) {
        out(Edit::TextFill(k));
    }
    let solid = f.text.fill.solid();
    rows("krub_chart_text_rows").show(ui, |ui| {
        if let Some(c) = color_row(ui, host, tr, "text", solid.map(|(c, _)| c), true) {
            out(Edit::TextColor(c));
        }
        let t = match f.text.fill {
            Fill::Solid { transparency, .. } => transparency,
            _ => 0.0,
        };
        if let Some(v) = number(ui, host, tr, "text_transparency", "Transparency:", t, 0.0..=100.0, "%", solid.is_some()) {
            out(Edit::TextTransparency(v));
        }
    });
}

fn outline(ui: &mut Ui, f: &Format, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, out: &mut dyn FnMut(Edit)) {
    let current = if f.text.outline.is_some() { Kind::Solid } else { Kind::None };
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (k, name) in [(Kind::None, "No line"), (Kind::Solid, "Solid")] {
            if ui.add(egui::Button::selectable(current == k, tr(name))).clicked() && current != k {
                picked = Some(k == Kind::Solid);
            }
        }
    });
    if let Some(on) = picked {
        out(Edit::TextOutline(on));
    }
    let o = f.text.outline.unwrap_or_default();
    let on = f.text.outline.is_some();
    rows("krub_chart_outline_rows").show(ui, |ui| {
        if let Some(c) = color_row(ui, host, tr, "outline", Some(o.color), true) {
            out(Edit::TextOutlineColor(c));
        }
        if let Some(v) = number(ui, host, tr, "outline_width", "Width:", o.width, 0.0..=20.0, " pt", on) {
            out(Edit::TextOutlineWidth(v));
        }
    });
}

fn font(ui: &mut Ui, f: &Format, tr: &dyn Fn(&str) -> String, host: &mut dyn Host, out: &mut dyn FnMut(Edit)) {
    let mut auto = f.text.size.is_none();
    rows("krub_chart_font_rows").show(ui, |ui| {
        if let Some(v) = number(ui, host, tr, "font_size", "Font size:", f.text.size.unwrap_or(10.0), 1.0..=400.0, " pt", true) {
            out(Edit::FontSize(Some(v)));
        }
    });
    if ui.checkbox(&mut auto, tr("Automatic")).changed() {
        out(Edit::FontSize(if auto { None } else { Some(f.text.size.unwrap_or(10.0)) }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;
    impl Host for Fake {
        fn color(&mut self, ui: &mut Ui, _: &'static str, _: Option<Color>, _: bool) -> Option<Color> {
            ui.label("colour");
            None
        }
        fn chart(&mut self, ui: &mut Ui) {
            ui.label("the app's own sections");
        }
    }

    #[test]
    fn every_sub_panel_draws_for_every_part() {
        let ctx = egui::Context::default();
        let parts: Vec<(Part, String)> =
            [Part::ChartArea, Part::PlotArea, Part::Title, Part::Gridlines, Part::Series(0)].iter().map(|p| (*p, p.name().to_string())).collect();
        let mut fs = Formats::default();
        fs.apply(Part::ChartArea, &Edit::Shadow(true));
        for tab in Tab::ALL {
            for (p, _) in &parts {
                set_tab(&ctx, tab);
                set_part(&ctx, *p);
                let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                    let edits = show(ui, &parts, &fs, &|s| s.to_string(), &mut Fake);
                    assert!(edits.is_empty(), "nothing clicked");
                });
                out.textures_delta.clear();
                assert_eq!(current_tab(&ctx), tab);
            }
        }
        assert_eq!(current_part(&ctx, &parts[..2]), Part::ChartArea, "a part the chart lacks falls back to the first");
    }
}
