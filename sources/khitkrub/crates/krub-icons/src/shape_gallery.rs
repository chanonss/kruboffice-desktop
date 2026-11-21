//! The Insert › Shapes menu, the same in every KrubOffice app (owner, 2026-11-19, from the
//! approved draft): the shapes used last on top, then the 182 standard DrawingML shapes of the
//! shared library (`craft_drawingml::preset`) in their nine groups (Lines, Rectangles, Basic
//! Shapes, Block Arrows, Equation Shapes, Flowchart, Stars and Banners, Callouts, Action Buttons),
//! then the app's own extras. Each icon is the real shape drawn small, outlined, with the pale gold
//! surface of the other KrubOffice icons; its name is its tooltip and accessible name.
//!
//! The apps run different egui versions, so this uses only what they share: each slot is a
//! selectable button whose (invisible) text is the shape's name, with the shape painted over it.

use craft_drawingml::preset::{self, CATALOG, Category, FillMode};
use egui::{Color32, Pos2, Rect, Response, RichText, Shape, Stroke, Ui, epaint::Mesh, pos2, vec2};

/// The pale gold surface.
const SURFACE: Color32 = Color32::from_rgb(0xFA, 0xC7, 0x75);
/// Shapes kept in the Recently Used row.
pub const RECENT: usize = 8;
/// Points a filled outline is cut into at most (larger outlines are only stroked).
const MAX_FILL_POINTS: usize = 400;

/// What an app offers besides the standard shapes.
pub struct Options<'a> {
    /// Action Buttons (they act in a slide show: PhutKrub only).
    pub action_buttons: bool,
    /// Drawing tools at the end of the Lines row: (id, translated name), with icons drawn by
    /// [`paint_tool`].
    pub line_tools: &'a [(&'a str, &'a str)],
    /// The app's own items after the groups: (id returned when picked, translated label).
    pub extras: &'a [(&'a str, &'a str)],
}

fn recent_id() -> egui::Id {
    crate::pane_ui::id("krub_shape_recent")
}

/// The shapes used last, newest first.
pub fn recent(ctx: &egui::Context) -> Vec<String> {
    ctx.data_mut(|d| d.get_persisted::<Vec<String>>(recent_id())).unwrap_or_default()
}

/// Puts `name` first in the Recently Used row.
pub fn remember(ctx: &egui::Context, name: &str) {
    if preset::info(name).is_none() {
        return;
    }
    let mut v = recent(ctx);
    v.retain(|n| n != name);
    v.insert(0, name.to_string());
    v.truncate(RECENT);
    ctx.data_mut(|d| d.insert_persisted(recent_id(), v));
}

/// The English name of a group, for the apps' translation tables.
pub fn group_name(c: Category) -> &'static str {
    c.label()
}

/// Every name the menu shows (group names and the Recently Used heading), for the apps' tables.
pub fn names() -> Vec<&'static str> {
    let mut v = vec!["Recently Used Shapes"];
    v.extend(Category::ALL.iter().map(|c| c.label()));
    v
}

/// The menu: returns the picked shape's DrawingML name (`rect`, `rightArrow`…) or an extra's id.
/// `tr` translates the headings and the shapes' names.
pub fn show(ui: &mut Ui, tr: &dyn Fn(&str) -> String, opts: &Options) -> Option<String> {
    let mut picked = None;
    // A fixed width: the rows wrap inside it (a maximum alone lets them run past the edge).
    ui.set_width(372.0);
    let row = |ui: &mut Ui, names: &mut dyn Iterator<Item = &'static str>, picked: &mut Option<String>| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
            for name in names {
                let label = preset::info(name).map_or(name, |i| i.label);
                if slot(ui, name, &tr(label)).clicked() {
                    *picked = Some(name.to_string());
                }
            }
        });
    };
    let heading = |ui: &mut Ui, s: &str| {
        ui.add_space(4.0);
        ui.label(RichText::new(tr(s)).small().strong());
    };
    let recent: Vec<&'static str> = recent(ui.ctx()).iter().filter_map(|n| preset::info(n).map(|i| i.name)).collect();
    if !recent.is_empty() {
        heading(ui, "Recently Used Shapes");
        row(ui, &mut recent.into_iter(), &mut picked);
    }
    for c in Category::ALL {
        if c == Category::ActionButtons && !opts.action_buttons {
            continue;
        }
        heading(ui, c.label());
        if c == Category::Lines && !opts.line_tools.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                for name in CATALOG.iter().filter(|i| i.category == c).map(|i| i.name) {
                    let label = preset::info(name).map_or(name, |i| i.label);
                    if slot(ui, name, &tr(label)).clicked() {
                        picked = Some(name.to_string());
                    }
                }
                for (id, label) in opts.line_tools {
                    if tool_slot(ui, id, label).clicked() {
                        picked = Some(id.to_string());
                    }
                }
            });
            continue;
        }
        row(ui, &mut CATALOG.iter().filter(|i| i.category == c).map(|i| i.name), &mut picked);
    }
    if !opts.extras.is_empty() {
        ui.separator();
        for (id, label) in opts.extras {
            if ui.button(*label).clicked() {
                picked = Some(id.to_string());
            }
        }
    }
    if let Some(name) = &picked {
        remember(ui.ctx(), name);
    }
    picked
}

