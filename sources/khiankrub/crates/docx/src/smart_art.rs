//! SmartArt graphics WordCraft makes ([`SmartArtSpec`]) as diagram parts (ECMA-376 Part 1 §21.4)
//! and back.
//!
//! WordCraft lays a graphic out with its own algorithms ([`layout`]: blocks, steps with arrows,
//! a cycle, a tree, a pyramid, a radial, a matrix, overlapping circles) and writes five parts:
//! - the data model ([`data_xml`], `dgm:dataModel`): the document point, a node point per item
//!   with its text, and connections that give each item its parent;
//! - a layout definition ([`layout_xml`]), a quick style ([`quick_style_xml`]) and a colour
//!   definition ([`colors_xml`]) of our own, named by the identifiers ECMA-376 applications use
//!   for the matching built-in layouts, styles and colours;
//! - the drawing ([`drawing_xml`], `dsp:drawing`): every shape already placed, so any reader
//!   shows the graphic as WordCraft laid it out. WordCraft draws its own graphics from that same
//!   drawing ([`smart_art_items`]).
//!
//! [`spec_of`] turns a data part back into the model only when writing that model again gives
//! the same markup: a diagram another program made or changed stays read-only.

use wordcraft_doc::graphic::GraphicItem;
use wordcraft_doc::props::Rgb;
use wordcraft_doc::smart_art::{MAX_ITEMS, SmartArtColors, SmartArtItem, SmartArtLayout, SmartArtSpec};

use crate::read::diagram::{text_width, wrap};
use crate::xml::{self, El, MAX_DEPTH, Node, W};

const NS_DGM: &str = "http://schemas.openxmlformats.org/drawingml/2006/diagram";
const NS_DSP: &str = "http://schemas.microsoft.com/office/drawing/2008/diagram";
const NS_A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const NS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// `a:graphicData` URI of a diagram.
pub(crate) const DIAGRAM_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/diagram";
/// Content types of the five parts.
pub(crate) const DATA_CT: &str = "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml";
pub(crate) const LAYOUT_CT: &str = "application/vnd.openxmlformats-officedocument.drawingml.diagramLayout+xml";
pub(crate) const STYLE_CT: &str = "application/vnd.openxmlformats-officedocument.drawingml.diagramStyle+xml";
pub(crate) const COLORS_CT: &str = "application/vnd.openxmlformats-officedocument.drawingml.diagramColors+xml";
pub(crate) const DRAWING_CT: &str = "application/vnd.ms-office.drawingml.diagramDrawing+xml";
/// The extension a data model names its drawing in (ECMA-376 extension list, `dsp:dataModelExt`).
const DRAWING_EXT_URI: &str = "http://schemas.microsoft.com/office/drawing/2008/diagram";
const QUICK_STYLE_ID: &str = "urn:microsoft.com/office/officeart/2005/8/quickstyle/simple1";
const EMU_PT: f32 = 12_700.0;
/// Line height as a multiple of the font size (as the drawing reader stacks lines).
const LINE: f32 = 1.2;
/// Largest and smallest text size a graphic is given, points.
const MAX_SIZE: f32 = 24.0;
const MIN_SIZE: f32 = 5.0;
/// A sub-item's text size against its item's.
const CHILD: f32 = 0.8;
/// Model ids of the drawing's shapes start here (the data model's points are below).
const SHAPE_IDS: usize = 0x10000;

/// The layout's identifier and its gallery category.
fn layout_id(l: SmartArtLayout) -> (&'static str, &'static str) {
    match l {
        SmartArtLayout::BasicList => ("urn:microsoft.com/office/officeart/2005/8/layout/default", "list"),
        SmartArtLayout::Process => ("urn:microsoft.com/office/officeart/2005/8/layout/process1", "process"),
        SmartArtLayout::Cycle => ("urn:microsoft.com/office/officeart/2005/8/layout/cycle2", "cycle"),
        SmartArtLayout::Hierarchy => ("urn:microsoft.com/office/officeart/2005/8/layout/hierarchy1", "hierarchy"),
        SmartArtLayout::Pyramid => ("urn:microsoft.com/office/officeart/2005/8/layout/pyramid1", "pyramid"),
        SmartArtLayout::Radial => ("urn:microsoft.com/office/officeart/2005/8/layout/radial1", "cycle"),
        SmartArtLayout::Matrix => ("urn:microsoft.com/office/officeart/2005/8/layout/matrix3", "matrix"),
        SmartArtLayout::Venn => ("urn:microsoft.com/office/officeart/2005/8/layout/venn1", "relationship"),
        SmartArtLayout::VerticalBulletList => ("urn:microsoft.com/office/officeart/2005/8/layout/vList2", "list"),
        SmartArtLayout::Chevron => ("urn:microsoft.com/office/officeart/2005/8/layout/chevron1", "process"),
        SmartArtLayout::OrgChart => ("urn:microsoft.com/office/officeart/2005/8/layout/orgChart1", "hierarchy"),
        SmartArtLayout::Timeline => ("urn:microsoft.com/office/officeart/2005/8/layout/hProcess11", "process"),
        SmartArtLayout::Target => ("urn:microsoft.com/office/officeart/2005/8/layout/target1", "relationship"),
        SmartArtLayout::Funnel => ("urn:microsoft.com/office/officeart/2005/8/layout/funnel1", "relationship"),
        SmartArtLayout::Gear => ("urn:microsoft.com/office/officeart/2005/8/layout/gear1", "relationship"),
        SmartArtLayout::VerticalProcess => ("urn:microsoft.com/office/officeart/2005/8/layout/process2", "process"),
        SmartArtLayout::InvertedPyramid => ("urn:microsoft.com/office/officeart/2005/8/layout/pyramid3", "pyramid"),
        SmartArtLayout::HorizontalHierarchy => ("urn:microsoft.com/office/officeart/2005/8/layout/hierarchy2", "hierarchy"),
        SmartArtLayout::LinearVenn => ("urn:microsoft.com/office/officeart/2005/8/layout/venn3", "relationship"),
    }
}

/// The colour definition's identifier and category.
fn colors_id(c: SmartArtColors) -> (String, String) {
    match c {
        SmartArtColors::Colorful => ("urn:microsoft.com/office/officeart/2005/8/colors/colorful1".into(), "colorful".into()),
        _ => {
            let n = c.accent(0);
            (format!("urn:microsoft.com/office/officeart/2005/8/colors/accent{n}_2"), format!("accent{n}"))
        }
    }
}

/// Model id number `n` as the GUID form ECMA-376 model ids take.
fn model_id(n: usize) -> String {
    format!("{{5743A57A-0000-4000-8000-{n:012X}}}")
}

// ---- layout ----

/// What a shape is filled or outlined with.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Paint {
    None,
    /// Theme accent `n` (1–6), lightened to `tint` (thousandths of a percent kept) when set.
    Accent(usize, Option<u32>),
    /// The theme's light background colour.
    Light,
}

/// One shape of a laid-out graphic, in points from the graphic's top-left corner.
#[derive(Clone, Debug)]
struct Sh {
    prst: &'static str,
    b: [f32; 4],
    /// Clockwise turn, degrees.
    rot: f32,
    flip_h: bool,
    /// The preset's `adj` value, when not its default.
    adj: Option<i64>,
    fill: Paint,
    /// Fill opacity (thousandths of a percent) for readers that blend.
    alpha: Option<u32>,
    line: Paint,
    line_w: f32,
    /// Paragraphs: text and whether it's a sub-item (smaller, bulleted, left-aligned).
    text: Vec<(String, bool)>,
    /// Where the text goes.
    tb: [f32; 4],
    dark_text: bool,
}

impl Sh {
    /// A filled node shape with a thin light outline.
    fn node(prst: &'static str, b: [f32; 4], accent: usize) -> Sh {
        Sh {
            prst,
            b,
            rot: 0.0,
            flip_h: false,
            adj: None,
            fill: Paint::Accent(accent, None),
            alpha: None,
            line: Paint::Light,
            line_w: 1.0,
            text: Vec::new(),
            tb: inset(b, 0.06),
            dark_text: false,
        }
    }

    /// A straight connector from (`x0`, `y0`) to (`x1`, `y1`).
    fn line(x0: f32, y0: f32, x1: f32, y1: f32, accent: usize) -> Sh {
        let b = [x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs()];
        Sh {
            prst: "line",
            b,
            rot: 0.0,
            flip_h: (x1 - x0) * (y1 - y0) < 0.0,
            adj: None,
            fill: Paint::None,
            alpha: None,
            line: Paint::Accent(accent, Some(60_000)),
            line_w: 1.5,
            text: Vec::new(),
            tb: b,
            dark_text: false,
        }
    }

    fn with_text(mut self, text: Vec<(String, bool)>) -> Sh {
        self.text = text;
        self
    }

    fn text_box(mut self, tb: [f32; 4]) -> Sh {
        self.tb = tb;
        self
    }
}

/// `b` shrunk by `f` of its width and height on every side.
fn inset(b: [f32; 4], f: f32) -> [f32; 4] {
    [b[0] + b[2] * f, b[1] + b[3] * f, b[2] * (1.0 - 2.0 * f), b[3] * (1.0 - 2.0 * f)]
}

/// The inscribed square's share of a circle's box to leave on each side.
const ROUND_INSET: f32 = 0.15;

/// The text of item `i`: its own line, then the items under it as bulleted sub-items.
fn paras(spec: &SmartArtSpec, i: usize, with_children: bool) -> Vec<(String, bool)> {
    let mut out = vec![(spec.items.get(i).map(|it| it.text.clone()).unwrap_or_default(), false)];
    if with_children {
        for j in spec.descendants(i) {
            if let Some(it) = spec.items.get(j) {
                out.push((format!("• {}", it.text), true));
            }
        }
    }
    out
}

/// The top-level items.
fn tops(spec: &SmartArtSpec) -> Vec<usize> {
    spec.items.iter().enumerate().filter(|(_, it)| it.level == 0).map(|(i, _)| i).collect()
}

