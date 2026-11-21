//! Insert › Diagram (the shared KrubOffice picker, with previews drawn in code here) and the
//! SmartArt Text Pane (the selected graphic's items as an editable outline). Each change runs one
//! command (`insert.smartArt`, `smartArt.items`, `smartArt.addShape`…), so agents get the same
//! result without the UI. The picker is shown through [`crate::dialogs_insert::InsertDialog`];
//! the pane through [`crate::panes`].

use egui::{Color32, Painter, Rect, Stroke, Ui, pos2, vec2};
use krub_icons::diagram_picker;
use serde::Serialize;
use serde_json::{Value, json};
use wordcraft_doc::props::Rgb;
use wordcraft_doc::smart_art::{MAX_ITEMS, SmartArtColors, SmartArtLayout, SmartArtSpec};

use crate::WordApp;
use crate::theme::{Tokens, c32};

/// Insert › Diagram: the category shown, the layout picked (its id) and the items typed. The
/// dialog itself is the shared [`krub_icons::diagram_picker`].
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartArtForm {
    pub category: String,
    pub layout: String,
    pub text: String,
}

impl SmartArtForm {
    pub fn new(_app: &WordApp) -> SmartArtForm {
        let s = diagram_picker::State::new(SmartArtLayout::default().id());
        SmartArtForm { category: s.category, layout: s.layout, text: s.text }
    }
}

/// The layouts KhianKrub offers, for the shared picker.
pub fn layouts() -> Vec<diagram_picker::Layout> {
    SmartArtLayout::ALL
        .into_iter()
        .map(|l| diagram_picker::Layout { id: l.id(), name: l.label(), category: l.category(), description: l.description() })
        .collect()
}

/// Theme accent `n` (1–6) of the document.
pub fn accent(theme: &[Rgb], n: usize) -> Color32 {
    theme.get(3 + n.clamp(1, 6)).map(|c| c32(*c)).unwrap_or(Color32::from_rgb(0x15, 0x60, 0x82))
}

