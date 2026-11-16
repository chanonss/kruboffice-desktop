"""Generate src/tabler.rs from the Tabler Icons outline SVGs and tabler-map.tsv.

Tabler Icons (https://tabler.io/icons) is MIT licensed, Copyright (c) 2020-2026 Pawel Kuna;
see ../LICENSE-tabler.txt. Get the SVGs from the npm package @tabler/icons at the version below
(package/icons/outline/*.svg), then, from crates/krub-icons:

    python3 -I tools/tabler_rs.py <path to package/icons/outline> tools/tabler-map.tsv src/tabler.rs
    rustfmt --edition 2024 src/tabler.rs

tabler-map.tsv lists every KrubOffice icon name that is drawn from Tabler and the Tabler icon used;
names not listed keep their KrubOffice drawing. Keep this crate identical in all three repositories.
"""
import json, math, re, sys

SVG_DIR, MAP, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
VERSION = "3.50.0"

ours = {}
for line in open(MAP, encoding="utf-8"):
    line = line.strip()
    if line and not line.startswith("#"):
        name, pick = line.split("\t")
        ours[name] = pick
picked = sorted(set(ours.values()))

TOK = re.compile(r"[MmLlHhVvCcSsQqTtAaZz]|-?(?:\d+\.?\d*|\.\d+)(?:e-?\d+)?")


def tokens(d):
    out = []
    for t in TOK.findall(d):
        out.append(t)
    return out


def arc_to_cubics(x1, y1, rx, ry, phi, fa, fs, x2, y2):
    if (x1, y1) == (x2, y2):
        return []
    if rx == 0 or ry == 0:
        return [("L", x2, y2)]
    rx, ry = abs(rx), abs(ry)
    phi = math.radians(phi)
    cp, sp = math.cos(phi), math.sin(phi)
    dx, dy = (x1 - x2) / 2, (y1 - y2) / 2
    x1p, y1p = cp * dx + sp * dy, -sp * dx + cp * dy
    lam = x1p * x1p / (rx * rx) + y1p * y1p / (ry * ry)
    if lam > 1:
        s = math.sqrt(lam)
        rx, ry = rx * s, ry * s
    num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p
    den = rx * rx * y1p * y1p + ry * ry * x1p * x1p
    coef = math.sqrt(max(0.0, num / den)) if den else 0.0
    if fa == fs:
        coef = -coef
    cxp, cyp = coef * rx * y1p / ry, -coef * ry * x1p / rx
    cx = cp * cxp - sp * cyp + (x1 + x2) / 2
    cy = sp * cxp + cp * cyp + (y1 + y2) / 2

    def ang(ux, uy, vx, vy):
        a = math.atan2(ux * vy - uy * vx, ux * vx + uy * vy)
        return a

    t1 = ang(1, 0, (x1p - cxp) / rx, (y1p - cyp) / ry)
    dt = ang((x1p - cxp) / rx, (y1p - cyp) / ry, (-x1p - cxp) / rx, (-y1p - cyp) / ry)
    if not fs and dt > 0:
        dt -= 2 * math.pi
    elif fs and dt < 0:
        dt += 2 * math.pi
    n = max(1, math.ceil(abs(dt) / (math.pi / 2) - 1e-9))
    step = dt / n
    k = 4 / 3 * math.tan(step / 4)
    segs = []

    def pt(t):
        x, y = rx * math.cos(t), ry * math.sin(t)
        return cp * x - sp * y + cx, sp * x + cp * y + cy

    def dpt(t):
        x, y = -rx * math.sin(t), ry * math.cos(t)
        return cp * x - sp * y, sp * x + cp * y

    t = t1
    for _ in range(n):
        a, b = pt(t), pt(t + step)
        da, db = dpt(t), dpt(t + step)
        segs.append(("C", a[0] + k * da[0], a[1] + k * da[1], b[0] - k * db[0], b[1] - k * db[1], b[0], b[1]))
        t += step
    # snap the end exactly
    last = segs[-1]
    segs[-1] = last[:5] + (x2, y2)
    return segs