/// The items under item `i` as bulleted sub-items (without its own line).
fn sub_items(spec: &SmartArtSpec, i: usize) -> Vec<(String, bool)> {
    spec.descendants(i).filter_map(|j| spec.items.get(j)).map(|it| (format!("• {}", it.text), true)).collect()
}

/// A box with no fill or outline holding dark text.
fn label(b: [f32; 4], text: Vec<(String, bool)>) -> Sh {
    let mut s = Sh::node("rect", b, 1).with_text(text).text_box(b);
    s.fill = Paint::None;
    s.line = Paint::None;
    s.dark_text = true;
    s
}

/// A tree of the items: each item's children, the top-level items, and how many leaf columns
/// each item spans (an item whose children `hang` spans one).
struct Tree {
    kids: Vec<Vec<usize>>,
    roots: Vec<usize>,
    leaves: Vec<usize>,
    /// Items whose children hang in a column under them instead of spreading out.
    hangs: Vec<bool>,
}

impl Tree {
    /// The tree of `spec`; with `hanging`, an item below the top whose children have no children
    /// of their own hangs them.
    fn of(spec: &SmartArtSpec, hanging: bool) -> Tree {
        let n = spec.items.len();
        let mut kids: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut roots = Vec::new();
        for (i, p) in spec.parents().iter().enumerate() {
            match p.and_then(|p| kids.get_mut(p)) {
                Some(k) => k.push(i),
                None => roots.push(i),
            }
        }
        let hangs: Vec<bool> = (0..n)
            .map(|i| {
                hanging
                    && spec.items.get(i).is_some_and(|it| it.level > 0)
                    && kids.get(i).is_some_and(|k| !k.is_empty() && k.iter().all(|&c| kids.get(c).is_some_and(Vec::is_empty)))
            })
            .collect();
        // Leaves under each item (children come after their parent, so from the end).
        let mut leaves = vec![1usize; n];
        for i in (0..n).rev() {
            if hangs.get(i).copied().unwrap_or(false) {
                continue;
            }
            let sum: usize = kids.get(i).map_or(0, |k| k.iter().map(|&c| leaves.get(c).copied().unwrap_or(1)).sum());
            if sum > 0
                && let Some(l) = leaves.get_mut(i)
            {
                *l = sum;
            }
        }
        Tree { kids, roots, leaves, hangs }
    }

    fn kids(&self, i: usize) -> &[usize] {
        self.kids.get(i).map(Vec::as_slice).unwrap_or(&[])
    }

    fn leaves(&self, i: usize) -> usize {
        self.leaves.get(i).copied().unwrap_or(1)
    }

    fn hangs(&self, i: usize) -> bool {
        self.hangs.get(i).copied().unwrap_or(false)
    }

    /// Where each item's span of leaf columns starts, in columns.
    fn starts(&self) -> Vec<f32> {
        let mut start = vec![0.0f32; self.kids.len()];
        let mut at = 0.0;
        for &r in &self.roots {
            if let Some(s) = start.get_mut(r) {
                *s = at;
            }
            at += self.leaves(r) as f32;
        }
        for i in 0..self.kids.len() {
            if self.hangs(i) {
                continue;
            }
            let mut at = start.get(i).copied().unwrap_or(0.0);
            for &k in self.kids(i) {
                if let Some(s) = start.get_mut(k) {
                    *s = at;
                }
                at += self.leaves(k) as f32;
            }
        }
        start
    }

    /// Leaf columns in all.
    fn total(&self) -> usize {
        self.roots.iter().map(|&r| self.leaves(r)).sum::<usize>().max(1)
    }
}

/// The shapes of `spec` laid out in a `w` × `h` points box, back to front.
fn layout(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let ok = |v: f32| if v.is_finite() { v.clamp(0.0, 20_000.0) } else { 0.0 };
    let (w, h) = (ok(w), ok(h));
    if spec.items.is_empty() || w < 1.0 || h < 1.0 {
        return Vec::new();
    }
    match spec.layout {
        SmartArtLayout::BasicList => basic_list(spec, w, h),
        SmartArtLayout::Process => process(spec, w, h),
        SmartArtLayout::Cycle => cycle(spec, w, h),
        SmartArtLayout::Hierarchy => hierarchy(spec, w, h),
        SmartArtLayout::Pyramid => pyramid(spec, w, h),
        SmartArtLayout::Radial => radial(spec, w, h),
        SmartArtLayout::Matrix => matrix(spec, w, h),
        SmartArtLayout::Venn => venn(spec, w, h),
        SmartArtLayout::VerticalBulletList => vertical_bullet_list(spec, w, h),
        SmartArtLayout::Chevron => chevron(spec, w, h),
        SmartArtLayout::OrgChart => org_chart(spec, w, h),
        SmartArtLayout::Timeline => timeline(spec, w, h),
        SmartArtLayout::Target => target(spec, w, h),
        SmartArtLayout::Funnel => funnel(spec, w, h),
        SmartArtLayout::Gear => gear(spec, w, h),
        SmartArtLayout::VerticalProcess => vertical_process(spec, w, h),
        SmartArtLayout::InvertedPyramid => inverted_pyramid(spec, w, h),
        SmartArtLayout::HorizontalHierarchy => horizontal_hierarchy(spec, w, h),
        SmartArtLayout::LinearVenn => linear_venn(spec, w, h),
    }
}

/// Blocks of 3:2 in rows, as large as fit; the last row centred.
fn basic_list(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let size = |c: usize| {
        let r = n.div_ceil(c);
        let bw = (w / c as f32 * 0.9).min(h / r as f32 * 0.9 * 1.5);
        (bw, bw / 1.5)
    };
    let cols = (1..=n).max_by(|a, b| (size(*a).0).total_cmp(&size(*b).0)).unwrap_or(1);
    let rows = n.div_ceil(cols);
    let (bw, bh) = size(cols);
    let (gx, gy) = (bw * 0.1, bh * 0.1);
    let y0 = (h - (rows as f32 * bh + (rows - 1) as f32 * gy)) / 2.0;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let (r, c) = (k / cols, k % cols);
        let in_row = if r + 1 == rows { n - r * cols } else { cols };
        let x0 = (w - (in_row as f32 * bw + (in_row - 1) as f32 * gx)) / 2.0;
        let b = [x0 + c as f32 * (bw + gx), y0 + r as f32 * (bh + gy), bw, bh];
        out.push(Sh::node("rect", b, spec.colors.accent(k)).with_text(paras(spec, i, true)));
    }
    out
}

/// Steps left to right with an arrow in each gap.
fn process(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1) as f32;
    const GAP: f32 = 0.45;
    let nw = w / (n + (n - 1.0) * GAP);
    let nh = (nw * 0.6).min(h * 0.9);
    let y = (h - nh) / 2.0;
    let gap = nw * GAP;
    let mut out = Vec::new();
    for (k, _) in tops.iter().enumerate().skip(1) {
        let gx = k as f32 * (nw + gap) - gap;
        let (aw, ah) = (gap * 0.62, (nh * 0.35).min(gap * 0.7));
        let mut a = Sh::node("rightArrow", [gx + (gap - aw) / 2.0, h / 2.0 - ah / 2.0, aw, ah], spec.colors.accent(k - 1));
        a.fill = Paint::Accent(spec.colors.accent(k - 1), Some(60_000));
        a.line = Paint::None;
        out.push(a);
    }
    for (k, &i) in tops.iter().enumerate() {
        let b = [k as f32 * (nw + gap), y, nw, nh];
        out.push(Sh::node("roundRect", b, spec.colors.accent(k)).with_text(paras(spec, i, true)));
    }
    out
}

/// The angle (degrees, clockwise from three o'clock) of place `k` of `n` round a circle, the
/// first at twelve o'clock.
fn angle(k: usize, n: usize) -> f32 {
    -90.0 + 360.0 * k as f32 / n.max(1) as f32
}

/// The point at `r` from (`cx`, `cy`) at `deg`.
fn polar(cx: f32, cy: f32, r: f32, deg: f32) -> (f32, f32) {
    let (s, c) = deg.to_radians().sin_cos();
    (cx + r * c, cy + r * s)
}

