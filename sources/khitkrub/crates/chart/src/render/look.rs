//! The chart pane's formats (owner, 2026-11-19; `krub_chart_format`) as chart primitives: a part's
//! shadow and glow (rings fading out), its fill with soft edges (rings fading in), its border
//! (compound lines as several strokes, dashes), and the colours of what each part drew.

use krub_chart_format::{Border, Compound, Format};

use super::{HAlign, Prim, Rgba};

const RINGS: usize = 5;

fn pts(v: f32, max: f32) -> f32 {
    if v.is_finite() { v.clamp(0.0, max) } else { 0.0 }
}

fn alpha(t: f32) -> f32 {
    if t.is_finite() { 1.0 - t.clamp(0.0, 100.0) / 100.0 } else { 1.0 }
}

/// A format colour at `opacity` (0–1).
pub(crate) fn rgba(c: [u8; 3], opacity: f32) -> Rgba {
    [c[0], c[1], c[2], (opacity.clamp(0.0, 1.0) * 255.0).round() as u8]
}

/// The part's text colour, when the pane gave it a solid one.
pub(crate) fn text_color(f: &Format) -> Option<Rgba> {
    f.text.fill.solid().map(|(c, a)| rgba(c, a))
}

/// The part's font size in points, when the pane set one.
pub(crate) fn text_size(f: &Format) -> Option<f32> {
    f.text.size.filter(|s| s.is_finite()).map(|s| s.clamp(1.0, 400.0))
}

fn rect(out: &mut Vec<Prim>, r: [f32; 4], radius: f32, fill: Option<Rgba>, stroke: Option<(Rgba, f32)>) {
    if r[2] > 0.0 && r[3] > 0.0 {
        out.push(Prim::Rect { x: r[0], y: r[1], w: r[2], h: r[3], fill, stroke, radius: radius.max(0.0) });
    }
}

fn grow(r: [f32; 4], d: f32) -> [f32; 4] {
    [r[0] - d, r[1] - d, (r[2] + 2.0 * d).max(0.0), (r[3] + 2.0 * d).max(0.0)]
}

/// A part's box `r`: shadow and glow, its solid fill (with soft edges), then its border. `true`
/// when it drew a fill.
pub(crate) fn part(out: &mut Vec<Prim>, r: [f32; 4], radius: f32, f: &Format) -> bool {
    if let Some(s) = &f.shadow {
        let a = (if s.angle.is_finite() { s.angle } else { 0.0 }).to_radians();
        let d = pts(s.distance, 200.0);
        let at = [r[0] + a.cos() * d, r[1] + a.sin() * d, r[2], r[3]];
        let blur = pts(s.blur, 100.0);
        // Layers of the same faint colour, each smaller: dark in the middle, fading out.
        for i in 0..RINGS {
            let t = (i + 1) as f32 / RINGS as f32;
            rect(
                out,
                grow(at, blur * (1.0 - t) / 2.0),
                radius + blur * (1.0 - t) / 2.0,
                Some(rgba(s.color, alpha(s.transparency) / RINGS as f32)),
                None,
            );
        }
    }
    if let Some(g) = &f.glow {
        let size = pts(g.size, 150.0);
        for i in 0..RINGS {
            let t = (i + 1) as f32 / RINGS as f32;
            let d = size * (1.0 - t * 0.8);
            rect(out, grow(r, d), radius + d, Some(rgba(g.color, alpha(g.transparency) / RINGS as f32 * 1.5)), None);
        }
    }
    let filled = match f.fill.solid() {
        Some((c, a)) => {
            let soft = pts(f.soft_edge, r[2].min(r[3]) / 2.0);
            if soft > 0.0 {
                for i in 0..RINGS {
                    let t = i as f32 / RINGS as f32;
                    rect(out, grow(r, -soft * t), (radius - soft * t).max(0.0), Some(rgba(c, a / RINGS as f32 * 1.6)), None);
                }
            } else {
                rect(out, r, radius, Some(rgba(c, a)), None);
            }
            true
        }
        None => false,
    };
    border(out, r, radius, f);
    filled
}

fn strokes(compound: Compound, w: f32) -> Vec<(f32, f32)> {
    match compound {
        Compound::Single => vec![(0.0, w)],
        Compound::Double => vec![(-w / 3.0, w / 3.0), (w / 3.0, w / 3.0)],
        Compound::ThickThin => vec![(-w * 0.25, w * 0.5), (w * 0.4, w * 0.2)],
        Compound::ThinThick => vec![(-w * 0.4, w * 0.2), (w * 0.25, w * 0.5)],
        Compound::Triple => vec![(-w * 0.4, w * 0.2), (0.0, w * 0.2), (w * 0.4, w * 0.2)],
    }
}