/// One shape's slot: a 26-point button named `label`, the shape drawn on it.
fn slot(ui: &mut Ui, name: &str, label: &str) -> Response {
    let resp = ui.add_sized(vec2(26.0, 26.0), egui::Button::selectable(false, RichText::new(label).color(Color32::TRANSPARENT)).truncate());
    let ink = ui.style().interact(&resp).text_color();
    paint(ui.painter(), resp.rect.shrink(4.0), name, ink);
    resp.on_hover_text(label)
}

/// A drawing tool's slot, like [`slot`].
fn tool_slot(ui: &mut Ui, id: &str, label: &str) -> Response {
    let resp = ui.add_sized(vec2(26.0, 26.0), egui::Button::selectable(false, RichText::new(label).color(Color32::TRANSPARENT)).truncate());
    let ink = ui.style().interact(&resp).text_color();
    paint_tool(ui.painter(), resp.rect.shrink(5.0), id, ink);
    resp.on_hover_text(label)
}

/// Draws a drawing tool's icon (`curve`, `freeform`, `scribble`, and the arrowed `lineArrow`,
/// `elbowArrow`, `curvedArrow`); `false` for another id.
pub fn paint_tool(p: &egui::Painter, r: Rect, id: &str, ink: Color32) -> bool {
    let at = |x: f32, y: f32| pos2(r.left() + x * r.width(), r.top() + y * r.height());
    let stroke = Stroke::new(1.2, ink);
    let pts: Vec<Pos2> = match id {
        "curve" => (0..=24).map(|i| i as f32 / 24.0).map(|t| at(t, 0.5 - 0.4 * (t * std::f32::consts::TAU * 0.75).sin())).collect(),
        "freeform" => {
            let v = vec![at(0.05, 0.9), at(0.3, 0.2), at(0.55, 0.6), at(0.95, 0.1), at(0.8, 0.9), at(0.05, 0.9)];
            p.add(Shape::mesh(fill_mesh(&v, SURFACE)));
            v
        }
        "scribble" => (0..=40).map(|i| i as f32 / 40.0).map(|t| at(t, 0.5 + 0.35 * (t * 23.0).sin() * (1.0 - t * 0.4))).collect(),
        // Lines and connectors with an arrowhead at their end.
        "lineArrow" => vec![at(0.05, 0.95), at(0.95, 0.05)],
        "elbowArrow" => vec![at(0.05, 0.85), at(0.5, 0.85), at(0.5, 0.15), at(0.95, 0.15)],
        "curvedArrow" => (0..=20).map(|i| i as f32 / 20.0).map(|t| at(0.05 + 0.9 * t, 0.85 - 0.7 * (3.0 * t * t - 2.0 * t * t * t))).collect(),
        _ => return false,
    };
    if id.ends_with("Arrow")
        && let [.., a, b] = pts[..]
    {
        let d = b - a;
        let len = d.length().max(1e-3);
        let (u, n) = (d / len, vec2(-d.y, d.x) / len);
        let size = r.width() * 0.3;
        let head = vec![b, b - u * size + n * size * 0.5, b - u * size - n * size * 0.5];
        p.add(Shape::convex_polygon(head, ink, Stroke::NONE));
    }
    p.add(Shape::line(pts, stroke));
    true
}

/// Draws standard shape `name` in `r` (its default proportions: square, or a short line); `false`
/// when the library doesn't know it.
pub fn paint(p: &egui::Painter, r: Rect, name: &str, ink: Color32) -> bool {
    let Some(info) = preset::info(name) else { return false };
    let (w, h) = if preset::is_line_like(name) { (20.0, 14.0) } else { (20.0, 20.0) };
    let Some(geo) = preset::build(name, w, h, info.defaults) else { return false };
    let k = (r.width() / w as f32).min(r.height() / h as f32);
    let origin = r.center() - vec2(w as f32 * k, h as f32 * k) / 2.0;
    let to = |pt: kurbo::Point| pos2(origin.x + pt.x as f32 * k, origin.y + pt.y as f32 * k);
    let stroke = Stroke::new((k * 1.1).clamp(0.8, 1.4), ink);
    for sp in &geo.paths {
        for (pts, closed) in polylines(&sp.path) {
            let pts: Vec<Pos2> = pts.into_iter().map(to).collect();
            if sp.fill != FillMode::None && closed && pts.len() >= 3 && pts.len() <= MAX_FILL_POINTS {
                let tone = match sp.fill {
                    FillMode::Darken | FillMode::DarkenLess => SURFACE.gamma_multiply(0.85),
                    _ => SURFACE,
                };
                p.add(Shape::mesh(fill_mesh(&pts, tone)));
            }
            if sp.stroke {
                if closed {
                    p.add(Shape::closed_line(pts, stroke));
                } else {
                    p.add(Shape::line(pts, stroke));
                }
            }
        }
    }
    true
}

