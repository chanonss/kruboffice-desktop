//! Alignment guides: while a picture, shape or text box is dragged, its left, centre or right
//! edge (and top, middle or bottom) snaps to the page's edges and centre, the margins and the
//! edges and centres of the other floating objects on its page, and a thin line across the page
//! shows each place it snapped to. WordCraft's own design (Word for Mac shows no guides).
//!
//! The maths here is pure (points in, points out) so it's tested without egui; `objects.rs`
//! gathers the candidates and draws the lines.

use wordcraft_geom::Rect;

/// How near (screen points) an edge or centre must come to a guide to snap to it.
pub const SNAP_SCREEN: f32 = 6.0;

/// Which way a guide runs: `X` is a vertical line at `at` (lefts, centres and rights snap to it),
/// `Y` a horizontal one (tops, middles and bottoms).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Axis {
    X,
    Y,
}

/// A place to snap to on the page, points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Guide {
    pub axis: Axis,
    pub at: f32,
}

impl Guide {
    pub fn x(at: f32) -> Guide {
        Guide { axis: Axis::X, at }
    }
    pub fn y(at: f32) -> Guide {
        Guide { axis: Axis::Y, at }
    }
}

/// What snapping does to a dragged rect: move it by (`dx`, `dy`) points, and show `lines`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snap {
    pub dx: f32,
    pub dy: f32,
    pub lines: Vec<Guide>,
}

/// Snap `moving` (the dragged object's bounds, points) to the nearest of `candidates` within
/// `threshold` points, x and y independently: its left, centre or right to an `X` guide, its top,
/// middle or bottom to a `Y` guide. Nearest wins; on a tie, the first candidate. The lines are
/// every guide an edge or centre sits on once moved.
pub fn snap(moving: Rect, candidates: &[Guide], threshold: f32) -> Snap {
    if !threshold.is_finite() || threshold < 0.0 {
        return Snap::default();
    }
    let xs = [moving.x, moving.x + moving.w / 2.0, moving.x + moving.w];
    let ys = [moving.y, moving.y + moving.h / 2.0, moving.y + moving.h];
    let mut out = Snap::default();
    for (axis, points) in [(Axis::X, xs), (Axis::Y, ys)] {
        let Some(d) = nearest(&points, candidates, axis, threshold) else { continue };
        match axis {
            Axis::X => out.dx = d,
            Axis::Y => out.dy = d,
        }
        for g in candidates.iter().filter(|g| g.axis == axis && g.at.is_finite()) {
            if points.iter().any(|p| (p + d - g.at).abs() <= LANDED) && !out.lines.contains(g) {
                out.lines.push(*g);
            }
        }
    }
    out
}

/// Snap the edges a resize drags (`hx`, `hy`: -1 left/top, 1 right/bottom, 0 that axis isn't
/// dragged) of `r` to the nearest of `candidates` within `threshold` points, so an object
/// stretched to the margin lands exactly on it. With `ratio` (a picture's corner), only the edge
/// nearer its guide snaps and the other follows to keep the proportions, the opposite corner
/// staying put. A snap that would make it smaller than `min` either way is left out. Returns the
/// snapped rect and the guides its dragged edges sit on.
pub fn snap_resize(r: Rect, (hx, hy): (i8, i8), candidates: &[Guide], threshold: f32, ratio: bool, min: f32) -> (Rect, Vec<Guide>) {
    if !threshold.is_finite() || threshold < 0.0 || !(r.w > 0.0 && r.h > 0.0) {
        return (r, Vec::new());
    }
    let edge = |h: i8, lo: f32, size: f32| match h {
        -1 => Some(lo),
        1 => Some(lo + size),
        _ => None,
    };
    let (ex, ey) = (edge(hx, r.x, r.w), edge(hy, r.y, r.h));
    let mut dx = ex.and_then(|e| nearest(&[e], candidates, Axis::X, threshold));
    let mut dy = ey.and_then(|e| nearest(&[e], candidates, Axis::Y, threshold));
    if ratio && ex.is_some() && ey.is_some() {
        match (dx, dy) {
            (Some(x), Some(y)) if y.abs() < x.abs() => dx = None,
            (Some(_), Some(_)) => dy = None,
            _ => {}
        }
    }
    let (mut x0, mut y0, mut x1, mut y1) = (r.x, r.y, r.x + r.w, r.y + r.h);
    let apply = |h: i8, d: f32, lo: &mut f32, hi: &mut f32| if h < 0 { *lo += d } else { *hi += d };
    if let Some(d) = dx {
        apply(hx, d, &mut x0, &mut x1);
    }
    if let Some(d) = dy {
        apply(hy, d, &mut y0, &mut y1);
    }
    if ratio && ex.is_some() && ey.is_some() {
        let aspect = r.w / r.h;
        if dx.is_some() {
            let h = (x1 - x0) / aspect;
            if hy < 0 { y0 = y1 - h } else { y1 = y0 + h }
        } else if dy.is_some() {
            let w = (y1 - y0) * aspect;
            if hx < 0 { x0 = x1 - w } else { x1 = x0 + w }
        }
    }
    let out = Rect::new(x0, y0, x1 - x0, y1 - y0);
    if dx.is_none() && dy.is_none() || !(out.w >= min && out.h >= min) || !out.w.is_finite() || !out.h.is_finite() {
        return (r, Vec::new());
    }
    let mut lines = Vec::new();
    for (axis, at) in [(Axis::X, edge(hx, out.x, out.w)), (Axis::Y, edge(hy, out.y, out.h))] {
        let Some(at) = at else { continue };
        for g in candidates.iter().filter(|g| g.axis == axis && (g.at - at).abs() <= LANDED) {
            if !lines.contains(g) {
                lines.push(*g);
            }
        }
    }
    (out, lines)
}