/// Circles round a circle, clockwise from the top, with a curved-path arrow between each.
fn cycle(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let m = w.min(h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let step = (std::f32::consts::PI / n as f32).sin();
    let mut d = m * 0.32;
    for _ in 0..4 {
        let r = (m - d) / 2.0;
        d = if n == 1 { m * 0.5 } else { (m * 0.32).min(2.0 * r * step * 0.62) };
    }
    let r = (m - d) / 2.0;
    let mut out = Vec::new();
    if n > 1 {
        let chord = 2.0 * r * step;
        let len = (chord - d) * 0.6;
        if len > 2.0 {
            let thick = (len * 0.6).min(d * 0.25);
            for k in 0..n {
                let mid = angle(k, n) + 180.0 / n as f32;
                let (x, y) = polar(cx, cy, r, mid);
                let mut a = Sh::node("rightArrow", [x - len / 2.0, y - thick / 2.0, len, thick], spec.colors.accent(k));
                a.rot = (mid + 90.0).rem_euclid(360.0);
                a.fill = Paint::Accent(spec.colors.accent(k), Some(60_000));
                a.line = Paint::None;
                out.push(a);
            }
        }
    }
    for (k, &i) in tops.iter().enumerate() {
        let (x, y) = if n == 1 { (cx, cy) } else { polar(cx, cy, r, angle(k, n)) };
        let b = [x - d / 2.0, y - d / 2.0, d, d];
        out.push(Sh::node("ellipse", b, spec.colors.accent(k)).with_text(paras(spec, i, true)).text_box(inset(b, ROUND_INSET)));
    }
    out
}

/// A tree, a row per level: each item centred over the items under it, joined by elbow lines.
fn hierarchy(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let n = spec.items.len();
    let parents = spec.parents();
    let mut kids: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut roots = Vec::new();
    for (i, p) in parents.iter().enumerate() {
        match p.and_then(|p| kids.get_mut(p)) {
            Some(k) => k.push(i),
            None => roots.push(i),
        }
    }
    // Leaves under each item (children come after their parent, so from the end).
    let mut leaves = vec![1usize; n];
    for i in (0..n).rev() {
        let sum: usize = kids.get(i).map_or(0, |k| k.iter().map(|&c| leaves.get(c).copied().unwrap_or(1)).sum());
        if sum > 0
            && let Some(l) = leaves.get_mut(i)
        {
            *l = sum;
        }
    }
    let depth = spec.items.iter().map(|it| usize::from(it.level)).max().unwrap_or(0) + 1;
    let total: usize = roots.iter().map(|&r| leaves.get(r).copied().unwrap_or(1)).sum::<usize>().max(1);
    let unit = w / total as f32;
    let rh = h / depth as f32;
    let nw = (unit * 0.88).min(rh * 0.62 * 1.8);
    let nh = (rh * 0.62).min(nw * 0.8);
    let mut start = vec![0.0f32; n];
    let mut at = 0.0;
    for &r in &roots {
        if let Some(s) = start.get_mut(r) {
            *s = at;
        }
        at += leaves.get(r).copied().unwrap_or(1) as f32 * unit;
    }
    for i in 0..n {
        let mut at = start.get(i).copied().unwrap_or(0.0);
        for &k in kids.get(i).map(Vec::as_slice).unwrap_or(&[]) {
            if let Some(s) = start.get_mut(k) {
                *s = at;
            }
            at += leaves.get(k).copied().unwrap_or(1) as f32 * unit;
        }
    }
    let cx = |i: usize| start.get(i).copied().unwrap_or(0.0) + leaves.get(i).copied().unwrap_or(1) as f32 * unit / 2.0;
    let top = |i: usize| spec.items.get(i).map_or(0.0, |it| f32::from(it.level) * rh + (rh - nh) / 2.0);
    let mut out = Vec::new();
    for i in 0..n {
        let Some(ks) = kids.get(i).filter(|k| !k.is_empty()) else { continue };
        let accent = spec.colors.accent(i);
        let (px, pb) = (cx(i), top(i) + nh);
        let ct = ks.first().map_or(pb, |&k| top(k));
        let mid = (pb + ct) / 2.0;
        out.push(Sh::line(px, pb, px, mid, accent));
        let xs: Vec<f32> = ks.iter().map(|&k| cx(k)).collect();
        let (lo, hi) = xs.iter().fold((px, px), |(lo, hi), &x| (lo.min(x), hi.max(x)));
        if hi - lo > 0.5 {
            out.push(Sh::line(lo, mid, hi, mid, accent));
        }
        for x in xs {
            out.push(Sh::line(x, mid, x, ct, accent));
        }
    }
    for i in 0..n {
        let b = [cx(i) - nw / 2.0, top(i), nw, nh];
        out.push(Sh::node("roundRect", b, spec.colors.accent(i)).with_text(paras(spec, i, false)));
    }
    out
}

/// Bands stacked into a triangle: the first item at the narrow top.
fn pyramid(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1) as f32;
    let tw = w.min(h * 1.3);
    let x0 = (w - tw) / 2.0;
    let bh = h / n;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let (top_w, bot_w) = (tw * k as f32 / n, tw * (k as f32 + 1.0) / n);
        let b = [x0 + (tw - bot_w) / 2.0, k as f32 * bh, bot_w, bh];
        let accent = spec.colors.accent(k);
        let sh = if k == 0 {
            let tb = [b[0] + bot_w * 0.22, b[1] + bh * 0.4, bot_w * 0.56, bh * 0.56];
            Sh::node("triangle", b, accent).text_box(tb)
        } else {
            let inset_x = (bot_w - top_w) / 2.0;
            let ss = bot_w.min(bh).max(0.01);
            let mut s = Sh::node("trapezoid", b, accent);
            s.adj = Some((inset_x / ss * 100_000.0).round().clamp(0.0, 10_000_000.0) as i64);
            let mid_w = (top_w + bot_w) / 2.0 * 0.8;
            s.text_box([w / 2.0 - mid_w / 2.0, b[1] + bh * 0.08, mid_w, bh * 0.84])
        };
        out.push(sh.with_text(paras(spec, i, true)));
    }
    out
}

/// The first item in the middle; the items under it (and any later top-level items) around it,
/// each joined to the middle by a line.
fn radial(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let n_items = spec.items.len();
    let parents = spec.parents();
    let spokes: Vec<usize> = (1..n_items).filter(|&i| matches!(parents.get(i), Some(Some(0)) | Some(None))).collect();
    let n = spokes.len();
    let m = w.min(h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let mut out = Vec::new();
    if n == 0 {
        let d = m * 0.5;
        let b = [cx - d / 2.0, cy - d / 2.0, d, d];
        out.push(Sh::node("ellipse", b, spec.colors.accent(0)).with_text(paras(spec, 0, false)).text_box(inset(b, ROUND_INSET)));
        return out;
    }
    let step = if n == 1 { 1.0 } else { (std::f32::consts::PI / n as f32).sin() };
    let mut ds = m * 0.24;
    for _ in 0..4 {
        let r = (m - ds) / 2.0;
        ds = (m * 0.24).min(2.0 * r * step * 0.85);
    }
    let r = (m - ds) / 2.0;
    let dc = (m * 0.36).min(2.0 * (r - ds / 2.0 - m * 0.03)).max(m * 0.05);
    let at = |k: usize| polar(cx, cy, r, angle(k, n));
    for k in 0..n {
        let (x, y) = at(k);
        out.push(Sh::line(cx, cy, x, y, spec.colors.accent(0)));
    }
    let bc = [cx - dc / 2.0, cy - dc / 2.0, dc, dc];
    out.push(Sh::node("ellipse", bc, spec.colors.accent(0)).with_text(paras(spec, 0, false)).text_box(inset(bc, ROUND_INSET)));
    for (k, &i) in spokes.iter().enumerate() {
        let (x, y) = at(k);
        let b = [x - ds / 2.0, y - ds / 2.0, ds, ds];
        out.push(Sh::node("ellipse", b, spec.colors.accent(k + 1)).with_text(paras(spec, i, true)).text_box(inset(b, ROUND_INSET)));
    }
    out
}

/// Quadrants of a square (more items: a larger grid).
fn matrix(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cols = (1..=n).find(|c| c * c >= n).unwrap_or(n).max(1);
    let rows = n.div_ceil(cols);
    let cell = (w / cols as f32).min(h / rows as f32);
    let gap = cell * 0.04;
    let (x0, y0) = ((w - cols as f32 * cell) / 2.0, (h - rows as f32 * cell) / 2.0);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let (r, c) = (k / cols, k % cols);
        let b = [x0 + c as f32 * cell + gap / 2.0, y0 + r as f32 * cell + gap / 2.0, cell - gap, cell - gap];
        out.push(Sh::node("roundRect", b, spec.colors.accent(k)).with_text(paras(spec, i, true)).text_box(inset(b, 0.1)));
    }
    out
}

/// Overlapping circles round the middle; each one's text on its outer side.
fn venn(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let m = w.min(h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    // Ring radius against circle radius: neighbours' centres 1.5 radii apart.
    let k = if n == 1 { 0.0 } else { (0.75 / (std::f32::consts::PI / n as f32).sin()).max(0.6) };
    let r = m / (2.0 * (k + 1.0));
    let start = if n == 2 { 180.0 } else { -90.0 };
    let place = |j: usize| -> (f32, f32, f32) {
        let deg = start + 360.0 * j as f32 / n as f32;
        let (x, y) = polar(cx, cy, r * k, deg);
        (x, y, deg)
    };
    let mut out = Vec::new();
    for (j, _) in tops.iter().enumerate() {
        let (x, y, _) = place(j);
        let mut s = Sh::node("ellipse", [x - r, y - r, 2.0 * r, 2.0 * r], spec.colors.accent(j));
        s.fill = Paint::Accent(spec.colors.accent(j), Some(40_000));
        s.alpha = Some(60_000);
        s.line = Paint::None;
        out.push(s);
    }
    // Every outline over every fill, so each circle shows whole.
    for (j, _) in tops.iter().enumerate() {
        let (x, y, _) = place(j);
        let mut s = Sh::node("ellipse", [x - r, y - r, 2.0 * r, 2.0 * r], spec.colors.accent(j));
        s.fill = Paint::None;
        s.line = Paint::Accent(spec.colors.accent(j), None);
        s.line_w = 1.5;
        out.push(s);
    }
    for (j, &i) in tops.iter().enumerate() {
        let (x, y, deg) = place(j);
        let (tx, ty) = if n == 1 { (x, y) } else { polar(x, y, r * 0.35, deg) };
        let (bw, bh) = (r * 1.1, r * 0.8);
        let b = [tx - bw / 2.0, ty - bh / 2.0, bw, bh];
        let mut s = Sh::node("rect", b, spec.colors.accent(j)).with_text(paras(spec, i, true)).text_box(b);
        s.fill = Paint::None;
        s.line = Paint::None;
        s.dark_text = true;
        out.push(s);
    }
    out
}

/// Wide bars one under another; each bar's sub-items as bullets on a pale band under it.
fn vertical_bullet_list(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    const GAP: f32 = 0.2;
    const SUB: f32 = 0.55;
    let subs: Vec<usize> = tops.iter().map(|&i| spec.descendants(i).len()).collect();
    let units: f32 = subs.iter().map(|&k| 1.0 + SUB * k as f32).sum::<f32>() + GAP * tops.len().saturating_sub(1) as f32;
    let unit = (h / units.max(1.0)).min(w * 0.3);
    let mut y = (h - unit * units).max(0.0) / 2.0;
    let mut out = Vec::new();
    for (k, (&i, &sub)) in tops.iter().zip(&subs).enumerate() {
        let accent = spec.colors.accent(k);
        let bar = [0.0, y, w, unit];
        out.push(Sh::node("roundRect", bar, accent).with_text(paras(spec, i, false)).text_box([w * 0.03, y, w * 0.94, unit]));
        y += unit;
        if sub > 0 {
            let band = [w * 0.04, y, w * 0.96, unit * SUB * sub as f32];
            let mut s = label(band, sub_items(spec, i)).text_box(inset(band, 0.04));
            s.fill = Paint::Accent(accent, Some(20_000));
            s.alpha = Some(90_000);
            out.push(s);
            y += band[3];
        }
        y += unit * GAP;
    }
    out
}

/// Steps as chevrons left to right; each step's sub-items under its chevron.
fn chevron(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1) as f32;
    let subs = tops.iter().any(|&i| !spec.descendants(i).is_empty());
    // A chevron is RATIO × its height wide, its point half its height deep; neighbours GAP apart.
    const RATIO: f32 = 2.2;
    const GAP: f32 = 0.08;
    let depth_of = if subs { 2.5 } else { 1.0 };
    let ch = (w / (n * RATIO - (n - 1.0) * (0.5 - GAP))).min(h * 0.9 / depth_of);
    let (cw, d) = (ch * RATIO, ch * 0.5);
    let total = n * cw - (n - 1.0) * (d - ch * GAP);
    let x0 = (w - total).max(0.0) / 2.0;
    let y = (h - ch * depth_of).max(0.0) / 2.0;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = x0 + k as f32 * (cw - d + ch * GAP);
        let b = [x, y, cw, ch];
        out.push(Sh::node("chevron", b, spec.colors.accent(k)).with_text(paras(spec, i, false)).text_box([
            x + d * 0.9,
            y + ch * 0.1,
            cw - d * 1.8,
            ch * 0.8,
        ]));
        let sub = sub_items(spec, i);
        if !sub.is_empty() {
            out.push(label([x + d * 0.5, y + ch * 1.1, cw - d, ch * 1.3], sub));
        }
    }
    out
}

