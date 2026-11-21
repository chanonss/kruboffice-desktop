//! Chart-type icons for the chart menus, the same in every KrubOffice app (owner, 2026-11-19,
//! from the approved draft): original drawings on a 24-unit grid, thin outlines with a pale gold
//! surface, each showing its chart's shape (stacked columns in two layers, 100 % stacked ones the
//! same height, a waterfall's steps, a histogram's touching bins). Tabler covers only some of
//! these, so all are drawn here in one style. Contributor-original, `MIT OR Apache-2.0`.
//!
//! [`item`] is a menu row with the icon before its label. The apps run different egui versions,
//! so this uses only what they share: the row is a selectable button whose (invisible) text is
//! the label, with the icon and the label painted over it.

use egui::{Color32, Painter, Pos2, Rect, Response, RichText, Shape, Stroke, Ui, pos2, vec2};

/// Every icon id, in menu order.
pub const IDS: [&str; 25] = [
    "column",
    "columnStacked",
    "columnStacked100",
    "bar",
    "barStacked",
    "barStacked100",
    "line",
    "lineMarkers",
    "lineStacked",
    "area",
    "areaStacked",
    "areaStacked100",
    "pie",
    "doughnut",
    "scatter",
    "scatterLines",
    "bubble",
    "radar",
    "waterfall",
    "funnel",
    "histogram",
    "treemap",
    "pareto",
    "boxWhisker",
    "sunburst",
];

/// The pale gold surface.
pub const SURFACE: Color32 = Color32::from_rgb(0xFA, 0xC7, 0x75);

/// The icon id for a chart type and subtype as the apps name them (`column` + `stacked100`,
/// `line` + `markers`, `scatter` + `lines`, `boxWhisker`…); `None` for one without an icon.
pub fn id_for(kind: &str, sub: &str) -> Option<&'static str> {
    let k = kind.to_ascii_lowercase().replace(['_', '-', ' '], "");
    let s = sub.to_ascii_lowercase();
    Some(match (k.as_str(), s.as_str()) {
        ("column" | "columnclustered" | "clusteredcolumn", "" | "clustered") => "column",
        ("column" | "columnstacked" | "stackedcolumn", "stacked") | ("columnstacked" | "stackedcolumn", "") => "columnStacked",
        ("column", "stacked100" | "percent") | ("columnstacked100" | "percentcolumn", _) => "columnStacked100",
        ("bar" | "barclustered" | "clusteredbar", "" | "clustered") => "bar",
        ("bar", "stacked") | ("barstacked" | "stackedbar", "") => "barStacked",
        ("bar", "stacked100" | "percent") | ("barstacked100" | "percentbar", _) => "barStacked100",
        ("line", "" | "standard") => "line",
        ("line", "markers") | ("linemarkers", _) => "lineMarkers",
        ("line", "stacked" | "stacked100") | ("linestacked" | "stackedline", _) => "lineStacked",
        ("area", "") => "area",
        ("area", "stacked") | ("areastacked" | "stackedarea", "") => "areaStacked",
        ("area", "stacked100" | "percent") | ("areastacked100" | "percentarea", _) => "areaStacked100",
        ("pie", _) => "pie",
        ("doughnut" | "donut", _) => "doughnut",
        ("scatter", "lines") | ("scatterlines", _) => "scatterLines",
        ("scatter", _) => "scatter",
        ("bubble", _) => "bubble",
        ("radar" | "filledradar", _) => "radar",
        ("waterfall", _) => "waterfall",
        ("funnel", _) => "funnel",
        ("histogram", _) => "histogram",
        ("treemap", _) => "treemap",
        ("pareto", _) => "pareto",
        ("boxwhisker" | "boxandwhisker", _) => "boxWhisker",
        ("sunburst", _) => "sunburst",
        _ => return None,
    })
}

struct Pen<'a> {
    p: &'a Painter,
    r: Rect,
    ink: Color32,
    surface: Color32,
}

