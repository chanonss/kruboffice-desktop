//! The Chart pane (owner, 2026-11-19): the shared chart pane (`krub_icons::chart_pane`) for the
//! selected chart. Its Chart sub-panel holds the former Chart Design controls; Fill and Line,
//! Effects and Text format the part chosen in its list, through `chart.format`.

use egui::{Ui, vec2};
use gridcraft_engine::model::{Chart, ChartKind};
use krub_chart_format::{Color, Part};
use serde_json::json;

use crate::SheetApp;

/// The selected chart on the active sheet.
fn selected(app: &SheetApp) -> Option<Chart> {
    let id = app.selected_chart()?;
    app.session.active().and_then(|d| d.wb.active().and_then(|s| s.charts.iter().find(|c| c.id == id).cloned()))
}

/// The parts of `c` for the pane's list: the chart and plot areas, the title and legend when
/// shown, the axes and gridlines of a chart with axes, then each series.
pub(crate) fn parts(c: &Chart, tr: &dyn Fn(&str) -> String) -> Vec<(Part, String)> {
    let mut v = vec![(Part::ChartArea, tr("Chart Area")), (Part::PlotArea, tr("Plot Area"))];
    if c.title.as_deref().is_some_and(|t| !t.is_empty()) {
        v.push((Part::Title, tr("Chart Title")));
    }
    if c.legend != gridcraft_engine::model::LegendPos::None {
        v.push((Part::Legend, tr("Legend")));
    }
    let round = matches!(c.kind, ChartKind::Pie | ChartKind::Doughnut | ChartKind::Sunburst | ChartKind::Treemap | ChartKind::Funnel);
    if !round {
        v.push((Part::CategoryAxis, tr("Horizontal Axis")));
        v.push((Part::ValueAxis, tr("Vertical Axis")));
        if c.gridlines {
            v.push((Part::Gridlines, tr("Gridlines")));
        }
    }
    for (i, s) in c.series.iter().enumerate().take(krub_chart_format::MAX_SERIES) {
        // A series named by a cell reference shows its number; a literal name shows itself.
        let name = s.name.as_deref().map(str::trim).filter(|n| !n.is_empty() && !n.contains('!')).map(|n| n.trim_matches('"').to_string());
        v.push((Part::Series(i), name.unwrap_or_else(|| tr("Series {n}").replace("{n}", &(i + 1).to_string()))));
    }
    v
}

struct Host<'a> {
    app: &'a mut SheetApp,
}

impl krub_icons::chart_pane::Host for Host<'_> {
    fn color(&mut self, ui: &mut Ui, key: &'static str, current: Option<Color>, enabled: bool) -> Option<Color> {
        let lang = crate::i18n::current(ui.ctx());
        let mut picked = None;
        ui.horizontal(|ui| {
            if let Some(c) = current {
                let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::hover());
                ui.painter().rect_filled(r, 3.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
            }
            let r = ui.add_enabled(enabled, egui::Button::new(lang.tr("Color")).min_size(vec2(80.0, 22.0)));
            let name = format!("{} {}", lang.tr("Color"), key);
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, &name));
            let theme = crate::ribbon::theme_colors_for(self.app);
            egui::Popup::menu(&r).show(|ui| {
                if let Some(c) = crate::widgets::color_palette(ui, &theme, "")
                    && let Some(rgb) = krub_chart_format::parse_hex(&c)
                {
                    picked = Some(rgb);
                }
            });
        });
        picked
    }

    fn chart(&mut self, ui: &mut Ui) {
        crate::ribbon::chart_design_pane(self.app, ui);
    }
}

/// The pane's body.
pub(crate) fn show(app: &mut SheetApp, ui: &mut Ui) {
    let Some(chart) = selected(app) else { return };
    let lang = crate::i18n::current(ui.ctx());
    let tr = |s: &str| lang.tr(s).to_string();
    let parts = parts(&chart, &tr);
    let edits = krub_icons::chart_pane::show(ui, &parts, &chart.format, &tr, &mut Host { app });
    for (part, e) in edits {
        let (key, value) = e.to_pair();
        app.run_or_alert("chart.format", json!({"chart": chart.id, "part": krub_chart_format::part_id(part), "edit": key, "value": value}));
    }
}