/// A tree of boxes, a row per level, joined by elbow lines; an item's children that have none of
/// their own hang in a column under it.
fn org_chart(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tree = Tree::of(spec, true);
    let n = spec.items.len();
    let parents = spec.parents();
    let hung = |i: usize| parents.get(i).copied().flatten().is_some_and(|p| tree.hangs(p));
    const STEP: f32 = 0.75;
    // Rows the chart needs: one per level, plus the hanging columns' extra rows.
    let rows = (0..n)
        .map(|i| {
            let l = spec.items.get(i).map_or(0.0, |it| f32::from(it.level));
            if tree.hangs(i) { l + 1.0 + STEP * tree.kids(i).len() as f32 + 0.1 } else { l + 1.0 }
        })
        .fold(1.0f32, f32::max);
    let unit = w / tree.total() as f32;
    let rh = h / rows;
    let nw = (unit * 0.86).min(rh * 0.6 * 2.2);
    let nh = (rh * 0.6).min(nw * 0.8);
    let start = tree.starts();
    let cx = |i: usize| start.get(i).copied().unwrap_or(0.0) * unit + tree.leaves(i) as f32 * unit / 2.0;
    let level = |i: usize| spec.items.get(i).map_or(0.0, |it| f32::from(it.level));
    let top = |i: usize| level(i) * rh + (rh - nh) / 2.0;
    let mut out = Vec::new();
    let mut boxes: Vec<[f32; 4]> = (0..n).map(|i| [cx(i) - nw / 2.0, top(i), nw, nh]).collect();
    for i in 0..n {
        let ks = tree.kids(i);
        if ks.is_empty() {
            continue;
        }
        let accent = spec.colors.accent(i);
        let p = boxes.get(i).copied().unwrap_or_default();
        let pb = p[1] + p[3];
        if tree.hangs(i) {
            // A spine down the box's left part, a stub to each hanging box.
            let sx = p[0] + nw * 0.08;
            let (hw, hh) = (nw * 0.84, (rh * STEP * 0.8).min(nh));
            let mut last = pb;
            for (k, &c) in ks.iter().enumerate() {
                let y = pb + rh * 0.15 + k as f32 * rh * STEP;
                if let Some(b) = boxes.get_mut(c) {
                    *b = [p[0] + nw - hw, y, hw, hh];
                }
                out.push(Sh::line(sx, y + hh / 2.0, p[0] + nw - hw, y + hh / 2.0, accent));
                last = y + hh / 2.0;
            }
            out.push(Sh::line(sx, pb, sx, last, accent));
            continue;
        }
        let px = cx(i);
        let ct = ks.first().map_or(pb, |&k| top(k));
        let mid = (pb + ct) / 2.0;
        out.push(Sh::line(px, pb, px, mid, accent));
        let xs: Vec<f32> = ks.iter().map(|&k| cx(k)).collect();
        let (lo, hi) = xs.iter().fold((px, px), |(lo, hi), &x| (lo.min(x), hi.max(x)));
        if hi - lo > 0.5 {
            out.push(Sh::line(lo, mid, hi, mid, accent));
        }
        for x in xs {
            out.push(Sh::line(x, mid, x, ct, accent));
        }
    }
    for (i, b) in boxes.into_iter().enumerate() {
        let accent = spec.colors.accent(if hung(i) { parents.get(i).copied().flatten().unwrap_or(i) } else { i });
        out.push(Sh::node("rect", b, accent).with_text(paras(spec, i, false)));
    }
    out
}

/// A long arrow; a dot per event on it, the event's text above and below by turns.
fn timeline(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let (cy, t) = (h / 2.0, (h * 0.07).min(w * 0.05));
    let mut arrow = Sh::node("rightArrow", [0.0, cy - t, w, 2.0 * t], spec.colors.accent(0));
    arrow.fill = Paint::Accent(spec.colors.accent(0), Some(50_000));
    arrow.line = Paint::None;
    let mut out = vec![arrow];
    let span = w * 0.94 / n as f32;
    let d = (t * 1.4).min(span * 0.4);
    // Labels sit just off the line, above and below by turns.
    let th = (h * 0.26).min(cy - t * 1.6).max(0.0);
    for (k, &i) in tops.iter().enumerate() {
        let x = (k as f32 + 0.5) * span;
        let accent = spec.colors.accent(k);
        let mut dot = Sh::node("ellipse", [x - d / 2.0, cy - d / 2.0, d, d], accent);
        dot.fill = Paint::Light;
        dot.line = Paint::Accent(accent, None);
        dot.line_w = 2.0;
        out.push(dot);
        let y = if k % 2 == 0 { cy - t * 1.6 - th } else { cy + t * 1.6 };
        out.push(label([x - span * 0.48, y, span * 0.96, th], paras(spec, i, true)));
    }
    out
}

/// Rings round a centre, the first item outermost; each ring's text at the right, a line from
/// the ring to it.
fn target(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let d = h.min(w * 0.55);
    let (cx, cy) = (d / 2.0, h / 2.0);
    let band = d / 2.0 / n as f32;
    let lx = d + w * 0.06;
    let mut out = Vec::new();
    for (k, _) in tops.iter().enumerate() {
        let r = d / 2.0 * (n - k) as f32 / n as f32;
        let accent = spec.colors.accent(k);
        let mut s = Sh::node("ellipse", [cx - r, cy - r, 2.0 * r, 2.0 * r], accent);
        if n > 1 && spec.colors.accent(0) == spec.colors.accent(1) {
            s.fill = Paint::Accent(accent, Some(35_000 + 65_000 * k as u32 / (n as u32 - 1)));
        }
        out.push(s);
    }
    for (k, &i) in tops.iter().enumerate() {
        let y = cy - d / 2.0 + band * (k as f32 + 0.5);
        out.push(Sh::line(cx, y, lx - w * 0.01, y, spec.colors.accent(k)));
        let lh = band.max(h / n as f32 * 0.9).min(band * 2.0);
        let b = [lx, (y - lh / 2.0).clamp(0.0, (h - lh).max(0.0)), (w - lx).max(0.0), lh];
        out.push(label(b, paras(spec, i, true)));
    }
    out
}

/// A funnel: every item but the last as circles in its mouth, an arrow, then the last item.
fn funnel(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let (inputs, result) = match tops.split_last() {
        Some((r, rest)) => (rest.to_vec(), *r),
        None => return Vec::new(),
    };
    let fw = w.min(h * 1.5);
    let (fh, cx) = (h * 0.62, w / 2.0);
    let accent = spec.colors.accent(0);
    let mut f = Sh::node("funnel", [cx - fw / 2.0, 0.0, fw, fh], accent);
    f.fill = Paint::Accent(accent, Some(30_000));
    f.line = Paint::Accent(accent, None);
    f.line_w = 1.5;
    let mut out = vec![f];
    let m = inputs.len();
    if m > 0 {
        let cols = m.min(3);
        let rows = m.div_ceil(cols);
        let dc = (fw * 0.8 / cols as f32).min(fh * 0.72 / rows as f32);
        for (k, &i) in inputs.iter().enumerate() {
            let (r, c) = (k / cols, k % cols);
            let in_row = if r + 1 == rows { m - r * cols } else { cols };
            let x = cx - in_row as f32 * dc / 2.0 + c as f32 * dc;
            let b = [x, fh * 0.04 + r as f32 * dc * 0.92, dc * 0.94, dc * 0.94];
            out.push(Sh::node("ellipse", b, spec.colors.accent(k + 1)).with_text(paras(spec, i, true)).text_box(inset(b, ROUND_INSET)));
        }
    }
    let (aw, ah) = (fw * 0.12, h * 0.12);
    let mut a = Sh::node("downArrow", [cx - aw / 2.0, fh + h * 0.01, aw, ah], accent);
    a.fill = Paint::Accent(accent, Some(60_000));
    a.line = Paint::None;
    out.push(a);
    let ry = fh + h * 0.14;
    let rw = fw * 0.6;
    out.push(Sh::node("roundRect", [cx - rw / 2.0, ry, rw, (h - ry).max(0.0)], spec.colors.accent(m)).with_text(paras(spec, result, true)));
    out
}