impl Pen<'_> {
    fn pt(&self, x: f32, y: f32) -> Pos2 {
        let k = self.r.width().min(self.r.height()) / 24.0;
        pos2(self.r.center().x + (x - 12.0) * k, self.r.center().y + (y - 12.0) * k)
    }
    fn stroke(&self) -> Stroke {
        Stroke::new((self.r.width() / 24.0 * 1.4).max(0.9), self.ink)
    }
    fn line(&self, pts: &[(f32, f32)]) {
        self.p.add(Shape::line(pts.iter().map(|&(x, y)| self.pt(x, y)).collect(), self.stroke()));
    }
    /// A rectangle: filled with the surface when `fill`, outlined either way.
    fn rect(&self, x: f32, y: f32, w: f32, h: f32, fill: bool) {
        let r = Rect::from_min_max(self.pt(x, y), self.pt(x + w, y + h));
        if fill {
            self.p.rect_filled(r, 0.0, self.surface);
        }
        self.p.rect_stroke(r, 0.0, self.stroke(), egui::StrokeKind::Middle);
    }
    fn poly(&self, pts: &[(f32, f32)], fill: bool) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        let fill = if fill { self.surface } else { Color32::TRANSPARENT };
        self.p.add(Shape::Path(egui::epaint::PathShape { points: pts, closed: true, fill, stroke: self.stroke().into() }));
    }
    fn circle(&self, x: f32, y: f32, r: f32, fill: bool) {
        let k = self.r.width().min(self.r.height()) / 24.0;
        let fill = if fill { self.surface } else { Color32::TRANSPARENT };
        self.p.circle(self.pt(x, y), r * k, fill, self.stroke());
    }
    /// Points along a circle about (12, 12) of radius `r` from angle `a0` to `a1` (degrees,
    /// 0 at 12 o'clock, clockwise).
    fn arc(r: f32, a0: f32, a1: f32) -> Vec<(f32, f32)> {
        let n = 16;
        (0..=n)
            .map(|i| {
                let a = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians();
                (12.0 + r * a.sin(), 12.0 - r * a.cos())
            })
            .collect()
    }
    /// A ring segment between radii `r0` and `r1` (a wedge when `r0` is 0), filled.
    fn segment(&self, r0: f32, r1: f32, a0: f32, a1: f32) {
        let mut pts = Self::arc(r1, a0, a1);
        if r0 <= 0.0 {
            pts.push((12.0, 12.0));
        } else {
            let mut inner = Self::arc(r0, a0, a1);
            inner.reverse();
            pts.extend(inner);
        }
        self.poly(&pts, true);
    }
    fn base(&self) {
        self.line(&[(3.0, 21.0), (21.0, 21.0)]);
    }
    fn left(&self) {
        self.line(&[(3.0, 3.0), (3.0, 21.0)]);
    }
}