/// How near (points) an edge must be to a guide after snapping to count as on it.
const LANDED: f32 = 0.01;

/// The shortest move (within `threshold`) that puts one of `points` on an `axis` guide.
fn nearest(points: &[f32], candidates: &[Guide], axis: Axis, threshold: f32) -> Option<f32> {
    let mut best: Option<f32> = None;
    for g in candidates.iter().filter(|g| g.axis == axis && g.at.is_finite()) {
        for p in points.iter().filter(|p| p.is_finite()) {
            let d = g.at - p;
            if d.abs() <= threshold && best.is_none_or(|b| d.abs() < b.abs()) {
                best = Some(d);
            }
        }
    }
    best
}

/// The guides of a page `w` × `h` points with text area `body`, and of the other floating objects
/// on it (their bounds): edges and centres.
pub fn candidates(w: f32, h: f32, body: Rect, others: impl IntoIterator<Item = Rect>) -> Vec<Guide> {
    let mut g = Vec::new();
    let mut add = |r: Rect| {
        for x in [r.x, r.x + r.w / 2.0, r.x + r.w] {
            g.push(Guide::x(x));
        }
        for y in [r.y, r.y + r.h / 2.0, r.y + r.h] {
            g.push(Guide::y(y));
        }
    };
    add(Rect::new(0.0, 0.0, w, h));
    add(body);
    for r in others {
        add(r);
    }
    g.retain(|g| g.at.is_finite());
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: Rect = Rect { x: 100.0, y: 200.0, w: 50.0, h: 20.0 };

    #[test]
    fn a_resized_edge_snaps_to_the_margin() {
        // Right edge (150) dragged to 3 pt short of the margin at 153.
        let (r, lines) = snap_resize(R, (1, 0), &[Guide::x(153.0), Guide::x(98.0)], 6.0, false, 1.0);
        assert_eq!(r, Rect::new(100.0, 200.0, 53.0, 20.0), "only the dragged edge moves");
        assert_eq!(lines, vec![Guide::x(153.0)]);
        // Left edge to 98: the right edge stays.
        let (r, _) = snap_resize(R, (-1, 0), &[Guide::x(98.0)], 6.0, false, 1.0);
        assert_eq!((r.x, r.x + r.w), (98.0, 150.0));
        // Nothing near: unchanged, no lines.
        assert_eq!(snap_resize(R, (1, 1), &[Guide::x(170.0)], 6.0, false, 1.0), (R, vec![]));
        // Too small after the snap: left out.
        assert_eq!(snap_resize(R, (-1, 0), &[Guide::x(146.0)], 6.0, false, 10.0).0, R);
    }

    #[test]
    fn a_picture_corner_snaps_one_edge_and_keeps_its_proportions() {
        // 50 × 20 (2.5 : 1) from the bottom-right corner: the right edge is 2 from 152, the bottom
        // 4 from 224; the right snaps and the bottom follows.
        let (r, lines) = snap_resize(R, (1, 1), &[Guide::x(152.0), Guide::y(224.0)], 6.0, true, 1.0);
        assert_eq!((r.x, r.y, r.w), (100.0, 200.0, 52.0));
        assert!((r.h - 20.8).abs() < 1e-4);
        assert_eq!(lines, vec![Guide::x(152.0)]);
        // From the top-left: the bottom-right corner stays put.
        let (r, _) = snap_resize(R, (-1, -1), &[Guide::x(95.0)], 6.0, true, 1.0);
        assert_eq!((r.x + r.w, r.y + r.h), (150.0, 220.0));
        assert!((r.w / r.h - 2.5).abs() < 1e-4);
    }

    #[test]
    fn edges_snap_to_guides() {
        // Left edge 3 pt from x = 97: moves left onto it.
        let s = snap(R, &[Guide::x(97.0)], 6.0);
        assert_eq!((s.dx, s.dy), (-3.0, 0.0));
        assert_eq!(s.lines, vec![Guide::x(97.0)]);
        // Right edge (150) to 154.
        let s = snap(R, &[Guide::x(154.0)], 6.0);
        assert_eq!(s.dx, 4.0);
        // Bottom (220) to 218.
        let s = snap(R, &[Guide::y(218.0)], 6.0);
        assert_eq!((s.dx, s.dy), (0.0, -2.0));
        assert_eq!(s.lines, vec![Guide::y(218.0)]);
    }

    #[test]
    fn centres_snap_to_guides() {
        // Centre 125, middle 210.
        let s = snap(R, &[Guide::x(127.0), Guide::y(207.5)], 6.0);
        assert_eq!((s.dx, s.dy), (2.0, -2.5));
        assert_eq!(s.lines, vec![Guide::x(127.0), Guide::y(207.5)]);
    }

    #[test]
    fn the_nearest_guide_wins() {
        // Left (100) is 5 from 95; right (150) is 1 from 151: the right edge snaps.
        let s = snap(R, &[Guide::x(95.0), Guide::x(151.0)], 6.0);
        assert_eq!(s.dx, 1.0);
        assert_eq!(s.lines, vec![Guide::x(151.0)]);
    }

    #[test]
    fn guides_beyond_the_threshold_are_ignored() {
        let s = snap(R, &[Guide::x(93.0), Guide::y(230.0)], 6.0);
        assert_eq!(s, Snap::default());
        // Exactly at the threshold still snaps.
        assert_eq!(snap(R, &[Guide::x(94.0)], 6.0).dx, -6.0);
    }

    #[test]
    fn x_and_y_snap_independently() {
        // Only y has a guide in reach: x stays put.
        let s = snap(R, &[Guide::x(300.0), Guide::y(199.0)], 6.0);
        assert_eq!((s.dx, s.dy), (0.0, -1.0));
        assert_eq!(s.lines, vec![Guide::y(199.0)]);
    }

    #[test]
    fn every_guide_an_edge_lands_on_is_shown() {
        // Snapping the left edge to 98 also puts the right edge on 148.
        let s = snap(R, &[Guide::x(98.0), Guide::x(148.0), Guide::x(148.0)], 6.0);
        assert_eq!(s.dx, -2.0);
        assert_eq!(s.lines, vec![Guide::x(98.0), Guide::x(148.0)], "each line once");
    }

    #[test]
    fn nothing_to_snap_to() {
        assert_eq!(snap(R, &[], 6.0), Snap::default());
        // Junk never moves anything.
        let junk = [Guide::x(f32::NAN), Guide::y(f32::INFINITY)];
        assert_eq!(snap(R, &junk, 6.0), Snap::default());
        assert_eq!(snap(R, &[Guide::x(99.0)], f32::NAN), Snap::default());
        assert_eq!(snap(R, &[Guide::x(99.0)], -1.0), Snap::default());
        assert_eq!(snap(Rect::new(f32::NAN, 0.0, 1.0, 1.0), &[Guide::x(0.0)], 6.0).dx, 0.0);
    }

    #[test]
    fn candidates_are_page_margins_and_other_objects() {
        let body = Rect::new(72.0, 72.0, 468.0, 648.0);
        let g = candidates(612.0, 792.0, body, [Rect::new(10.0, 20.0, 30.0, 40.0)]);
        for x in [0.0, 306.0, 612.0, 72.0, 540.0, 10.0, 25.0, 40.0] {
            assert!(g.contains(&Guide::x(x)), "x {x}: {g:?}");
        }
        for y in [0.0, 396.0, 792.0, 72.0, 720.0, 20.0, 40.0, 60.0] {
            assert!(g.contains(&Guide::y(y)), "y {y}: {g:?}");
        }
    }
}