/// Up to three meshing gears, large to small, each holding an item's text.
fn gear(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    // Centres and radii in a 1.3 × 1 frame.
    const PLACES: [(f32, f32, f32); 3] = [(0.45, 0.58, 0.36), (0.98, 0.3, 0.26), (1.0, 0.78, 0.2)];
    let (fw, fh) = if tops.len() == 1 { (1.0, 1.0) } else { (1.3, 1.0) };
    let k = (w / fw).min(h / fh);
    let (ox, oy) = ((w - fw * k) / 2.0, (h - fh * k) / 2.0);
    let mut out = Vec::new();
    for (j, (&i, &(x, y, r))) in tops.iter().zip(PLACES.iter()).enumerate() {
        let (x, y, r) = if tops.len() == 1 { (0.5, 0.5, 0.48) } else { (x, y, r) };
        let b = [ox + (x - r) * k, oy + (y - r) * k, 2.0 * r * k, 2.0 * r * k];
        let prst = if j == 0 { "gear9" } else { "gear6" };
        out.push(Sh::node(prst, b, spec.colors.accent(j)).with_text(paras(spec, i, true)).text_box(inset(b, 0.24)));
    }
    out
}

/// Steps top to bottom with a down arrow in each gap.
fn vertical_process(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1) as f32;
    const GAP: f32 = 0.6;
    let nh = h / (n + (n - 1.0) * GAP);
    let nw = (nh * 3.0).min(w * 0.9);
    let gap = nh * GAP;
    let x = (w - nw) / 2.0;
    let mut out = Vec::new();
    for (k, _) in tops.iter().enumerate().skip(1) {
        let gy = k as f32 * (nh + gap) - gap;
        let (aw, ah) = ((nw * 0.2).min(gap * 0.9), gap * 0.7);
        let mut a = Sh::node("downArrow", [w / 2.0 - aw / 2.0, gy + (gap - ah) / 2.0, aw, ah], spec.colors.accent(k - 1));
        a.fill = Paint::Accent(spec.colors.accent(k - 1), Some(60_000));
        a.line = Paint::None;
        out.push(a);
    }
    for (k, &i) in tops.iter().enumerate() {
        let b = [x, k as f32 * (nh + gap), nw, nh];
        out.push(Sh::node("roundRect", b, spec.colors.accent(k)).with_text(paras(spec, i, true)));
    }
    out
}

/// Bands stacked into an upside-down triangle: the first item at the wide top.
fn inverted_pyramid(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1) as f32;
    let tw = w.min(h * 1.3);
    let x0 = (w - tw) / 2.0;
    let bh = h / n;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let last = k + 1 == tops.len();
        let (top_w, bot_w) = (tw * (n - k as f32) / n, tw * (n - k as f32 - 1.0) / n);
        let b = [x0 + (tw - top_w) / 2.0, k as f32 * bh, top_w, bh];
        let accent = spec.colors.accent(k);
        // The upright shapes turned half round; their text stays upright.
        let mut s = if last {
            Sh::node("triangle", b, accent).text_box([b[0] + top_w * 0.22, b[1] + bh * 0.04, top_w * 0.56, bh * 0.56])
        } else {
            let inset_x = (top_w - bot_w) / 2.0;
            let ss = top_w.min(bh).max(0.01);
            let mut s = Sh::node("trapezoid", b, accent);
            s.adj = Some((inset_x / ss * 100_000.0).round().clamp(0.0, 10_000_000.0) as i64);
            let mid_w = (top_w + bot_w) / 2.0 * 0.8;
            s.text_box([w / 2.0 - mid_w / 2.0, b[1] + bh * 0.08, mid_w, bh * 0.84])
        };
        s.rot = 180.0;
        out.push(s.with_text(paras(spec, i, true)));
    }
    out
}

/// A tree from left to right, a column per level: each item level with the items under it,
/// joined by elbow lines.
fn horizontal_hierarchy(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tree = Tree::of(spec, false);
    let n = spec.items.len();
    let depth = spec.items.iter().map(|it| usize::from(it.level)).max().unwrap_or(0) + 1;
    let unit = h / tree.total() as f32;
    let cw = w / depth as f32;
    let nh = (unit * 0.82).min(cw * 0.62 / 1.6);
    let nw = (cw * 0.7).min(nh * 3.0);
    let start = tree.starts();
    let cy = |i: usize| start.get(i).copied().unwrap_or(0.0) * unit + tree.leaves(i) as f32 * unit / 2.0;
    let left = |i: usize| spec.items.get(i).map_or(0.0, |it| f32::from(it.level) * cw + (cw - nw) / 2.0);
    let mut out = Vec::new();
    for i in 0..n {
        let ks = tree.kids(i);
        if ks.is_empty() {
            continue;
        }
        let accent = spec.colors.accent(i);
        let (py, pr) = (cy(i), left(i) + nw);
        let cl = ks.first().map_or(pr, |&k| left(k));
        let mid = (pr + cl) / 2.0;
        out.push(Sh::line(pr, py, mid, py, accent));
        let ys: Vec<f32> = ks.iter().map(|&k| cy(k)).collect();
        let (lo, hi) = ys.iter().fold((py, py), |(lo, hi), &y| (lo.min(y), hi.max(y)));
        if hi - lo > 0.5 {
            out.push(Sh::line(mid, lo, mid, hi, accent));
        }
        for y in ys {
            out.push(Sh::line(mid, y, cl, y, accent));
        }
    }
    for i in 0..n {
        let b = [left(i), cy(i) - nh / 2.0, nw, nh];
        out.push(Sh::node("roundRect", b, spec.colors.accent(i)).with_text(paras(spec, i, false)));
    }
    out
}

/// Circles in a row, each overlapping the next; each one's text in its middle.
fn linear_venn(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1) as f32;
    const STEP: f32 = 0.75;
    let d = (w / (1.0 + STEP * (n - 1.0))).min(h);
    let x0 = (w - d * (1.0 + STEP * (n - 1.0))) / 2.0;
    let y = (h - d) / 2.0;
    let at = |k: usize| [x0 + k as f32 * d * STEP, y, d, d];
    let mut out = Vec::new();
    for (k, _) in tops.iter().enumerate() {
        let mut s = Sh::node("ellipse", at(k), spec.colors.accent(k));
        s.fill = Paint::Accent(spec.colors.accent(k), Some(40_000));
        s.alpha = Some(60_000);
        s.line = Paint::None;
        out.push(s);
    }
    for (k, _) in tops.iter().enumerate() {
        let mut s = Sh::node("ellipse", at(k), spec.colors.accent(k));
        s.fill = Paint::None;
        s.line = Paint::Accent(spec.colors.accent(k), None);
        s.line_w = 1.5;
        out.push(s);
    }
    for (k, &i) in tops.iter().enumerate() {
        let b = at(k);
        out.push(label([b[0] + d * 0.27, b[1] + d * 0.2, d * 0.46, d * 0.6], paras(spec, i, true)));
    }
    out
}

// ---- text ----

/// Whether `paras` fit a `w` × `h` text box at `size` points (sub-items at [`CHILD`] of it).
fn fits(paras: &[(String, bool)], size: f32, w: f32, h: f32) -> bool {
    let mut total = 0.0;
    for (t, child) in paras {
        let s = if *child { child_size(size) } else { size };
        if t.split_whitespace().any(|word| text_width(word, s) > w) {
            return false;
        }
        let lines = wrap(t, s, w, 10_000).len().max(1);
        total += lines as f32 * s * LINE;
    }
    total <= h
}

fn child_size(size: f32) -> f32 {
    (size * CHILD * 2.0).round() / 2.0
}

/// The largest text size (half points, [`MIN_SIZE`]–[`MAX_SIZE`]) at which every shape's text fits.
fn text_size(shapes: &[Sh]) -> f32 {
    let mut size = MAX_SIZE;
    while size > MIN_SIZE && !shapes.iter().filter(|s| !s.text.is_empty()).all(|s| fits(&s.text, size, s.tb[2], s.tb[3])) {
        size -= 0.5;
    }
    size
}

// ---- parts ----

/// A length in points as EMU text, bounded.
fn emu(pt: f32) -> String {
    let v = if pt.is_finite() { (pt * EMU_PT).round() } else { 0.0 };
    format!("{}", v.clamp(-1.0e9, 1.0e9) as i64)
}

fn paint(w: &mut W, p: Paint, alpha: Option<u32>) {
    match p {
        Paint::None => w.empty("a:noFill", &[]),
        Paint::Light => {
            w.open("a:solidFill", &[]);
            w.empty("a:schemeClr", &[("val", "lt1")]);
            w.close("a:solidFill");
        }
        Paint::Accent(n, tint) => {
            w.open("a:solidFill", &[]);
            let val = format!("accent{}", n.clamp(1, 6));
            if tint.is_none() && alpha.is_none() {
                w.empty("a:schemeClr", &[("val", &val)]);
            } else {
                w.open("a:schemeClr", &[("val", &val)]);
                if let Some(t) = tint {
                    w.empty("a:tint", &[("val", &t.to_string())]);
                }
                if let Some(a) = alpha {
                    w.empty("a:alpha", &[("val", &a.to_string())]);
                }
                w.close("a:schemeClr");
            }
            w.close("a:solidFill");
        }
    }
}