def parse(d):
    """SVG path -> list of subpaths, each [('M',x,y), ('L',x,y) | ('C',...), ..., ('Z',)]."""
    t = tokens(d)
    i = 0
    cmd = None
    x = y = sx = sy = 0.0
    lc = None  # last cubic control point for S
    out = []
    cur = None

    def num():
        nonlocal i
        v = float(t[i])
        i += 1
        return v

    def flag():
        nonlocal i
        tok = t[i]
        # compact flags like "01" can arrive glued; take the first char
        if len(tok) > 1 and tok[0] in "01" and tok[1] in "01.":
            t[i] = tok[1:]
            return int(tok[0])
        i += 1
        return int(float(tok))

    while i < len(t):
        if re.match(r"[A-Za-z]", t[i]):
            cmd = t[i]
            i += 1
            if cmd in "Zz":
                if cur is not None:
                    cur.append(("Z",))
                x, y = sx, sy
                lc = None
                continue
        rel = cmd.islower()
        c = cmd.upper()
        if c == "M":
            nx, ny = num(), num()
            if rel:
                nx, ny = x + nx, y + ny
            cur = [("M", nx, ny)]
            out.append(cur)
            x, y, sx, sy = nx, ny, nx, ny
            cmd = "l" if rel else "L"
            lc = None
        elif c == "L":
            nx, ny = num(), num()
            if rel:
                nx, ny = x + nx, y + ny
            cur.append(("L", nx, ny))
            x, y = nx, ny
            lc = None
        elif c == "H":
            nx = num()
            nx = x + nx if rel else nx
            cur.append(("L", nx, y))
            x = nx
            lc = None
        elif c == "V":
            ny = num()
            ny = y + ny if rel else ny
            cur.append(("L", x, ny))
            y = ny
            lc = None
        elif c == "C":
            a = [num() for _ in range(6)]
            if rel:
                a = [a[0] + x, a[1] + y, a[2] + x, a[3] + y, a[4] + x, a[5] + y]
            cur.append(("C", *a))
            lc = (a[2], a[3])
            x, y = a[4], a[5]
        elif c == "S":
            a = [num() for _ in range(4)]
            if rel:
                a = [a[0] + x, a[1] + y, a[2] + x, a[3] + y]
            c1 = (2 * x - lc[0], 2 * y - lc[1]) if lc else (x, y)
            cur.append(("C", c1[0], c1[1], *a))
            lc = (a[0], a[1])
            x, y = a[2], a[3]
        elif c == "Q":
            a = [num() for _ in range(4)]
            if rel:
                a = [a[0] + x, a[1] + y, a[2] + x, a[3] + y]
            q = (a[0], a[1])
            cur.append(("C", x + 2 / 3 * (q[0] - x), y + 2 / 3 * (q[1] - y), a[2] + 2 / 3 * (q[0] - a[2]), a[3] + 2 / 3 * (q[1] - a[3]), a[2], a[3]))
            x, y = a[2], a[3]
            lc = None
        elif c == "A":
            rx, ry, phi = num(), num(), num()
            fa, fs = flag(), flag()
            nx, ny = num(), num()
            if rel:
                nx, ny = x + nx, y + ny
            cur.extend(arc_to_cubics(x, y, rx, ry, phi, fa, fs, nx, ny))
            x, y = nx, ny
            lc = None
        else:
            raise ValueError(f"unsupported command {cmd} in {d}")
    return out