/// The path flattened into polylines, each with whether it is closed (for apps drawing the
/// shared shapes with egui, as KhitKrub's sheet does).
pub fn polylines(path: &kurbo::BezPath) -> Vec<(Vec<kurbo::Point>, bool)> {
    let mut out: Vec<(Vec<kurbo::Point>, bool)> = Vec::new();
    kurbo::flatten(path.iter(), 0.25, |el| match el {
        kurbo::PathEl::MoveTo(p) => out.push((vec![p], false)),
        kurbo::PathEl::LineTo(p) => {
            if let Some(last) = out.last_mut() {
                last.0.push(p);
            }
        }
        kurbo::PathEl::ClosePath => {
            if let Some(last) = out.last_mut() {
                last.1 = true;
            }
        }
        _ => {}
    });
    out.retain(|(v, _)| v.len() >= 2);
    out
}

/// A simple polygon filled by ear clipping (egui fills only convex paths itself).
pub fn fill_mesh(pts: &[Pos2], color: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    // The last point repeats the first on a closed outline.
    let mut v: Vec<Pos2> = pts.to_vec();
    if v.len() > 1 && v.first() == v.last() {
        v.pop();
    }
    for p in &v {
        mesh.colored_vertex(*p, color);
    }
    let area: f32 = (0..v.len())
        .map(|i| {
            let (a, b) = (v[i], v[(i + 1) % v.len()]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    let ccw = area > 0.0;
    let mut idx: Vec<usize> = (0..v.len()).collect();
    let cross = |a: Pos2, b: Pos2, c: Pos2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let mut guard = 0;
    while idx.len() > 3 && guard < 10_000 {
        guard += 1;
        let n = idx.len();
        let mut clipped = false;
        for i in 0..n {
            let (ia, ib, ic) = (idx[(i + n - 1) % n], idx[i], idx[(i + 1) % n]);
            let (a, b, c) = (v[ia], v[ib], v[ic]);
            let turn = cross(a, b, c);
            // A corner on a straight run adds nothing: drop it, or no ear may be found.
            if turn.abs() < 1e-6 {
                idx.remove(i);
                clipped = true;
                break;
            }
            if (turn > 0.0) != ccw {
                continue;
            }
            let inside = idx.iter().any(|&j| {
                if j == ia || j == ib || j == ic {
                    return false;
                }
                let p = v[j];
                let (d1, d2, d3) = (cross(a, b, p), cross(b, c, p), cross(c, a, p));
                // A point on the triangle's edge blocks it too (a reflex corner touching it).
                if ccw { d1 >= 0.0 && d2 >= 0.0 && d3 >= 0.0 } else { d1 <= 0.0 && d2 <= 0.0 && d3 <= 0.0 }
            });
            if inside {
                continue;
            }
            mesh.add_triangle(ia as u32, ib as u32, ic as u32);
            idx.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if let [a, b, c] = idx[..] {
        mesh.add_triangle(a as u32, b as u32, c as u32);
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_shape_draws_and_recent_keeps_eight() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            for info in CATALOG {
                assert!(paint(ui.painter(), Rect::from_min_size(pos2(0.0, 0.0), vec2(18.0, 18.0)), info.name, Color32::BLACK), "{}", info.name);
            }
            assert!(!paint(ui.painter(), Rect::NOTHING, "nope", Color32::BLACK));
            let picked = show(
                ui,
                &|s| s.to_string(),
                &Options { action_buttons: false, line_tools: &[("curve", "Curve"), ("scribble", "Scribble")], extras: &[("canvas", "Canvas")] },
            );
            for id in ["curve", "freeform", "scribble", "lineArrow", "elbowArrow", "curvedArrow"] {
                assert!(paint_tool(ui.painter(), Rect::from_min_size(pos2(0.0, 0.0), vec2(16.0, 16.0)), id, Color32::BLACK));
            }
            assert!(picked.is_none());
        });
        out.textures_delta.clear();
        for info in CATALOG.iter().take(12) {
            remember(&ctx, info.name);
        }
        remember(&ctx, "nope");
        let r = recent(&ctx);
        assert_eq!(r.len(), RECENT);
        assert_eq!(r.first().map(String::as_str), CATALOG.get(11).map(|i| i.name), "newest first");
        assert!(CATALOG.len() >= 180, "{}", CATALOG.len());
    }

    #[test]
    fn concave_shapes_fill_inside_their_outline() {
        // An L: the ear clipper must not cover the notch.
        let l = [pos2(0.0, 0.0), pos2(2.0, 0.0), pos2(2.0, 1.0), pos2(1.0, 1.0), pos2(1.0, 2.0), pos2(0.0, 2.0)];
        let m = fill_mesh(&l, Color32::RED);
        assert_eq!(m.indices.len(), 12, "four triangles");
        let area: f32 = m
            .indices
            .chunks(3)
            .map(|t| {
                let (a, b, c) = (m.vertices[t[0] as usize].pos, m.vertices[t[1] as usize].pos, m.vertices[t[2] as usize].pos);
                ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs() / 2.0
            })
            .sum();
        assert!((area - 3.0).abs() < 1e-4, "{area}");
    }
}