/// The `dsp:drawing` of `spec` laid out `w` × `h` points: its shapes, back to front.
pub fn drawing_xml(spec: &SmartArtSpec, w: f32, h: f32) -> String {
    let shapes = layout(spec, w, h);
    let size = text_size(&shapes);
    let mut x = W::new();
    x.open("dsp:drawing", &[("xmlns:dgm", NS_DGM), ("xmlns:dsp", NS_DSP), ("xmlns:a", NS_A)]);
    x.open("dsp:spTree", &[]);
    x.open("dsp:nvGrpSpPr", &[]);
    x.empty("dsp:cNvPr", &[("id", "0"), ("name", "")]);
    x.empty("dsp:cNvGrpSpPr", &[]);
    x.close("dsp:nvGrpSpPr");
    x.empty("dsp:grpSpPr", &[]);
    for (k, sh) in shapes.iter().enumerate() {
        shape_xml(&mut x, sh, &model_id(SHAPE_IDS + k), size);
    }
    x.close("dsp:spTree");
    x.close("dsp:drawing");
    x.s
}

fn shape_xml(x: &mut W, sh: &Sh, id: &str, size: f32) {
    x.open("dsp:sp", &[("modelId", id)]);
    x.open("dsp:nvSpPr", &[]);
    x.empty("dsp:cNvPr", &[("id", "0"), ("name", "")]);
    x.empty("dsp:cNvSpPr", &[]);
    x.close("dsp:nvSpPr");
    x.open("dsp:spPr", &[]);
    let rot = ((sh.rot.rem_euclid(360.0) * 60_000.0).round() as i64).to_string();
    let mut attrs: Vec<(&str, &str)> = Vec::new();
    if sh.rot != 0.0 {
        attrs.push(("rot", &rot));
    }
    if sh.flip_h {
        attrs.push(("flipH", "1"));
    }
    x.open("a:xfrm", &attrs);
    x.empty("a:off", &[("x", &emu(sh.b[0])), ("y", &emu(sh.b[1]))]);
    x.empty("a:ext", &[("cx", &emu(sh.b[2].max(0.0))), ("cy", &emu(sh.b[3].max(0.0)))]);
    x.close("a:xfrm");
    x.open("a:prstGeom", &[("prst", sh.prst)]);
    match sh.adj {
        Some(v) => {
            x.open("a:avLst", &[]);
            x.empty("a:gd", &[("name", "adj"), ("fmla", &format!("val {v}"))]);
            x.close("a:avLst");
        }
        None => x.empty("a:avLst", &[]),
    }
    x.close("a:prstGeom");
    paint(x, sh.fill, sh.alpha);
    if sh.line != Paint::None && sh.line_w > 0.0 {
        x.open("a:ln", &[("w", &emu(sh.line_w))]);
        paint(x, sh.line, None);
        x.close("a:ln");
    } else {
        x.open("a:ln", &[]);
        x.empty("a:noFill", &[]);
        x.close("a:ln");
    }
    x.close("dsp:spPr");
    let text_clr = if sh.dark_text { "tx1" } else { "lt1" };
    x.open("dsp:style", &[]);
    for (name, idx) in [("a:lnRef", "2"), ("a:fillRef", "1"), ("a:effectRef", "0")] {
        x.open(name, &[("idx", idx)]);
        x.empty("a:scrgbClr", &[("r", "0"), ("g", "0"), ("b", "0")]);
        x.close(name);
    }
    x.open("a:fontRef", &[("idx", "minor")]);
    x.empty("a:schemeClr", &[("val", text_clr)]);
    x.close("a:fontRef");
    x.close("dsp:style");
    if !sh.text.is_empty() {
        x.open("dsp:txBody", &[]);
        x.open(
            "a:bodyPr",
            &[
                ("spcFirstLastPara", "0"),
                ("vert", "horz"),
                ("wrap", "square"),
                ("lIns", "0"),
                ("tIns", "0"),
                ("rIns", "0"),
                ("bIns", "0"),
                ("numCol", "1"),
                ("spcCol", "0"),
                ("anchor", "ctr"),
                ("anchorCtr", "0"),
            ],
        );
        x.empty("a:noAutofit", &[]);
        x.close("a:bodyPr");
        x.empty("a:lstStyle", &[]);
        for (t, child) in &sh.text {
            let s = if *child { child_size(size) } else { size };
            x.open("a:p", &[]);
            x.open("a:pPr", &[("lvl", if *child { "1" } else { "0" }), ("algn", if *child { "l" } else { "ctr" })]);
            x.empty("a:buNone", &[]);
            x.close("a:pPr");
            if t.is_empty() {
                x.empty("a:endParaRPr", &[("lang", "en-US"), ("sz", &((s * 100.0).round() as i64).to_string())]);
            } else {
                x.open("a:r", &[]);
                x.open("a:rPr", &[("lang", "en-US"), ("sz", &((s * 100.0).round() as i64).to_string())]);
                x.open("a:solidFill", &[]);
                x.empty("a:schemeClr", &[("val", text_clr)]);
                x.close("a:solidFill");
                x.close("a:rPr");
                x.leaf("a:t", &[], t);
                x.close("a:r");
            }
            x.close("a:p");
        }
        x.close("dsp:txBody");
        x.open("dsp:txXfrm", &[]);
        x.empty("a:off", &[("x", &emu(sh.tb[0])), ("y", &emu(sh.tb[1]))]);
        x.empty("a:ext", &[("cx", &emu(sh.tb[2].max(0.0))), ("cy", &emu(sh.tb[3].max(0.0)))]);
        x.close("dsp:txXfrm");
    }
    x.close("dsp:sp");
}

/// What `spec` draws as, `w` × `h` points, in the colours of `theme`: its drawing part, read the
/// way SmartArt drawings from files are.
pub fn smart_art_items(spec: &SmartArtSpec, theme: &[Rgb], w: f32, h: f32) -> Vec<GraphicItem> {
    match xml::parse(drawing_xml(spec, w, h).as_bytes()) {
        Ok(d) => crate::read::diagram::diagram_items(&d, theme, w, h, &mut |_| None),
        Err(_) => Vec::new(),
    }
}

/// Ids of the points and connections of item `i`: (node, connection, parent transition, sibling
/// transition).
fn item_ids(i: usize) -> (String, String, String, String) {
    let b = 1 + 4 * i;
    (model_id(b), model_id(b + 1), model_id(b + 2), model_id(b + 3))
}

/// An empty point text body.
fn empty_t(w: &mut W) {
    w.open("dgm:t", &[]);
    w.empty("a:bodyPr", &[]);
    w.empty("a:lstStyle", &[]);
    w.open("a:p", &[]);
    w.empty("a:endParaRPr", &[("lang", "en-US")]);
    w.close("a:p");
    w.close("dgm:t");
}

/// The data part (`dgm:dataModel`) of `spec`; its drawing is the story part's relationship
/// `drawing_rel`.
pub fn data_xml(spec: &SmartArtSpec, drawing_rel: &str) -> String {
    let (lo, lo_cat) = layout_id(spec.layout);
    let (cs, cs_cat) = colors_id(spec.colors);
    let doc = model_id(0);
    let mut w = W::new();
    w.open("dgm:dataModel", &[("xmlns:dgm", NS_DGM), ("xmlns:a", NS_A)]);
    w.open("dgm:ptLst", &[]);
    w.open("dgm:pt", &[("modelId", &doc), ("type", "doc")]);
    w.empty(
        "dgm:prSet",
        &[("loTypeId", lo), ("loCatId", lo_cat), ("qsTypeId", QUICK_STYLE_ID), ("qsCatId", "simple"), ("csTypeId", &cs), ("csCatId", &cs_cat)],
    );
    w.empty("dgm:spPr", &[]);
    empty_t(&mut w);
    w.close("dgm:pt");
    for (i, it) in spec.items.iter().enumerate() {
        let (node, cxn, par, sib) = item_ids(i);
        w.open("dgm:pt", &[("modelId", &node)]);
        w.empty("dgm:prSet", &[]);
        w.empty("dgm:spPr", &[]);
        w.open("dgm:t", &[]);
        w.empty("a:bodyPr", &[]);
        w.empty("a:lstStyle", &[]);
        w.open("a:p", &[]);
        w.open("a:r", &[]);
        w.empty("a:rPr", &[("lang", "en-US")]);
        w.leaf("a:t", &[], &it.text);
        w.close("a:r");
        w.close("a:p");
        w.close("dgm:t");
        w.close("dgm:pt");
        for (id, ty) in [(&par, "parTrans"), (&sib, "sibTrans")] {
            w.open("dgm:pt", &[("modelId", id), ("type", ty), ("cxnId", &cxn)]);
            w.empty("dgm:prSet", &[]);
            w.empty("dgm:spPr", &[]);
            empty_t(&mut w);
            w.close("dgm:pt");
        }
    }
    w.close("dgm:ptLst");
    w.open("dgm:cxnLst", &[]);
    let parents = spec.parents();
    let mut order = vec![0usize; spec.items.len() + 1];
    for (i, p) in parents.iter().enumerate() {
        let (node, cxn, par, sib) = item_ids(i);
        let src = p.map_or_else(|| doc.clone(), |p| item_ids(p).0);
        let slot = p.map_or(0, |p| p + 1);
        let ord = order.get(slot).copied().unwrap_or(0);
        if let Some(o) = order.get_mut(slot) {
            *o += 1;
        }
        w.empty(
            "dgm:cxn",
            &[
                ("modelId", &cxn),
                ("srcId", &src),
                ("destId", &node),
                ("srcOrd", &ord.to_string()),
                ("destOrd", "0"),
                ("parTransId", &par),
                ("sibTransId", &sib),
            ],
        );
    }
    w.close("dgm:cxnLst");
    w.empty("dgm:bg", &[]);
    w.empty("dgm:whole", &[]);
    w.open("dgm:extLst", &[]);
    w.open("a:ext", &[("uri", DRAWING_EXT_URI)]);
    w.empty(
        "dsp:dataModelExt",
        &[("xmlns:dsp", NS_DSP), ("relId", drawing_rel), ("minVer", "http://schemas.openxmlformats.org/drawingml/2006/diagram")],
    );
    w.close("a:ext");
    w.close("dgm:extLst");
    w.close("dgm:dataModel");
    w.s
}