/// The border of box `r`, when solid.
pub(crate) fn border(out: &mut Vec<Prim>, r: [f32; 4], radius: f32, f: &Format) {
    let Border::Solid(l) = &f.border else { return };
    let w = pts(l.width, 20.0);
    if w <= 0.0 {
        return;
    }
    let c = rgba(l.color, alpha(l.transparency));
    for (off, sw) in strokes(l.compound, w) {
        let b = grow(r, -off);
        if l.dash.pattern().is_empty() {
            rect(out, b, (radius - off).max(0.0), None, Some((c, sw)));
        } else {
            let (x0, y0, x1, y1) = (b[0], b[1], b[0] + b[2], b[1] + b[3]);
            out.push(Prim::Line { pts: vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]], color: c, width: sw, dash: true });
        }
    }
}

/// The box around `out[from..]` grown by `pad`.
pub(crate) fn bounds(out: &[Prim], from: usize, pad: f32, measure: &dyn Fn(&str, f32) -> f32) -> Option<[f32; 4]> {
    let mut b: Option<[f32; 4]> = None;
    let mut add = |r: [f32; 4]| {
        b = Some(match b {
            None => r,
            Some(o) => {
                let (x0, y0) = (o[0].min(r[0]), o[1].min(r[1]));
                let (x1, y1) = ((o[0] + o[2]).max(r[0] + r[2]), (o[1] + o[3]).max(r[1] + r[3]));
                [x0, y0, x1 - x0, y1 - y0]
            }
        })
    };
    for p in out.get(from..).unwrap_or_default() {
        match p {
            Prim::Rect { x, y, w, h, .. } => add([*x, *y, *w, *h]),
            Prim::Text { x, y, text, size, align, valign, .. } => {
                let tw = measure(text, *size);
                let x0 = match align {
                    HAlign::Left => *x,
                    HAlign::Center => x - tw / 2.0,
                    HAlign::Right => x - tw,
                };
                let y0 = match valign {
                    super::VAlign::Top => *y,
                    super::VAlign::Middle => y - size * 0.65,
                    super::VAlign::Bottom => y - size * 1.3,
                };
                add([x0, y0, tw, size * 1.3]);
            }
            _ => {}
        }
    }
    b.map(|r| grow(r, pad))
}

/// Text in `was` within `out[from..to]` recoloured to `to_color`, where `pick` agrees.
pub(crate) fn recolor(out: &mut [Prim], range: std::ops::Range<usize>, was: Rgba, to_color: Rgba, pick: impl Fn(&Prim) -> bool) {
    let end = range.end.min(out.len());
    for p in out.get_mut(range.start.min(end)..end).unwrap_or_default() {
        if !pick(p) {
            continue;
        }
        if let Prim::Text { color, .. } = p
            && *color == was
        {
            *color = to_color;
        }
    }
}

/// Lines in `was` within `out[range]` that `pick` chooses, drawn in the look of `f`'s border:
/// its colour, width and dash; hidden for No line; left as they are when automatic.
pub(crate) fn restyle_lines(out: &mut Vec<Prim>, range: std::ops::Range<usize>, was: Rgba, f: &Format, pick: impl Fn(&[[f32; 2]]) -> bool) {
    let end = range.end.min(out.len());
    let start = range.start.min(end);
    let mut keep = Vec::with_capacity(out.len());
    for (i, p) in std::mem::take(out).into_iter().enumerate() {
        let hit = (start..end).contains(&i) && matches!(&p, Prim::Line { color, pts, .. } if *color == was && pick(pts));
        if !hit {
            keep.push(p);
            continue;
        }
        match (&f.border, p) {
            (Border::None, _) => {}
            (Border::Solid(l), Prim::Line { pts, .. }) => {
                keep.push(Prim::Line { pts, color: rgba(l.color, alpha(l.transparency)), width: pts_w(l.width), dash: !l.dash.pattern().is_empty() })
            }
            (_, p) => keep.push(p),
        }
    }
    *out = keep;
}

fn pts_w(w: f32) -> f32 {
    pts(w, 20.0).max(0.25)
}

/// A line run is horizontal (its ends at the same height).
pub(crate) fn horizontal(pts: &[[f32; 2]]) -> bool {
    match (pts.first(), pts.last()) {
        (Some(a), Some(b)) => (a[1] - b[1]).abs() < 0.01 && (a[0] - b[0]).abs() > 0.01,
        _ => false,
    }
}
