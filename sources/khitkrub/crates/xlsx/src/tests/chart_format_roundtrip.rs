//! The chart pane's formats (owner, 2026-11-19; `krub_chart_format`) survive a save.

use gridcraft_model::{Anchor, Chart, ChartKind, Color, LegendPos, Series, Sheet, Workbook};
use krub_chart_format::{Dash, Edit, Kind, Part};
use std::sync::Arc;

use crate::{read_xlsx, write_xlsx};

#[test]
fn chart_pane_formats_survive_a_save() {
    for kind in [ChartKind::ColumnClustered, ChartKind::Line, ChartKind::Pie, ChartKind::Scatter, ChartKind::BarClustered] {
        let mut wb = Workbook::default();
        let mut s = Sheet::new("Data");
        let series = Series {
            name: Some("Data!$A$1".into()),
            categories: Some("Data!$A$2:$A$5".into()),
            values: "Data!$B$2:$B$5".into(),
            bubble_sizes: None,
            color: Some(Color::Rgb(0x4472C4)),
            secondary: false,
            kind: None,
            quartiles: Default::default(),
            smooth: false,
            trendlines: Vec::new(),
        };
        let mut c = Chart {
            id: 1,
            kind,
            anchor: Anchor { cell: gridcraft_core::CellRef::new(0, 3), width: 480.0, height: 300.0, ..Anchor::default() },
            title: Some("Sales".into()),
            series: vec![series.clone(), series],
            legend: LegendPos::Bottom,
            data_labels: false,
            data_table: None,
            gridlines: true,
            style: 2,
            hole_size: 75,
            first_slice_angle: 0,
            x_title: None,
            y_title: None,
            source: None,
            by_rows: false,
            pivot: None,
            format: Default::default(),
        };
        let round = kind == ChartKind::Pie;
        let edits = [
            (Part::ChartArea, Edit::FillColor([0xF7, 0xF4, 0xEC])),
            (Part::ChartArea, Edit::Shadow(true)),
            (Part::ChartArea, Edit::RoundedCorners(true)),
            (Part::ChartArea, Edit::TextColor([0x1E, 0x3A, 0x6E])),
            (Part::PlotArea, Edit::BorderColor([0, 0, 0])),
            (Part::Title, Edit::FontSize(Some(18.0))),
            (Part::Legend, Edit::Fill(Kind::None)),
            (Part::CategoryAxis, Edit::Dash(Dash::Dotted)),
            (Part::ValueAxis, Edit::TextColor([0xC0, 0, 0])),
            (Part::Gridlines, Edit::BorderColor([0xEE, 0xEE, 0xEE])),
            (Part::Series(1), Edit::FillColor([0xE9, 0xC4, 0x6A])),
            (Part::Series(1), Edit::GlowSize(3.0)),
        ];
        for (p, e) in edits {
            if round && matches!(p, Part::CategoryAxis | Part::ValueAxis | Part::Gridlines) {
                continue;
            }
            c.format.apply(p, &e);
        }
        s.charts.push(c.clone());
        wb.sheets = vec![Arc::new(s)];
        let bytes = write_xlsx(&wb).unwrap();
        let (back, _rep) = read_xlsx(&bytes).unwrap();
        assert_eq!(back.sheets[0].charts[0].format, c.format, "{kind:?}");
    }
}