/// A layout node: `alg` with its parameters, a shape (`None`: no geometry), what text it shows
/// (`presOf` attributes), then `body`.
fn layout_node(
    w: &mut W,
    name: &str,
    style: Option<&str>,
    alg: &str,
    params: &[(&str, &str)],
    shape: Option<&str>,
    pres_of: &[(&str, &str)],
    body: impl FnOnce(&mut W),
) {
    let mut attrs = vec![("name", name)];
    if let Some(s) = style {
        attrs.push(("styleLbl", s));
    }
    w.open("dgm:layoutNode", &attrs);
    if params.is_empty() {
        w.empty("dgm:alg", &[("type", alg)]);
    } else {
        w.open("dgm:alg", &[("type", alg)]);
        for (t, v) in params {
            w.empty("dgm:param", &[("type", t), ("val", v)]);
        }
        w.close("dgm:alg");
    }
    let mut sa = vec![("r:blip", "")];
    if let Some(t) = shape {
        sa.insert(0, ("type", t));
    }
    w.open("dgm:shape", &sa);
    w.empty("dgm:adjLst", &[]);
    w.close("dgm:shape");
    w.empty("dgm:presOf", pres_of);
    w.empty("dgm:constrLst", &[]);
    w.empty("dgm:ruleLst", &[]);
    body(w);
    w.close("dgm:layoutNode");
}

/// The text node of an item: its own text and the items under it as bullets.
const ITEM_TEXT: &[(&str, &str)] = &[("axis", "desc;self"), ("ptType", "node;node"), ("st", "1;1"), ("cnt", "0;1")];

/// A layout definition (`dgm:layoutDef`) of our own for `l`, named by its layout identifier.
pub fn layout_xml(l: SmartArtLayout) -> String {
    let (id, cat) = layout_id(l);
    let mut w = W::new();
    w.open("dgm:layoutDef", &[("xmlns:dgm", NS_DGM), ("xmlns:a", NS_A), ("xmlns:r", NS_R), ("uniqueId", id)]);
    w.empty("dgm:title", &[("val", l.label())]);
    w.empty("dgm:desc", &[("val", l.description())]);
    w.open("dgm:catLst", &[]);
    w.empty("dgm:cat", &[("type", cat), ("pri", "1000")]);
    w.close("dgm:catLst");
    let (alg, params): (&str, &[(&str, &str)]) = match l {
        SmartArtLayout::BasicList | SmartArtLayout::Matrix => ("snake", &[("grDir", "tL"), ("flowDir", "row"), ("contDir", "sameDir")]),
        SmartArtLayout::Process | SmartArtLayout::Chevron | SmartArtLayout::Timeline | SmartArtLayout::LinearVenn => {
            ("lin", &[("linDir", "fromL"), ("nodeVertAlign", "mid")])
        }
        SmartArtLayout::VerticalBulletList | SmartArtLayout::VerticalProcess => ("lin", &[("linDir", "fromT"), ("nodeHorzAlign", "ctr")]),
        SmartArtLayout::Cycle | SmartArtLayout::Venn => ("cycle", &[("stAng", "0"), ("spanAng", "360")]),
        SmartArtLayout::Radial => ("cycle", &[("stAng", "0"), ("spanAng", "360"), ("ctrShpMap", "fNode")]),
        SmartArtLayout::Hierarchy | SmartArtLayout::OrgChart => ("hierChild", &[("linDir", "fromL"), ("chAlign", "t")]),
        SmartArtLayout::HorizontalHierarchy => ("hierChild", &[("linDir", "fromT"), ("chAlign", "l")]),
        SmartArtLayout::Pyramid => ("pyra", &[("linDir", "fromT")]),
        SmartArtLayout::InvertedPyramid => ("pyra", &[("linDir", "fromB")]),
        SmartArtLayout::Target | SmartArtLayout::Funnel | SmartArtLayout::Gear => ("composite", &[]),
    };
    let shape = match l {
        SmartArtLayout::BasicList | SmartArtLayout::OrgChart => "rect",
        SmartArtLayout::Process
        | SmartArtLayout::Hierarchy
        | SmartArtLayout::HorizontalHierarchy
        | SmartArtLayout::Matrix
        | SmartArtLayout::VerticalBulletList
        | SmartArtLayout::VerticalProcess => "roundRect",
        SmartArtLayout::Cycle
        | SmartArtLayout::Radial
        | SmartArtLayout::Venn
        | SmartArtLayout::LinearVenn
        | SmartArtLayout::Target
        | SmartArtLayout::Timeline
        | SmartArtLayout::Funnel => "ellipse",
        SmartArtLayout::Pyramid | SmartArtLayout::InvertedPyramid => "trapezoid",
        SmartArtLayout::Chevron => "chevron",
        SmartArtLayout::Gear => "gear9",
    };
    layout_node(&mut w, "diagram", None, alg, params, None, &[], |w| match l {
        SmartArtLayout::Hierarchy | SmartArtLayout::OrgChart | SmartArtLayout::HorizontalHierarchy => {
            let below: &[(&str, &str)] =
                if l == SmartArtLayout::HorizontalHierarchy { &[("linDir", "fromT"), ("chAlign", "l")] } else { &[("chAlign", "t")] };
            let ends = if l == SmartArtLayout::HorizontalHierarchy { ("midR", "midL") } else { ("bCtr", "tCtr") };
            w.open("dgm:forEach", &[("name", "branches"), ("axis", "ch"), ("ptType", "node")]);
            layout_node(w, "branch", None, "hierRoot", &[], None, &[], |w| {
                layout_node(w, "item", Some("node1"), "tx", &[], Some(shape), &[("axis", "self")], |_| {});
                layout_node(w, "below", None, "hierChild", below, None, &[], |w| {
                    w.open("dgm:forEach", &[("name", "joins"), ("axis", "ch"), ("ptType", "parTrans")]);
                    layout_node(
                        w,
                        "join",
                        Some("connector"),
                        "conn",
                        &[("dim", "1D"), ("connRout", "bend"), ("begPts", ends.0), ("endPts", ends.1)],
                        Some("conn"),
                        &[("axis", "self")],
                        |_| {},
                    );
                    w.close("dgm:forEach");
                    w.empty("dgm:forEach", &[("name", "deeper"), ("ref", "branches")]);
                });
            });
            w.close("dgm:forEach");
        }
        SmartArtLayout::Radial => {
            w.open("dgm:forEach", &[("name", "middle"), ("axis", "ch"), ("ptType", "node"), ("cnt", "1")]);
            layout_node(w, "centre", Some("node1"), "tx", &[], Some(shape), &[("axis", "self")], |w| {
                w.open("dgm:forEach", &[("name", "around"), ("axis", "ch"), ("ptType", "node")]);
                layout_node(w, "spoke", Some("node1"), "tx", &[], Some(shape), ITEM_TEXT, |_| {});
                w.close("dgm:forEach");
            });
            w.close("dgm:forEach");
        }
        _ => {
            w.open("dgm:forEach", &[("name", "items"), ("axis", "ch"), ("ptType", "node")]);
            let style = if matches!(l, SmartArtLayout::Venn | SmartArtLayout::LinearVenn) { "venn" } else { "node1" };
            layout_node(w, "item", Some(style), "tx", &[], Some(shape), ITEM_TEXT, |_| {});
            if matches!(l, SmartArtLayout::Process | SmartArtLayout::Cycle | SmartArtLayout::VerticalProcess) {
                let arrow = if l == SmartArtLayout::VerticalProcess { "downArrow" } else { "rightArrow" };
                w.open("dgm:forEach", &[("name", "arrows"), ("axis", "followSib"), ("ptType", "sibTrans"), ("cnt", "1")]);
                layout_node(w, "arrow", Some("connector"), "conn", &[("dim", "2D")], Some(arrow), &[("axis", "self")], |_| {});
                w.close("dgm:forEach");
            }
            w.close("dgm:forEach");
        }
    });
    w.close("dgm:layoutDef");
    w.s
}

/// The style labels our layouts name.
const STYLE_LABELS: [&str; 3] = ["node1", "connector", "venn"];

fn scheme_list(w: &mut W, tag: &str, meth: &str, vals: &[String]) {
    w.open(tag, &[("meth", meth)]);
    for v in vals {
        w.empty("a:schemeClr", &[("val", v)]);
    }
    w.close(tag);
}

/// A colour definition (`dgm:colorsDef`) of our own for `c`.
pub fn colors_xml(c: SmartArtColors) -> String {
    let (id, cat) = colors_id(c);
    let accents: Vec<String> = match c {
        SmartArtColors::Colorful => (1..=6).map(|n| format!("accent{n}")).collect(),
        _ => vec![format!("accent{}", c.accent(0))],
    };
    let meth = if c == SmartArtColors::Colorful { "cycle" } else { "repeat" };
    let mut w = W::new();
    w.open("dgm:colorsDef", &[("xmlns:dgm", NS_DGM), ("xmlns:a", NS_A), ("uniqueId", &id)]);
    w.empty("dgm:title", &[("val", c.label())]);
    w.empty("dgm:desc", &[("val", "")]);
    w.open("dgm:catLst", &[]);
    w.empty("dgm:cat", &[("type", &cat), ("pri", "1000")]);
    w.close("dgm:catLst");
    for lbl in STYLE_LABELS {
        w.open("dgm:styleLbl", &[("name", lbl)]);
        scheme_list(&mut w, "dgm:fillClrLst", meth, &accents);
        scheme_list(&mut w, "dgm:linClrLst", "repeat", &["lt1".to_string()]);
        w.empty("dgm:effectClrLst", &[]);
        w.empty("dgm:txLinClrLst", &[]);
        scheme_list(&mut w, "dgm:txFillClrLst", "repeat", &[if lbl == "venn" { "tx1" } else { "lt1" }.to_string()]);
        w.empty("dgm:txEffectClrLst", &[]);
        w.close("dgm:styleLbl");
    }
    w.close("dgm:colorsDef");
    w.s
}