def flatten(sub, n=12):
    pts = []
    x = y = 0.0
    for s in sub:
        if s[0] == "M" or s[0] == "L":
            x, y = s[1], s[2]
            pts.append((x, y))
        elif s[0] == "C":
            for k in range(1, n + 1):
                u = k / n
                v = 1 - u
                px = v**3 * x + 3 * v * v * u * s[1] + 3 * v * u * u * s[3] + u**3 * s[5]
                py = v**3 * y + 3 * v * v * u * s[2] + 3 * v * u * u * s[4] + u**3 * s[6]
                pts.append((px, py))
            x, y = s[5], s[6]
    # drop duplicates
    clean = []
    for p in pts:
        if not clean or abs(p[0] - clean[-1][0]) > 1e-6 or abs(p[1] - clean[-1][1]) > 1e-6:
            clean.append(p)
    if len(clean) > 2 and abs(clean[0][0] - clean[-1][0]) < 1e-6 and abs(clean[0][1] - clean[-1][1]) < 1e-6:
        clean.pop()
    return clean


def as_circle(sub):
    """A closed outline made only of curves, all points equally far from its centre: a circle."""
    if len(sub) < 3 or sub[0][0] != "M" or any(s[0] != "C" for s in sub[1:]):
        return None
    if abs(sub[0][1] - sub[-1][5]) > 1e-3 or abs(sub[0][2] - sub[-1][6]) > 1e-3:
        return None
    pts = flatten(sub, 24)
    cx = sum(p[0] for p in pts) / len(pts)
    cy = sum(p[1] for p in pts) / len(pts)
    ds = [math.hypot(p[0] - cx, p[1] - cy) for p in pts]
    rad = sum(ds) / len(ds)
    if rad <= 0 or max(abs(d - rad) for d in ds) > 0.02 * max(rad, 0.5):
        return None
    return [("O", cx, cy, rad)]


def triangulate(poly):
    """Ear clipping for a simple polygon."""
    area = sum(poly[i][0] * poly[(i + 1) % len(poly)][1] - poly[(i + 1) % len(poly)][0] * poly[i][1] for i in range(len(poly)))
    idx = list(range(len(poly)))
    if area < 0:
        idx.reverse()
    tris = []

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    guard = 0
    while len(idx) > 3 and guard < 10000:
        guard += 1
        for k in range(len(idx)):
            a, b, c = poly[idx[k - 1]], poly[idx[k]], poly[idx[(k + 1) % len(idx)]]
            if cross(a, b, c) <= 1e-9:
                continue
            if any(cross(a, b, poly[j]) >= 0 and cross(b, c, poly[j]) >= 0 and cross(c, a, poly[j]) >= 0 for j in idx if poly[j] not in (a, b, c)):
                continue
            tris.append((a, b, c))
            idx.pop(k)
            break
        else:
            break
    if len(idx) == 3:
        tris.append(tuple(poly[j] for j in idx))
    return tris


def f(v):
    v = round(v, 3)
    if v == 0:
        v = 0.0
    s = repr(float(v))
    return s


def const_name(n):
    return "T_" + re.sub(r"[^A-Za-z0-9]", "_", n).upper()


glyphs = []
for name in picked:
    svg = open(f"{SVG_DIR}/{name}.svg", encoding="utf-8").read()
    strokes, fills = [], []
    for el in re.findall(r"<path\b[^>]*>", svg):
        if 'stroke="none"' in el:
            continue
        d = re.search(r'\bd="([^"]*)"', el).group(1)
        subs = parse(d)
        if 'fill="currentColor"' in el:
            for sub in subs:
                for tri in triangulate(flatten(sub)):
                    fills.append(tri)
            # the outline is stroked too in Tabler (stroke inherits)
        strokes.extend(as_circle(sub) or sub for sub in subs)
    glyphs.append((name, strokes, fills))