/// A small picture of `layout` in `r`, in the colours `colors` takes from `theme`; connector
/// lines in `line`.
pub fn preview(p: &Painter, r: Rect, layout: SmartArtLayout, colors: SmartArtColors, theme: &[Rgb], line: Color32) {
    let r = r.shrink(r.width().min(r.height()) * 0.08);
    let s = r.width().min(r.height());
    let c = r.center();
    let col = |i: usize| accent(theme, colors.accent(i));
    // Unit-square coordinates (of the square in the middle of `r`) to screen.
    let at = |x: f32, y: f32| pos2(c.x + (x - 0.5) * s, c.y + (y - 0.5) * s);
    let block = |x0: f32, y0: f32, x1: f32, y1: f32, k: usize, round: f32| p.rect_filled(Rect::from_min_max(at(x0, y0), at(x1, y1)), round, col(k));
    let thin = Stroke::new(1.0, line);
    match layout {
        SmartArtLayout::BasicList => {
            for (k, (x, y)) in [(0.0, 0.18), (0.35, 0.18), (0.7, 0.18), (0.175, 0.55), (0.525, 0.55)].into_iter().enumerate() {
                block(x, y, x + 0.3, y + 0.27, k, 0.0);
            }
        }
        SmartArtLayout::Process => {
            for k in 0..3 {
                let x = k as f32 * 0.37;
                block(x, 0.36, x + 0.26, 0.64, k, 3.0);
                if k < 2 {
                    let ax = x + 0.285;
                    p.add(egui::Shape::convex_polygon(
                        vec![at(ax, 0.45), at(ax + 0.06, 0.5), at(ax, 0.55)],
                        col(k).gamma_multiply(0.6),
                        Stroke::NONE,
                    ));
                }
            }
        }
        SmartArtLayout::Cycle => {
            p.circle_stroke(c, s * 0.34, thin);
            for k in 0..5 {
                let a = (-90.0 + 72.0 * k as f32).to_radians();
                p.circle_filled(c + vec2(a.cos(), a.sin()) * s * 0.34, s * 0.12, col(k));
            }
        }
        SmartArtLayout::Hierarchy => {
            p.line_segment([at(0.5, 0.3), at(0.5, 0.45)], thin);
            p.line_segment([at(0.17, 0.45), at(0.83, 0.45)], thin);
            for x in [0.17, 0.5, 0.83] {
                p.line_segment([at(x, 0.45), at(x, 0.6)], thin);
            }
            block(0.33, 0.08, 0.67, 0.3, 0, 3.0);
            for (k, x) in [0.02, 0.35, 0.68].into_iter().enumerate() {
                block(x, 0.6, x + 0.3, 0.82, k + 1, 3.0);
            }
        }
        SmartArtLayout::Pyramid => {
            // Half the pyramid's width at height `y` (apex at 0.05, base at 0.95).
            let hw = |y: f32| 0.45 * (y - 0.05) / 0.9;
            for (k, (y0, y1)) in [(0.05, 0.37), (0.37, 0.66), (0.66, 0.95)].into_iter().enumerate() {
                let pts = if k == 0 {
                    vec![at(0.5, y0), at(0.5 + hw(y1), y1), at(0.5 - hw(y1), y1)]
                } else {
                    vec![at(0.5 - hw(y0), y0), at(0.5 + hw(y0), y0), at(0.5 + hw(y1), y1), at(0.5 - hw(y1), y1)]
                };
                p.add(egui::Shape::convex_polygon(pts, col(k), Stroke::new(1.0, Color32::WHITE)));
            }
        }
        SmartArtLayout::Radial => {
            let spokes: Vec<egui::Pos2> = (0..4)
                .map(|k| {
                    let a = (-90.0 + 90.0 * k as f32).to_radians();
                    c + vec2(a.cos(), a.sin()) * s * 0.36
                })
                .collect();
            for q in &spokes {
                p.line_segment([c, *q], thin);
            }
            p.circle_filled(c, s * 0.17, col(0));
            for (k, q) in spokes.into_iter().enumerate() {
                p.circle_filled(q, s * 0.12, col(k + 1));
            }
        }
        SmartArtLayout::Matrix => {
            for k in 0..4 {
                let (x, y) = ((k % 2) as f32 * 0.5 + 0.03, (k / 2) as f32 * 0.5 + 0.03);
                block(x, y, x + 0.44, y + 0.44, k, 3.0);
            }
        }
        SmartArtLayout::Venn => {
            for k in 0..3 {
                let a = (-90.0 + 120.0 * k as f32).to_radians();
                let q = c + vec2(a.cos(), a.sin()) * s * 0.15;
                p.circle_filled(q, s * 0.3, col(k).gamma_multiply(0.5));
                p.circle_stroke(q, s * 0.3, Stroke::new(1.0, col(k)));
            }
        }
        SmartArtLayout::VerticalBulletList => {
            for k in 0..3 {
                let y = 0.04 + k as f32 * 0.33;
                block(0.0, y, 1.0, y + 0.15, k, 3.0);
                p.rect_filled(Rect::from_min_max(at(0.06, y + 0.15), at(1.0, y + 0.27)), 0.0, col(k).gamma_multiply(0.2));
                p.circle_filled(at(0.13, y + 0.21), s * 0.015, col(k));
            }
        }
        SmartArtLayout::Chevron => {
            for k in 0..3 {
                let x = k as f32 * 0.33;
                // Two halves either side of the point's axis, each convex.
                for half in [[(0.0, 0.36), (0.26, 0.36), (0.34, 0.5), (0.08, 0.5)], [(0.08, 0.5), (0.34, 0.5), (0.26, 0.64), (0.0, 0.64)]] {
                    p.add(egui::Shape::convex_polygon(half.iter().map(|&(u, v)| at(x + u, v)).collect(), col(k), Stroke::NONE));
                }
            }
        }
        SmartArtLayout::OrgChart => {
            p.line_segment([at(0.5, 0.24), at(0.5, 0.32)], thin);
            p.line_segment([at(0.2, 0.32), at(0.8, 0.32)], thin);
            for x in [0.2, 0.5, 0.8] {
                p.line_segment([at(x, 0.32), at(x, 0.4)], thin);
            }
            p.line_segment([at(0.1, 0.54), at(0.1, 0.89)], thin);
            for y in [0.66, 0.85] {
                p.line_segment([at(0.1, y), at(0.14, y)], thin);
            }
            block(0.36, 0.06, 0.64, 0.24, 0, 0.0);
            for (k, x) in [0.06, 0.36, 0.66].into_iter().enumerate() {
                block(x, 0.4, x + 0.28, 0.54, k + 1, 0.0);
            }
            block(0.14, 0.6, 0.34, 0.72, 4, 0.0);
            block(0.14, 0.79, 0.34, 0.91, 5, 0.0);
        }
        SmartArtLayout::Timeline => {
            p.add(egui::Shape::convex_polygon(vec![at(0.9, 0.44), at(1.0, 0.5), at(0.9, 0.56)], col(0).gamma_multiply(0.5), Stroke::NONE));
            p.rect_filled(Rect::from_min_max(at(0.0, 0.475), at(0.9, 0.525)), 0.0, col(0).gamma_multiply(0.5));
            for k in 0..4 {
                let x = 0.1 + k as f32 * 0.24;
                p.circle_filled(at(x, 0.5), s * 0.045, Color32::WHITE);
                p.circle_stroke(at(x, 0.5), s * 0.045, Stroke::new(2.0, col(k)));
                let y = if k % 2 == 0 { 0.3 } else { 0.68 };
                p.rect_filled(Rect::from_min_max(at(x - 0.08, y), at(x + 0.08, y + 0.03)), 1.0, line);
            }
        }
        SmartArtLayout::Target => {
            let tc = at(0.32, 0.5);
            for (k, r) in [0.3, 0.2, 0.1].into_iter().enumerate() {
                p.circle_filled(tc, s * r, col(k).gamma_multiply(0.4 + 0.3 * k as f32));
            }
            for (k, y) in [0.25, 0.35, 0.45].into_iter().enumerate() {
                p.line_segment([at(0.32, y), at(0.7, y)], thin);
                p.rect_filled(Rect::from_min_max(at(0.72, y - 0.02), at(0.98, y + 0.02)), 1.0, col(k));
            }
        }
        SmartArtLayout::Funnel => {
            for (k, (x, y)) in [(0.38, 0.17), (0.62, 0.17), (0.5, 0.33)].into_iter().enumerate() {
                p.circle_filled(at(x, y), s * 0.1, col(k + 1));
            }
            let mouth = [at(0.1, 0.06), at(0.9, 0.06), at(0.58, 0.5), at(0.58, 0.6), at(0.42, 0.6), at(0.42, 0.5), at(0.1, 0.06)];
            p.add(egui::Shape::line(mouth.to_vec(), Stroke::new(1.5, col(0))));
            p.add(egui::Shape::convex_polygon(vec![at(0.45, 0.64), at(0.55, 0.64), at(0.5, 0.72)], col(0).gamma_multiply(0.6), Stroke::NONE));
            block(0.3, 0.76, 0.7, 0.94, 4, 3.0);
        }
        SmartArtLayout::Gear => {
            for (k, (x, y, r, teeth)) in [(0.36, 0.58, 0.3, 9), (0.78, 0.26, 0.2, 6), (0.8, 0.76, 0.15, 6)].into_iter().enumerate() {
                gear(p, at(x, y), s * r, teeth, col(k));
            }
        }
        SmartArtLayout::VerticalProcess => {
            for k in 0..3 {
                let y = k as f32 * 0.37;
                block(0.25, y, 0.75, y + 0.24, k, 3.0);
                if k < 2 {
                    let ay = y + 0.27;
                    p.add(egui::Shape::convex_polygon(
                        vec![at(0.45, ay), at(0.55, ay), at(0.5, ay + 0.07)],
                        col(k).gamma_multiply(0.6),
                        Stroke::NONE,
                    ));
                }
            }
        }
        SmartArtLayout::InvertedPyramid => {
            // Half the pyramid's width at height `y` (base at 0.05, apex at 0.95).
            let hw = |y: f32| 0.45 * (0.95 - y) / 0.9;
            for (k, (y0, y1)) in [(0.05, 0.34), (0.34, 0.63), (0.63, 0.95)].into_iter().enumerate() {
                let pts = if k == 2 {
                    vec![at(0.5 - hw(y0), y0), at(0.5 + hw(y0), y0), at(0.5, y1)]
                } else {
                    vec![at(0.5 - hw(y0), y0), at(0.5 + hw(y0), y0), at(0.5 + hw(y1), y1), at(0.5 - hw(y1), y1)]
                };
                p.add(egui::Shape::convex_polygon(pts, col(k), Stroke::new(1.0, Color32::WHITE)));
            }
        }
        SmartArtLayout::HorizontalHierarchy => {
            p.line_segment([at(0.3, 0.5), at(0.36, 0.5)], thin);
            p.line_segment([at(0.36, 0.25), at(0.36, 0.75)], thin);
            for y in [0.25, 0.75] {
                p.line_segment([at(0.36, y), at(0.42, y)], thin);
            }
            block(0.0, 0.4, 0.3, 0.6, 0, 3.0);
            block(0.42, 0.16, 0.7, 0.34, 1, 3.0);
            block(0.42, 0.66, 0.7, 0.84, 2, 3.0);
            p.line_segment([at(0.7, 0.25), at(0.74, 0.25)], thin);
            p.line_segment([at(0.74, 0.12), at(0.74, 0.38)], thin);
            for y in [0.12, 0.38] {
                p.line_segment([at(0.74, y), at(0.76, y)], thin);
            }
            block(0.76, 0.05, 1.0, 0.19, 3, 3.0);
            block(0.76, 0.31, 1.0, 0.45, 4, 3.0);
        }
        SmartArtLayout::LinearVenn => {
            for k in 0..4 {
                let q = at(0.14 + k as f32 * 0.24, 0.5);
                p.circle_filled(q, s * 0.15, col(k).gamma_multiply(0.45));
                p.circle_stroke(q, s * 0.15, Stroke::new(1.0, col(k)));
            }
        }
    }
}