/// The quick style (`dgm:styleDef`) of our own: flat shapes in the theme's plain line and fill.
pub fn quick_style_xml() -> String {
    let mut w = W::new();
    w.open("dgm:styleDef", &[("xmlns:dgm", NS_DGM), ("xmlns:a", NS_A), ("uniqueId", QUICK_STYLE_ID)]);
    w.empty("dgm:title", &[("val", "Flat")]);
    w.empty("dgm:desc", &[("val", "")]);
    w.open("dgm:catLst", &[]);
    w.empty("dgm:cat", &[("type", "simple"), ("pri", "1000")]);
    w.close("dgm:catLst");
    let scene = |w: &mut W| {
        w.open("dgm:scene3d", &[]);
        w.empty("a:camera", &[("prst", "orthographicFront")]);
        w.empty("a:lightRig", &[("rig", "threePt"), ("dir", "t")]);
        w.close("dgm:scene3d");
    };
    scene(&mut w);
    for lbl in STYLE_LABELS {
        w.open("dgm:styleLbl", &[("name", lbl)]);
        scene(&mut w);
        w.empty("dgm:sp3d", &[]);
        w.empty("dgm:txPr", &[]);
        w.open("dgm:style", &[]);
        for (name, idx) in [("a:lnRef", "2"), ("a:fillRef", "1"), ("a:effectRef", "0")] {
            w.open(name, &[("idx", idx)]);
            w.empty("a:scrgbClr", &[("r", "0"), ("g", "0"), ("b", "0")]);
            w.close(name);
        }
        w.open("a:fontRef", &[("idx", "minor")]);
        w.empty("a:schemeClr", &[("val", if lbl == "venn" { "tx1" } else { "lt1" })]);
        w.close("a:fontRef");
        w.close("dgm:style");
        w.close("dgm:styleLbl");
    }
    w.close("dgm:styleDef");
    w.s
}

// ---- reading back ----

/// The graphic a data part holds, when the part is exactly what [`data_xml`] writes for it (so
/// editing and saving it loses nothing). `None` for any other diagram.
pub(crate) fn spec_of(data: &El) -> Option<SmartArtSpec> {
    let pts = data.child("dgm:ptLst")?;
    let doc = pts.children("dgm:pt").find(|p| p.attr("type") == Some("doc"))?;
    let pr = doc.child("dgm:prSet")?;
    let lo = pr.attr("loTypeId")?;
    let layout = SmartArtLayout::ALL.into_iter().find(|l| layout_id(*l).0 == lo)?;
    let cs = pr.attr("csTypeId")?;
    let colors = SmartArtColors::ALL.into_iter().find(|c| colors_id(*c).0 == cs)?;
    let rel = data.find("dsp:dataModelExt")?.attr("relId")?.to_string();
    // Items are node points (no type) in order; more than the model holds can't be ours.
    let nodes: Vec<&El> = pts.children("dgm:pt").filter(|p| p.attr("type").is_none()).take(MAX_ITEMS + 1).collect();
    if nodes.len() > MAX_ITEMS {
        return None;
    }
    let doc_id = doc.attr("modelId")?;
    let index: Vec<&str> = nodes.iter().filter_map(|p| p.attr("modelId")).collect();
    let mut levels: Vec<Option<u8>> = vec![None; nodes.len()];
    for c in data.child("dgm:cxnLst")?.children("dgm:cxn").take(MAX_ITEMS + 1) {
        let dest = index.iter().position(|id| Some(*id) == c.attr("destId"))?;
        let src = c.attr("srcId")?;
        let level = if src == doc_id {
            0
        } else {
            let p = index.iter().position(|id| *id == src)?;
            levels.get(p).copied().flatten()?.checked_add(1)?
        };
        *levels.get_mut(dest)? = Some(level);
    }
    let items = nodes
        .iter()
        .zip(&levels)
        .map(|(p, l)| {
            let text: String = p.child("dgm:t")?.child("a:p")?.children("a:r").filter_map(|r| r.child("a:t")).map(|t| t.text()).collect();
            Some(SmartArtItem { text, level: (*l)? })
        })
        .collect::<Option<Vec<_>>>()?;
    let spec = SmartArtSpec { layout, items, colors }.sanitized();
    // Editable only when nothing would be lost: writing the model again gives the same part.
    let again = xml::parse(data_xml(&spec, &rel).as_bytes()).ok()?;
    same(data, &again, 0).then_some(spec)
}

/// Whether two elements are the same markup: names, attributes (in any order), child elements in
/// order and text (whitespace-only text between elements ignored).
fn same(a: &El, b: &El, depth: usize) -> bool {
    if depth > MAX_DEPTH || a.name != b.name || a.attrs.len() != b.attrs.len() {
        return false;
    }
    if !a.attrs.iter().all(|(k, v)| b.attr(k) == Some(v.as_str())) {
        return false;
    }
    let text = |e: &El| -> String {
        e.kids
            .iter()
            .filter_map(|n| match n {
                Node::Text(t) if !t.trim().is_empty() => Some(t.as_str()),
                _ => None,
            })
            .collect()
    };
    if text(a) != text(b) {
        return false;
    }
    let (mut x, mut y) = (a.els(), b.els());
    loop {
        match (x.next(), y.next()) {
            (None, None) => return true,
            (Some(p), Some(q)) if same(p, q, depth + 1) => {}
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordcraft_doc::THEME_COLORS;

    fn parse(s: &str) -> El {
        xml::parse(s.as_bytes()).unwrap()
    }

    fn names(e: &El) -> Vec<&str> {
        e.els().map(|c| c.name.as_str()).collect()
    }

    #[test]
    fn every_layout_reads_back_and_draws_inside_its_frame() {
        for layout in SmartArtLayout::ALL {
            for colors in [SmartArtColors::Accent3, SmartArtColors::Colorful] {
                let mut spec = SmartArtSpec::sample(layout);
                spec.colors = colors;
                spec.items.push(SmartArtItem::new("Under <last> & more", 1));
                let spec = spec.sanitized();
                assert_eq!(spec_of(&parse(&data_xml(&spec, "rId9"))).as_ref(), Some(&spec), "{layout:?}");
                let items = smart_art_items(&spec, &THEME_COLORS, 400.0, 250.0);
                assert!(items.iter().any(|i| matches!(i, GraphicItem::Text { .. })), "{layout:?} has text");
                for it in &items {
                    if let GraphicItem::Shape { rect, .. } = it {
                        assert!(
                            rect[0] >= -0.5 && rect[1] >= -0.5 && rect[0] + rect[2] <= 400.5 && rect[1] + rect[3] <= 250.5,
                            "{layout:?}: {rect:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_diagram_with_more_than_the_model_stays_read_only() {
        let spec = SmartArtSpec::sample(SmartArtLayout::Process);
        let x = data_xml(&spec, "rId4");
        assert_eq!(spec_of(&parse(&x.replace("<dgm:bg/>", "<dgm:bg><a:noFill/></dgm:bg>"))), None);
        assert_eq!(spec_of(&parse(&x.replace("layout/process1", "layout/process9"))), None);
        // A run split in two (as another program might save it) isn't ours either.
        let split = x.replace(
            "<a:r><a:rPr lang=\"en-US\"/><a:t>Plan</a:t></a:r>",
            "<a:r><a:rPr lang=\"en-US\"/><a:t>Pl</a:t></a:r><a:r><a:rPr lang=\"en-US\"/><a:t>an</a:t></a:r>",
        );
        assert_ne!(split, x);
        assert_eq!(spec_of(&parse(&split)), None);
    }

    #[test]
    fn parts_come_in_schema_order() {
        let spec = SmartArtSpec::sample(SmartArtLayout::Hierarchy);
        let data = parse(&data_xml(&spec, "rId1"));
        assert_eq!(names(&data), ["dgm:ptLst", "dgm:cxnLst", "dgm:bg", "dgm:whole", "dgm:extLst"]);
        let pt = data.child("dgm:ptLst").unwrap().els().nth(1).unwrap();
        assert_eq!(names(pt), ["dgm:prSet", "dgm:spPr", "dgm:t"]);
        // Each item's connection names its parent: Design (item 1) under Lead (item 0).
        let cx = data.child("dgm:cxnLst").unwrap().els().nth(1).unwrap();
        assert_eq!(cx.attr("srcId"), Some(model_id(1).as_str()));
        let drawing = parse(&drawing_xml(&spec, 300.0, 200.0));
        let tree = drawing.child("dsp:spTree").unwrap();
        assert_eq!(names(tree).first(), Some(&"dsp:nvGrpSpPr"));
        let sp = tree.children("dsp:sp").last().unwrap();
        assert_eq!(names(sp), ["dsp:nvSpPr", "dsp:spPr", "dsp:style", "dsp:txBody", "dsp:txXfrm"]);
        assert_eq!(names(sp.child("dsp:spPr").unwrap()), ["a:xfrm", "a:prstGeom", "a:solidFill", "a:ln"]);
        for part in [layout_xml(SmartArtLayout::Radial), colors_xml(SmartArtColors::Colorful), quick_style_xml()] {
            let el = parse(&part);
            assert_eq!(&names(&el)[..3], ["dgm:title", "dgm:desc", "dgm:catLst"]);
        }
    }

    #[test]
    fn hostile_sizes_and_many_items_stay_finite() {
        let spec =
            SmartArtSpec { items: (0..MAX_ITEMS).map(|i| SmartArtItem::new("word ".repeat(50), (i % 9) as u8)).collect(), ..Default::default() }
                .sanitized();
        for layout in SmartArtLayout::ALL {
            let spec = SmartArtSpec { layout, ..spec.clone() };
            for (w, h) in [(f32::NAN, 10.0), (0.0, 0.0), (1.0e30, 5.0), (300.0, 200.0)] {
                for it in smart_art_items(&spec, &THEME_COLORS, w, h) {
                    if let GraphicItem::Shape { rect, .. } | GraphicItem::Text { rect, .. } = it {
                        assert!(rect.iter().all(|v| v.is_finite()), "{layout:?} {w} {h}");
                    }
                }
            }
        }
    }
}