/// Paints icon `id` in `r` with `ink` lines; `false` when the id is unknown.
pub fn paint(p: &Painter, r: Rect, id: &str, ink: Color32) -> bool {
    let d = Pen { p, r, ink, surface: SURFACE };
    match id {
        "column" => {
            d.base();
            d.rect(4.0, 11.0, 3.0, 10.0, true);
            d.rect(7.0, 7.0, 3.0, 14.0, false);
            d.rect(13.0, 13.0, 3.0, 8.0, true);
            d.rect(16.0, 5.0, 3.0, 16.0, false);
        }
        "columnStacked" | "columnStacked100" => {
            d.base();
            let full = id == "columnStacked100";
            for (x, low, high) in [(4.5, 14.0, 9.0), (10.0, 12.0, 5.0), (15.5, 15.0, 11.0)] {
                let (low, high) = if full { (low - 2.0, 4.0) } else { (low, high) };
                d.rect(x, low, 4.0, 21.0 - low, true);
                d.rect(x, high, 4.0, low - high, false);
            }
        }
        "bar" => {
            d.left();
            d.rect(3.0, 4.0, 10.0, 3.0, true);
            d.rect(3.0, 7.0, 14.0, 3.0, false);
            d.rect(3.0, 13.0, 8.0, 3.0, true);
            d.rect(3.0, 16.0, 16.0, 3.0, false);
        }
        "barStacked" | "barStacked100" => {
            d.left();
            let full = id == "barStacked100";
            for (y, a, b) in [(4.5, 7.0, 5.0), (10.0, 9.0, 7.0), (15.5, 6.0, 4.0)] {
                let (a, b) = if full { (a + 2.0, 17.0 - (a + 2.0)) } else { (a, b) };
                d.rect(3.0, y, a, 4.0, true);
                d.rect(3.0 + a, y, b, 4.0, false);
            }
        }
        "line" | "lineMarkers" => {
            d.base();
            let pts = [(4.0, 16.0), (9.0, 11.0), (13.0, 14.0), (20.0, 6.0)];
            d.line(&pts);
            if id == "lineMarkers" {
                for (x, y) in pts {
                    d.circle(x, y, 1.8, true);
                }
            }
        }
        "lineStacked" => {
            d.base();
            d.line(&[(4.0, 17.0), (9.0, 15.0), (14.0, 16.0), (20.0, 12.0)]);
            d.line(&[(4.0, 11.0), (9.0, 8.0), (14.0, 10.0), (20.0, 4.0)]);
        }
        "area" => {
            d.base();
            d.poly(&[(4.0, 21.0), (4.0, 15.0), (9.0, 10.0), (13.0, 13.0), (20.0, 6.0), (20.0, 21.0)], true);
        }
        "areaStacked" => {
            d.base();
            d.poly(&[(4.0, 21.0), (4.0, 16.0), (9.0, 14.0), (14.0, 15.0), (20.0, 11.0), (20.0, 21.0)], true);
            d.line(&[(4.0, 16.0), (4.0, 11.0), (9.0, 8.0), (14.0, 10.0), (20.0, 4.0), (20.0, 11.0)]);
        }
        "areaStacked100" => {
            d.base();
            d.poly(&[(4.0, 21.0), (4.0, 14.0), (9.0, 11.0), (14.0, 13.0), (20.0, 9.0), (20.0, 21.0)], true);
            d.line(&[(4.0, 14.0), (4.0, 4.0), (20.0, 4.0), (20.0, 9.0)]);
        }
        "pie" => {
            d.circle(12.0, 12.0, 8.5, false);
            d.segment(0.0, 8.5, 0.0, 110.0);
        }
        "doughnut" => {
            d.circle(12.0, 12.0, 8.5, false);
            d.circle(12.0, 12.0, 4.0, false);
            d.segment(4.0, 8.5, 0.0, 110.0);
        }
        "sunburst" => {
            for r in [3.0, 6.0, 9.0] {
                d.circle(12.0, 12.0, r, false);
            }
            d.segment(6.0, 9.0, 0.0, 90.0);
            d.line(&[(12.0, 3.0), (12.0, 9.0)]);
            d.line(&[(21.0, 12.0), (15.0, 12.0)]);
            d.line(&[(12.0, 21.0), (12.0, 15.0)]);
        }
        "scatter" | "scatterLines" | "bubble" => {
            d.base();
            d.left();
            let pts = [(7.0, 16.0), (10.0, 11.0), (14.0, 13.0), (15.0, 7.0), (19.0, 9.0)];
            if id == "scatterLines" {
                d.line(&pts);
            }
            for (i, (x, y)) in pts.into_iter().enumerate() {
                let r = if id == "bubble" { [2.4, 1.4, 3.0, 1.8, 2.2].get(i).copied().unwrap_or(2.0) } else { 1.6 };
                d.circle(x, y, r, true);
            }
        }
        "radar" => {
            d.poly(&[(12.0, 3.0), (20.0, 9.0), (17.0, 19.0), (7.0, 19.0), (4.0, 9.0)], false);
            d.poly(&[(12.0, 7.0), (17.0, 10.0), (15.0, 16.0), (9.5, 16.0), (7.5, 10.0)], true);
        }
        "waterfall" => {
            d.base();
            d.rect(4.0, 13.0, 3.0, 8.0, true);
            d.rect(10.5, 9.0, 3.0, 4.0, true);
            d.rect(17.0, 5.0, 3.0, 4.0, true);
            d.line(&[(7.0, 13.0), (10.5, 13.0)]);
            d.line(&[(13.5, 9.0), (17.0, 9.0)]);
        }
        "histogram" => {
            d.base();
            for (x, y) in [(4.0, 9.0), (8.0, 5.0), (12.0, 11.0), (16.0, 15.0)] {
                d.rect(x, y, 4.0, 21.0 - y, true);
            }
        }
        "pareto" => {
            d.base();
            for (x, y) in [(4.0, 7.0), (9.0, 12.0), (14.0, 16.0)] {
                d.rect(x, y, 4.0, 21.0 - y, true);
            }
            d.line(&[(6.0, 12.0), (11.0, 8.0), (16.0, 5.0), (20.0, 3.5)]);
        }
        "treemap" => {
            d.rect(3.0, 4.0, 10.0, 16.0, false);
            d.rect(13.0, 4.0, 8.0, 9.0, true);
            d.rect(13.0, 13.0, 8.0, 7.0, false);
            d.line(&[(3.0, 12.0), (13.0, 12.0)]);
        }
        "funnel" => {
            d.rect(3.0, 4.0, 18.0, 3.5, true);
            d.rect(5.5, 9.5, 13.0, 3.5, true);
            d.rect(8.0, 15.0, 8.0, 3.5, true);
        }
        "boxWhisker" => {
            d.base();
            for (x, top, b0, b1, bottom) in [(8.0, 4.0, 8.0, 15.0, 19.0), (16.0, 6.0, 9.0, 14.0, 17.0)] {
                d.line(&[(x, top), (x, b0)]);
                d.line(&[(x, b1), (x, bottom)]);
                d.line(&[(x - 2.0, top), (x + 2.0, top)]);
                d.line(&[(x - 2.0, bottom), (x + 2.0, bottom)]);
                d.rect(x - 2.5, b0, 5.0, b1 - b0, true);
                let mid = (b0 + b1) / 2.0;
                d.line(&[(x - 2.5, mid), (x + 2.5, mid)]);
            }
        }
        _ => return false,
    }
    true
}