/// A gear of `teeth` round `c`, its teeth reaching `r`.
fn gear(p: &Painter, c: egui::Pos2, r: f32, teeth: usize, fill: Color32) {
    p.circle_filled(c, r * 0.8, fill);
    let step = std::f32::consts::TAU / teeth as f32;
    for k in 0..teeth {
        let a = -std::f32::consts::FRAC_PI_2 + step * k as f32;
        let pt = |da: f32, rr: f32| c + vec2((a + da * step).cos(), (a + da * step).sin()) * r * rr;
        p.add(egui::Shape::convex_polygon(vec![pt(-0.3, 0.7), pt(-0.2, 1.0), pt(0.2, 1.0), pt(0.3, 0.7)], fill, Stroke::NONE));
    }
    p.circle_filled(c, r * 0.25, Color32::from_white_alpha(200));
}

/// A colour variant's name in the interface language.
pub fn colors_label(c: SmartArtColors) -> String {
    match c {
        SmartArtColors::Colorful => tl!("Colorful").to_string(),
        _ => crate::i18n::fmt(tl!("Accent {n}"), &[("n", &c.accent(0).to_string())]),
    }
}

/// Swatches of a colour variant in `r`: the accents its shapes take.
pub fn swatches(p: &Painter, r: Rect, colors: SmartArtColors, theme: &[Rgb]) {
    let n = if colors == SmartArtColors::Colorful { 5 } else { 3 };
    let w = r.width() / n as f32;
    for k in 0..n {
        let cell = Rect::from_min_size(pos2(r.left() + k as f32 * w, r.top()), vec2(w - 2.0, r.height()));
        p.rect_filled(cell, 2.0, accent(theme, colors.accent(k)));
    }
}

