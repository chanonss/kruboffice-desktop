//! Drawing the chart pane's formats (owner, 2026-11-19; `krub_chart_format`): a part's shadow and
//! glow behind it, its fill with soft edges, its border (width, compound type, dash), and lines
//! (axes, gridlines) in a border's look. The series' formats go through the series' own fill and
//! line ([`with_series_formats`]), so bars, lines, markers and legend keys all follow them.

use deckcraft_color::Rgba;
use deckcraft_model::chart::Chart;
use deckcraft_model::style::{ColorRef, Fill};
use krub_chart_format::{Border, Compound, Format, Line};
use kurbo::{Affine, BezPath, Point, Rect, RoundedRect, Shape as _, Vec2};
use vello_cpu::RenderContext;

use crate::geometry::{blur, color, scale_of};

/// The sRGB colour of a format.
pub(crate) fn rgba(c: [u8; 3]) -> Rgba {
    Rgba::rgb(c[0], c[1], c[2])
}

/// A transparency in percent as an opacity 0–1.
fn opacity(t: f32) -> f64 {
    let t = if t.is_finite() { t.clamp(0.0, 100.0) } else { 0.0 };
    f64::from(1.0 - t / 100.0)
}

/// A size from a format in points, kept finite and sensible.
fn pts(v: f32, max: f64) -> f64 {
    if v.is_finite() { f64::from(v).clamp(0.0, max) } else { 0.0 }
}

fn shape(r: Rect, radius: f64) -> BezPath {
    if radius > 0.0 { RoundedRect::from_rect(r, radius).to_path(0.1) } else { r.to_path(0.1) }
}

/// The part's text colour, when the pane gave it a solid one.
pub(crate) fn text_color(f: &Format) -> Option<Rgba> {
    f.text.fill.solid().map(|(c, a)| rgba(c).with_alpha((f64::from(a).clamp(0.0, 1.0) * 255.0).round() as u8))
}

/// The part's font size in points, when the pane set one.
pub(crate) fn text_size(f: &Format) -> Option<f64> {
    f.text.size.filter(|s| s.is_finite()).map(|s| f64::from(s).clamp(1.0, 400.0))
}

/// The shadow and glow of a box `r` (corner `radius`), drawn before the box itself.
pub(crate) fn effects(ctx: &mut RenderContext, m: Affine, r: Rect, radius: f64, f: &Format) {
    let k = scale_of(m);
    let path = shape(r, radius);
    if let Some(sh) = &f.shadow {
        let a = f64::from(if sh.angle.is_finite() { sh.angle } else { 0.0 }).to_radians();
        let d = Vec2::new(a.cos(), a.sin()) * pts(sh.distance, 200.0);
        ctx.set_transform(Affine::IDENTITY);
        ctx.push_layer(None, None, Some(opacity(sh.transparency) as f32), None, Some(blur(pts(sh.blur, 100.0) / 3.0 * k)));
        ctx.set_transform(Affine::translate(d * k) * m);
        ctx.set_paint(color(rgba(sh.color), 1.0));
        ctx.fill_path(&path);
        ctx.pop_layer();
    }
    if let Some(g) = &f.glow {
        let size = pts(g.size, 150.0);
        ctx.set_transform(Affine::IDENTITY);
        // As the shapes' glow: half strength about half the size out, gone by the size.
        ctx.push_layer(None, None, Some(opacity(g.transparency) as f32), None, Some(blur(size * 0.17 * k)));
        ctx.set_transform(m);
        ctx.set_paint(color(rgba(g.color), 1.0));
        ctx.set_stroke(kurbo::Stroke::new(size * 0.95).with_join(kurbo::Join::Round));
        ctx.stroke_path(&path);
        ctx.fill_path(&path);
        ctx.pop_layer();
    }
}

/// The solid fill of a box, with its soft edges; `false` when the format's fill is not solid
/// (automatic: the caller draws its own; none: nothing).
pub(crate) fn fill(ctx: &mut RenderContext, m: Affine, r: Rect, radius: f64, f: &Format) -> bool {
    let Some((c, a)) = f.fill.solid() else { return false };
    let soft = pts(f.soft_edge, r.width().min(r.height()) / 2.0);
    ctx.set_transform(Affine::IDENTITY);
    if soft > 0.0 {
        // Soft edges: the fill shrunk by half the radius and blurred out to the edge.
        ctx.push_layer(None, None, Some(f64::from(a) as f32), None, Some(blur(soft / 3.0 * scale_of(m))));
        ctx.set_transform(m);
        ctx.set_paint(color(rgba(c), 1.0));
        ctx.fill_path(&shape(r.inset(-soft / 2.0), (radius - soft / 2.0).max(0.0)));
        ctx.pop_layer();
    } else {
        ctx.set_transform(m);
        ctx.set_paint(color(rgba(c), f64::from(a)));
        ctx.fill_path(&shape(r, radius));
    }
    true
}