L = []
w = L.append
w("//! Interface icons adapted from Tabler Icons " + VERSION + " (https://tabler.io/icons), outline style.")
w("//! MIT License, Copyright (c) 2020-2026 Paweł Kuna; the full text is `LICENSE-tabler.txt` in this crate,")
w("//! and it must travel with every copy of this file. Converted from the SVG path data: arcs and")
w("//! quadratic curves became cubic Béziers, and the three filled shapes were pre-triangulated.")
w("//! Generated by `tools/tabler_rs.py` from `tools/tabler-map.tsv`: regenerate rather than edit by hand.")
w("")
w("use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};")
w("")
w("/// One drawing step on Tabler's 24-unit grid.")
w("#[derive(Clone, Copy)]")
w("enum Seg {")
w("    /// Start a new outline.")
w("    M(f32, f32),")
w("    /// A whole circle: centre and radius.")
w("    O(f32, f32, f32),")
w("    /// Straight line.")
w("    L(f32, f32),")
w("    /// Cubic curve: two control points, then the end point.")
w("    C(f32, f32, f32, f32, f32, f32),")
if any(s[0] == "Z" for _, st, _ in glyphs for sub in st for s in sub):
    w("    /// Close the outline.")
    w("    Z,")
w("}")
w("")
w("use Seg::{C, L, M, O" + (", Z" if any(s[0] == "Z" for _, st, _ in glyphs for sub in st for s in sub) else "") + "};")
w("")
w("struct Glyph {")
w("    strokes: &'static [Seg],")
w("    fills: &'static [[f32; 6]],")
w("}")
w("")
for name, strokes, fills in glyphs:
    segs = []
    for sub in strokes:
        for s in sub:
            if s[0] == "Z":
                segs.append("Z")
            else:
                segs.append(s[0] + "(" + ", ".join(f(v) for v in s[1:]) + ")")
    fl = ["[" + ", ".join(f(c) for p in tri for c in p) + "]" for tri in fills]
    w(f"/// Tabler `{name}`.")
    w(f"const {const_name(name)}: Glyph = Glyph {{ strokes: &[{', '.join(segs)}], fills: &[{', '.join(fl)}] }};")
w("")
w("/// The Tabler drawing chosen for a KrubOffice icon name (any catalog spelling or alias).")
w("/// Names that keep their KrubOffice drawing are absent.")
w("pub fn tabler_name(name: &str) -> Option<&'static str> {")
w("    Some(match name {")
by_pick = {}
for n, p in sorted(ours.items()):
    by_pick.setdefault(p, []).append(n)
for p in sorted(by_pick):
    w("        " + " | ".join(json.dumps(n) for n in sorted(by_pick[p])) + f" => {json.dumps(p)},")
w("        _ => return None,")
w("    })")
w("}")
w("")
w("fn glyph(tabler: &str) -> Option<&'static Glyph> {")
w("    Some(match tabler {")
for name, _, _ in glyphs:
    w(f"        {json.dumps(name)} => &{const_name(name)},")