/// The picker; returns true to close.
pub fn picker_ui(app: &mut WordApp, ui: &mut Ui, f: &mut SmartArtForm) -> bool {
    let t = Tokens::get(ui.ctx());
    let theme = app.session.doc.settings.theme_colors.clone();
    let layouts = layouts();
    let mut state = diagram_picker::State { category: f.category.clone(), layout: f.layout.clone(), text: f.text.clone() };
    // Keep the action buttons in the window, even a short one.
    let max_height = (ui.ctx().content_rect().height() - 190.0).max(100.0);
    let outcome = diagram_picker::show(ui, &|s| tl!(s).to_string(), &layouts, &mut state, max_height, &mut |p, r, id| {
        if let Some(layout) = SmartArtLayout::ALL.into_iter().find(|l| l.id() == id) {
            preview(p, r, layout, SmartArtColors::default(), &theme, t.text_dim);
        }
    });
    (f.category, f.layout, f.text) = (state.category.clone(), state.layout.clone(), state.text.clone());
    match outcome {
        diagram_picker::Outcome::Open => false,
        diagram_picker::Outcome::Cancel => true,
        diagram_picker::Outcome::Insert => app.run("insert.smartArt", insert_params(&state)).is_ok(),
    }
}

/// The `insert.smartArt` params for what the picker shows: the layout, and the typed items
/// (none typed: the layout's sample items).
pub fn insert_params(state: &diagram_picker::State) -> Value {
    let items: Vec<Value> = state.items().into_iter().map(|(text, level)| json!({"text": text, "level": level})).collect();
    let mut params = json!({"layout": state.layout});
    if !items.is_empty() {
        params["items"] = Value::Array(items);
    }
    params
}