/// The strokes of a compound line `w` wide: (offset inwards, width) of each.
fn strokes(compound: Compound, w: f64) -> Vec<(f64, f64)> {
    match compound {
        Compound::Single => vec![(0.0, w)],
        Compound::Double => vec![(-w / 3.0, w / 3.0), (w / 3.0, w / 3.0)],
        Compound::ThickThin => vec![(-w * 0.25, w * 0.5), (w * 0.4, w * 0.2)],
        Compound::ThinThick => vec![(-w * 0.4, w * 0.2), (w * 0.25, w * 0.5)],
        Compound::Triple => vec![(-w * 0.4, w * 0.2), (0.0, w * 0.2), (w * 0.4, w * 0.2)],
    }
}

fn pen(l: &Line, width: f64) -> kurbo::Stroke {
    let s = kurbo::Stroke::new(width);
    let pattern: Vec<f64> = l.dash.pattern().iter().map(|p| f64::from(*p) * width.max(0.5)).collect();
    if pattern.is_empty() { s } else { s.with_dashes(0.0, pattern) }
}

/// The border of a box `r`, when the format's border is solid.
pub(crate) fn border(ctx: &mut RenderContext, m: Affine, r: Rect, radius: f64, f: &Format) {
    let Border::Solid(l) = &f.border else { return };
    let w = pts(l.width, 20.0);
    if w <= 0.0 {
        return;
    }
    ctx.set_transform(m);
    ctx.set_paint(color(rgba(l.color), opacity(l.transparency)));
    for (off, sw) in strokes(l.compound, w) {
        ctx.set_stroke(pen(l, sw));
        ctx.stroke_path(&shape(r.inset(-off), (radius - off).max(0.0)));
    }
}

/// A whole box: effects, fill (when solid), border. `true` when it drew a fill.
pub(crate) fn part(ctx: &mut RenderContext, m: Affine, r: Rect, radius: f64, f: &Format) -> bool {
    effects(ctx, m, r, radius, f);
    let filled = fill(ctx, m, r, radius, f);
    border(ctx, m, r, radius, f);
    filled
}

/// A straight line from `a` to `b` in the look of a solid border (an axis line).
pub(crate) fn line(ctx: &mut RenderContext, m: Affine, a: Point, b: Point, f: &Format) {
    let Border::Solid(l) = &f.border else { return };
    let w = pts(l.width, 20.0);
    if w <= 0.0 {
        return;
    }
    let n = (b - a).normalize();
    let normal = Vec2::new(-n.y, n.x);
    ctx.set_transform(m);
    ctx.set_paint(color(rgba(l.color), opacity(l.transparency)));
    for (off, sw) in strokes(l.compound, w) {
        ctx.set_stroke(pen(l, sw));
        let mut p = BezPath::new();
        p.move_to(a + normal * off);
        p.line_to(b + normal * off);
        ctx.stroke_path(&p);
    }
}

/// The gridlines' colour: their solid border's, invisible for No line, else `default`.
pub(crate) fn grid_color(f: &Format, default: Rgba) -> Rgba {
    match &f.border {
        Border::Solid(l) => rgba(l.color).with_alpha((opacity(l.transparency) * 255.0).round() as u8),
        Border::None => default.with_alpha(0),
        Border::Auto => default,
    }
}

/// `c` with each series' pane format (`krub_chart_format`) as the series' own fill and line, so
/// every place that draws a series follows it. `None` when no series has one.
pub(crate) fn with_series_formats(c: &Chart) -> Option<Chart> {
    if c.format.series.iter().all(|f| f.shape_is_auto()) {
        return None;
    }
    let mut out = c.clone();
    for (s, f) in out.series.iter_mut().zip(&c.format.series) {
        let solid = |rgb: [u8; 3], a: f64| Fill::solid(ColorRef::rgb(rgba(rgb).with_alpha((a.clamp(0.0, 1.0) * 255.0).round() as u8)));
        match f.fill {
            krub_chart_format::Fill::Auto => {}
            krub_chart_format::Fill::None => s.fill = Some(Fill::None),
            krub_chart_format::Fill::Solid { .. } => {
                if let Some((rgb, a)) = f.fill.solid() {
                    s.fill = Some(solid(rgb, f64::from(a)));
                }
            }
        }
        match &f.border {
            Border::Auto => {}
            Border::None => s.line = Some(deckcraft_model::Line { fill: Some(Fill::None), ..Default::default() }),
            Border::Solid(l) => {
                s.line = Some(deckcraft_model::Line {
                    fill: Some(solid(l.color, opacity(l.transparency))),
                    width: Some(pts(l.width, 20.0)),
                    ..s.line.clone().unwrap_or_default()
                })
            }
        }
    }
    Some(out)
}
