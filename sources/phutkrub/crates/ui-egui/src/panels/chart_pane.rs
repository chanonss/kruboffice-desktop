//! The Chart pane (owner, 2026-11-19): the shared chart pane (`krub_icons::chart_pane`) for the
//! selected chart. Its Chart sub-panel holds the former Chart Design groups; Fill and Line,
//! Effects and Text format the part chosen in its list, through `chart.format`.

use deckcraft_model::ShapeKind;
use deckcraft_model::chart::{Chart, ChartType};
use egui::{Ui, vec2};
use krub_chart_format::{Color, Part};
use krub_icons::chart_pane::Host;
use krub_icons::shape_pane::Number;
use serde_json::json;

use super::run;
use crate::SlideApp;
use crate::widgets;

/// The selected chart and its shape id.
fn selected(app: &SlideApp) -> Option<(deckcraft_model::ShapeId, Chart)> {
    let st = app.session.active()?;
    st.selected_shapes().into_iter().find_map(|s| match &s.kind {
        ShapeKind::Chart(c) => Some((s.id, (**c).clone())),
        _ => None,
    })
}

/// The parts the chart has, named for the list: the chart area, the plot area, the title and the
/// legend when shown, the axes and gridlines of a chart that has axes, then each series by name.
pub(crate) fn parts(c: &Chart) -> Vec<(Part, String)> {
    let mut v = vec![(Part::ChartArea, tl!("Chart Area").to_string()), (Part::PlotArea, tl!("Plot Area").to_string())];
    if c.title.is_some() {
        v.push((Part::Title, tl!("Chart Title").to_string()));
    }
    if c.legend.is_some() {
        v.push((Part::Legend, tl!("Legend").to_string()));
    }
    let round = matches!(c.kind, ChartType::Pie | ChartType::Doughnut | ChartType::Funnel | ChartType::Treemap | ChartType::Sunburst);
    if !round {
        v.push((Part::CategoryAxis, tl!("Horizontal Axis").to_string()));
        v.push((Part::ValueAxis, tl!("Vertical Axis").to_string()));
        if c.gridlines {
            v.push((Part::Gridlines, tl!("Gridlines").to_string()));
        }
    }
    for (i, s) in c.series.iter().enumerate().take(krub_chart_format::MAX_SERIES) {
        let name = if s.name.trim().is_empty() { crate::i18n::fmt(tl!("Series {n}"), &[("n", &(i + 1).to_string())]) } else { s.name.clone() };
        v.push((Part::Series(i), name));
    }
    v
}

struct ChartHost<'a> {
    app: &'a mut SlideApp,
}

impl Host for ChartHost<'_> {
    fn color(&mut self, ui: &mut Ui, key: &'static str, current: Option<Color>, enabled: bool) -> Option<Color> {
        let sc = super::scheme(self.app);
        let name = match key {
            "fill" => "Fill color",
            "border" => "Line color",
            "shadow" => "Shadow color",
            "glow" => "Glow color",
            "text" => "Text color",
            _ => "Outline color",
        };
        let mut picked = None;
        ui.horizontal(|ui| {
            if let Some(c) = current {
                let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::hover());
                ui.painter().rect_filled(r, 3.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
            }
            let r = ui.add_enabled_ui(enabled, |ui| widgets::drop_button(ui, tl!("Color"), vec2(80.0, 22.0), enabled)).inner;
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, tl!(name)));
            widgets::color_popup(&r).show(|ui| {
                if let Some(Some(c)) = widgets::color_grid(ui, &sc, None) {
                    let rgba = c.resolve(&sc, None);
                    picked = Some([rgba.r, rgba.g, rgba.b]);
                }
            });
        });
        picked
    }

    fn number(&mut self, ui: &mut Ui, n: Number) -> Option<f32> {
        let (lo, hi) = (*n.range.start(), *n.range.end());
        let id = format!("chart_pane_{}", n.key);
        ui.add_enabled_ui(n.enabled, |ui| widgets::spinner(ui, &id, n.name, f64::from(n.value), f64::from(n.speed), n.suffix, 80.0))
            .inner
            .map(|v| (v as f32).clamp(lo, hi))
    }

    fn chart(&mut self, ui: &mut Ui) {
        crate::ribbon::chart_design_pane(self.app, ui);
    }
}

/// The pane's body.
pub(crate) fn chart_pane(app: &mut SlideApp, ui: &mut Ui) {
    let Some((id, chart)) = selected(app) else { return };
    let parts = parts(&chart);
    let edits = krub_icons::chart_pane::show(ui, &parts, &chart.format, &|s| tl!(s).to_string(), &mut ChartHost { app });
    for (part, e) in edits {
        let (key, value) = e.to_pair();
        run(app, "chart.format", json!({"id": id, "part": krub_chart_format::part_id(part), "edit": key, "value": value}));
    }
}