/// The `smartArt.items` params for `g` with item `i`'s text replaced by `text`.
pub fn items_with(g: &SmartArtSpec, i: usize, text: &str) -> Value {
    let items: Vec<Value> =
        g.items.iter().enumerate().map(|(j, it)| json!({"text": if j == i { text } else { it.text.as_str() }, "level": it.level})).collect();
    json!({"items": items})
}

/// The `smartArt.items` params for `g` without item `i` (the items under it move up a level).
pub fn items_without(g: &SmartArtSpec, i: usize) -> Value {
    let under = g.descendants(i);
    let items: Vec<Value> = g
        .items
        .iter()
        .enumerate()
        .filter(|(j, _)| *j != i)
        .map(|(j, it)| json!({"text": it.text, "level": if under.contains(&j) { it.level.saturating_sub(1) } else { it.level }}))
        .collect();
    json!({"items": items})
}

/// The Text Pane: a row per item, indented by level. Typing changes the graphic as you go (one
/// undo step per item typed in); Enter adds an item after the current one.
pub fn text_pane(app: &mut WordApp, ui: &mut Ui) {
    let Some(g) = wordcraft_engine::cmd::smart_art::selected_smart_art(&app.session) else { return };
    let t = Tokens::get(ui.ctx());
    ui.label(egui::RichText::new(tl!("One line per item. Enter adds an item.")).color(t.text_dim));
    ui.add_space(4.0);
    let focus_id = egui::Id::new("smart_art_focus");
    let want_focus: Option<usize> = ui.data_mut(|d| d.remove_temp(focus_id));
    let mut typed = false;
    egui::ScrollArea::vertical().id_salt("smart_art_items").max_height((ui.available_height() - 40.0).max(60.0)).show(ui, |ui| {
        for (i, it) in g.items.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.add_space(f32::from(it.level) * 16.0);
                ui.label(egui::RichText::new("•").color(t.text_dim));
                let mut text = it.text.clone();
                let r = ui.add(egui::TextEdit::singleline(&mut text).id_salt(("smart_art_item", i)).desired_width(f32::INFINITY));
                if want_focus == Some(i) {
                    r.request_focus();
                }
                if r.has_focus() || r.gained_focus() {
                    app.smart_art_item = Some(i);
                }
                if r.changed() {
                    typed = true;
                    if app.smart_art_typing == Some(i) {
                        app.session.join_next_undo();
                    }
                    if app.run("smartArt.items", items_with(&g, i, &text)).is_ok() {
                        app.smart_art_typing = Some(i);
                    }
                }
                if r.lost_focus() && ui.input(|inp| inp.key_pressed(egui::Key::Enter)) && g.items.len() < MAX_ITEMS {
                    app.smart_art_typing = None;
                    if app.run("smartArt.addShape", json!({"after": i})).is_ok() {
                        // The new item comes after this one and the items under it.
                        let next = g.descendants(i).end;
                        app.smart_art_item = Some(next);
                        ui.data_mut(|d| d.insert_temp(focus_id, next));
                    }
                }
            });
        }
    });
    if !typed && !ui.ctx().memory(|m| m.focused().is_some()) {
        app.smart_art_typing = None;
    }
    ui.separator();
    let current = app.smart_art_item.filter(|i| *i < g.items.len());
    ui.horizontal_wrapped(|ui| {
        let at = current.map_or(json!({}), |i| json!({"after": i}));
        if ui.add_enabled(g.items.len() < MAX_ITEMS, egui::Button::new(tl!("Add"))).clicked() {
            app.smart_art_typing = None;
            let _ = app.run("smartArt.addShape", at);
        }
        let idx = current.map_or(json!({}), |i| json!({"index": i}));
        if ui.button(tl!("Promote")).clicked() {
            app.smart_art_typing = None;
            let _ = app.run("smartArt.promote", idx.clone());
        }
        if ui.button(tl!("Demote")).clicked() {
            app.smart_art_typing = None;
            let _ = app.run("smartArt.demote", idx);
        }
        if ui.add_enabled(current.is_some() && g.items.len() > 1, egui::Button::new(tl!("Remove"))).clicked()
            && let Some(i) = current
        {
            app.smart_art_typing = None;
            if app.run("smartArt.items", items_without(&g, i)).is_ok() {
                app.smart_art_item = i.checked_sub(1);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_keeps_action_buttons_inside_short_windows_and_inserts() {
        use egui_kittest::kittest::Queryable;
        for height in [420.0, 600.0, 900.0] {
            let mut app = WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), crate::Services::default());
            app.run("insert.smartArt", json!({})).unwrap();
            let mut h = egui_kittest::Harness::builder().with_size(vec2(900.0, height)).build_ui_state(
                |ui, app: &mut WordApp| {
                    let ctx = ui.ctx().clone();
                    app.logic(&ctx);
                    app.ui(ui);
                },
                app,
            );
            h.run_steps(5);
            let ok = h.get_by_label("OK").rect();
            let cancel = h.get_by_label("Cancel").rect();
            assert!(ok.top() >= 0.0 && ok.bottom() <= height && cancel.bottom() <= height, "buttons outside viewport {height}: {ok:?} {cancel:?}");
            h.get_by_label("OK").click();
            h.run_steps(3);
            assert!(wordcraft_engine::cmd::smart_art::selected_smart_art(&h.state().session).is_some());
        }
    }

    #[test]
    fn typed_items_become_the_diagram_with_tabs_as_sub_points() {
        let mut app = WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        let mut state = diagram_picker::State::new("hierarchy");
        assert_eq!(insert_params(&state), json!({"layout": "hierarchy"}));
        state.text = "Lead\n\tTeam A\n\tTeam B\n".into();
        app.run("insert.smartArt", insert_params(&state)).unwrap();
        let g = wordcraft_engine::cmd::smart_art::selected_smart_art(&app.session).unwrap();
        assert_eq!(g.layout, SmartArtLayout::Hierarchy);
        assert_eq!(g.items.iter().map(|i| (i.text.as_str(), i.level)).collect::<Vec<_>>(), [("Lead", 0), ("Team A", 1), ("Team B", 1)]);
    }

    #[test]
    fn removing_an_item_lifts_the_items_under_it() {
        let g = SmartArtSpec::sample(SmartArtLayout::Hierarchy);
        let v = items_without(&g, 1);
        let levels: Vec<u64> = v["items"].as_array().unwrap().iter().map(|i| i["level"].as_u64().unwrap()).collect();
        assert_eq!(levels, [0, 1, 1, 2, 2]);
        assert_eq!(items_with(&g, 0, "Boss")["items"][0]["text"], "Boss");
    }
}