w("        _ => return None,")
w("    })")
w("}")
w("")
w(r'''/// Draw the Tabler icon chosen for `name` into the largest square centred in `r`, in `ink`
/// (35% of it when `disabled`). `false` when `name` keeps its KrubOffice drawing.
pub fn paint(p: &Painter, r: Rect, name: &str, ink: Color32, disabled: bool) -> bool {
    let Some(glyph) = tabler_name(name).and_then(glyph) else {
        return false;
    };
    let size = r.width().min(r.height());
    if !size.is_finite() || size <= 0.5 || !r.center().is_finite() {
        return false;
    }
    let ink = if disabled { ink.gamma_multiply(0.35) } else { ink };
    let scale = size / 24.0;
    let origin = r.center() - vec2(size, size) * 0.5;
    let at = |x: f32, y: f32| origin + vec2(x, y) * scale;
    // KrubOffice's line weight on the 24-unit grid, never thinner than one hairline.
    let stroke = Stroke::new((1.75 * scale).max(0.9), ink);
    // Curves are split finely enough to look smooth at the drawn size on this screen.
    let steps = (size * p.ctx().pixels_per_point() / 6.0).ceil().clamp(3.0, 16.0) as usize;
    let mut outline: Vec<Pos2> = Vec::new();
    let half = stroke.width * 0.5;
    // Round ends and dots: egui's filled circles, as the KrubOffice drawings use for their caps.
    let cap = |end: Pos2| {
        p.circle_filled(end, half, ink);
    };
    let flush = |outline: &mut Vec<Pos2>, closed: bool| {
        let length: f32 = outline.windows(2).map(|pair| if let [a, b] = pair { a.distance(*b) } else { 0.0 }).sum();
        let first = outline.first().copied();
        let second = outline.get(1).copied();
        let last = outline.last().copied();
        let before_last = outline.iter().rev().nth(1).copied();
        match (first, second, last, before_last) {
            (Some(centre), None, _, _) => cap(centre),
            (Some(centre), Some(_), _, _) if length < 0.05 * scale => cap(centre),
            (Some(_), Some(_), _, _) if closed && outline.len() > 2 => {
                p.add(Shape::closed_line(std::mem::take(outline), stroke));
            }
            (Some(start), Some(_), Some(end), Some(_)) => {
                cap(start);
                cap(end);
                p.add(Shape::line(std::mem::take(outline), stroke));
            }
            _ => {}
        }
        outline.clear();
    };
    let mut pen = pos2(0.0, 0.0);
    for seg in glyph.strokes {
        match *seg {
            O(x, y, radius) => {
                flush(&mut outline, false);
                p.circle_stroke(at(x, y), radius * scale, stroke);
            }
            M(x, y) => {
                flush(&mut outline, false);
                pen = pos2(x, y);
                outline.push(at(x, y));
            }
            L(x, y) => {
                pen = pos2(x, y);
                outline.push(at(x, y));
            }
            C(x1, y1, x2, y2, x, y) => {
                for k in 1..=steps {
                    let u = k as f32 / steps as f32;
                    let v = 1.0 - u;
                    let px = v * v * v * pen.x + 3.0 * v * v * u * x1 + 3.0 * v * u * u * x2 + u * u * u * x;
                    let py = v * v * v * pen.y + 3.0 * v * v * u * y1 + 3.0 * v * u * u * y2 + u * u * u * y;
                    outline.push(at(px, py));
                }
                pen = pos2(x, y);
            }
@@ZARM@@        }
    }
    flush(&mut outline, false);
    if !glyph.fills.is_empty() {
        let mut mesh = egui::Mesh::default();
        for &[ax, ay, bx, by, cx, cy] in glyph.fills {
            let base = mesh.vertices.len() as u32;
            for point in [at(ax, ay), at(bx, by), at(cx, cy)] {
                mesh.colored_vertex(point, ink);
            }
            mesh.add_triangle(base, base + 1, base + 2);
        }
        p.add(Shape::mesh(mesh));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mapped_name_has_a_drawing() {
        for name in crate::catalog() {
            if let Some(tabler) = tabler_name(&name) {
                assert!(glyph(tabler).is_some(), "{name} -> {tabler}");
            }
        }
    }

    #[test]
    fn kept_icons_stay_krub_office_drawings() {
        for name in ["thai-distribution", "thaiDistribute", "thaiDigits", "fn-book-clock", "fn-book-theta"] {
            assert!(tabler_name(name).is_none(), "{name}");
        }
    }
}
''')
HAS_Z = any(s[0] == "Z" for _, st, _ in glyphs for sub in st for s in sub)
ZARM = '            Z => {\n                if outline.len() > 2 && outline.first().zip(outline.last()).is_some_and(|(a, b)| a.distance(*b) < 0.01) {\n                    outline.pop();\n                }\n                flush(&mut outline, true);\n            }\n'
open(OUT, "w", encoding="utf-8").write("\n".join(L).replace("@@ZARM@@", ZARM if HAS_Z else ""))
print("glyphs", len(glyphs), "names", len(ours), "fills", sum(len(g[2]) for g in glyphs))