/// A menu row: chart icon `id` (none when unknown) before `label`, at least `width` wide;
/// `selected` marks the current choice. The label names it for assistive technology.
pub fn item(ui: &mut Ui, id: Option<&str>, label: &str, selected: bool, width: f32) -> Response {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, Color32::PLACEHOLDER);
    let w = (galley.size().x + 40.0).max(width);
    let resp = ui.add_sized(vec2(w, 24.0), egui::Button::selectable(selected, RichText::new(label).color(Color32::TRANSPARENT)));
    let visuals = ui.style().interact_selectable(&resp, selected);
    let ink = if ui.is_enabled() { visuals.text_color() } else { visuals.text_color().gamma_multiply(0.4) };
    let r = resp.rect;
    if let Some(id) = id {
        paint(ui.painter(), Rect::from_center_size(pos2(r.left() + 16.0, r.center().y), vec2(18.0, 18.0)), id, ink);
    }
    ui.painter().galley(pos2(r.left() + 32.0, r.center().y - galley.size().y / 2.0), galley, ink);
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_draws_and_names_map_to_them() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            for id in IDS {
                assert!(paint(ui.painter(), Rect::from_min_size(pos2(0.0, 0.0), vec2(18.0, 18.0)), id, Color32::BLACK), "{id}");
            }
            assert!(!paint(ui.painter(), Rect::NOTHING, "nope", Color32::BLACK));
        });
        out.textures_delta.clear();
        assert_eq!(id_for("column", "stacked100"), Some("columnStacked100"));
        assert_eq!(id_for("line", "markers"), Some("lineMarkers"));
        assert_eq!(id_for("scatter", "lines"), Some("scatterLines"));
        assert_eq!(id_for("boxWhisker", ""), Some("boxWhisker"));
        assert_eq!(id_for("ColumnStacked", ""), Some("columnStacked"));
        assert_eq!(id_for("stock", ""), None);
        for id in IDS {
            assert!(IDS.iter().filter(|x| **x == id).count() == 1);
        }
    }
}
