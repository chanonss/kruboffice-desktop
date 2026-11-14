use super::animations::marked_options;
use super::{
    Align2, CornerRadius, Icon, Rect, Sense, SlideApp, Stroke, Tokens, Ui, Value, big, big_button, group, json, pos2, rows, run, theme, vec2, widgets,
};

// ---------- Transitions ----------

/// Render transition preview, gallery, options, sound, and timing controls for the current
/// slide.
pub(super) fn transitions(app: &mut SlideApp, ui: &mut Ui) {
    let cur = app.session.active().and_then(|d| d.current_slide().and_then(|s| s.transition.clone()));
    let kind = cur.as_ref().map(|t| t.kind.clone()).unwrap_or_else(|| "none".into());
    group(ui, |ui| {
        let playing = app.show.as_ref().is_some_and(|s| s.preview_only);
        let (resp, _) = widgets::big_button_ex(ui, Icon::Preview, tl!("Preview"), true, false, playing);
        if resp.on_hover_text(tl!("Play this slide's transition")).clicked() {
            let to = app.session.active().map(|d| d.selection.slide).unwrap_or(0);
            app.start_show(to, true);
            let now = ui.input(|i| i.time);
            if let (Some(s), Some(d)) = (app.show.as_mut(), app.session.active()) {
                s.start_transition_preview(&d.doc, now);
            }
        }
    });
    group(ui, |ui| {
        let list = deckcraft_model::anim::TRANSITIONS;
        let shown: Vec<_> = list.iter().take(12).collect();
        let sel = shown.iter().position(|x| x.0 == kind);
        if let Some(i) = widgets::gallery(
            ui,
            shown.len(),
            vec2(54.0, 56.0),
            sel,
            |p, r, i| paint_transition_tile(p, r, shown.get(i).map(|x| (x.0, x.1)).unwrap_or(("", ""))),
            |i| shown.get(i).map(|x| crate::i18n::t(x.1).to_string()).unwrap_or_default(),
        ) && let Some(x) = shown.get(i)
        {
            run(app, "transition.set", json!({"kind": x.0}));
        }
        let more = widgets::drop_button(ui, "", vec2(14.0, 52.0), true);
        egui::Popup::menu(&more).width(480.0).show(|ui| {
            for cat in ["Subtle", "Exciting", "Dynamic Content"] {
                ui.label(egui::RichText::new(crate::i18n::t(cat)).font(theme::bold(12.0)));
                ui.horizontal_wrapped(|ui| {
                    for x in list.iter().filter(|x| x.2 == cat) {
                        let (r, resp) = ui.allocate_exact_size(vec2(56.0, 58.0), Sense::click());
                        if resp.hovered() || x.0 == kind {
                            ui.painter().rect_filled(r, CornerRadius::same(4), Tokens::get(ui.ctx()).hover);
                        }
                        paint_transition_tile(ui.painter(), r.shrink(3.0), (x.0, x.1));
                        if resp.clicked() {
                            run(app, "transition.set", json!({"kind": x.0}));
                            ui.close();
                        }
                    }
                });
            }
        });
    });
    group(ui, |ui| {
        let selected = cur.as_ref().map(|t| t.option.as_str());
        let rows = transition_effect_rows(&kind, selected);
        let r = big_button(ui, Icon::EffectOptions, tl!("Effect\nOptions"), !rows.is_empty());
        egui::Popup::menu(&r).show(|ui| {
            for &(o, on) in &rows {
                if ui.selectable_label(on, deckcraft_model::anim::transition_option_label(&kind, o)).clicked() {
                    run(app, "transition.options", json!({"option": o}));
                    ui.close();
                }
            }
        });
    });
    group(ui, |ui| {
        // Sound in its own column, so the timing column keeps its three rows.
        rows(ui, |ui| {
            ui.horizontal(|ui| {
                let field = ui.label(tl!("Sound:"));
                let stop = cur.as_ref().is_some_and(|t| t.stop_sound);
                let name = cur
                    .as_ref()
                    .and_then(|t| t.sound)
                    .and_then(|m| app.session.active().and_then(|d| d.doc.media(m).map(|i| i.name.trim_end_matches(".wav").to_string())))
                    .unwrap_or_else(|| tl!(if stop { "[Stop Previous Sound]" } else { "[No Sound]" }).to_string());
                let combo = craft_ui::ComboBox::from_id_salt("transition_sound").width(140.0).selected_text(name).show_ui(ui, |ui| {
                    let none = cur.as_ref().is_none_or(|t| t.sound.is_none() && !t.stop_sound);
                    if ui.selectable_label(none, tl!("[No Sound]")).clicked() {
                        run(app, "transition.sound", json!({"none": true}));
                    }
                    if ui.selectable_label(stop, tl!("[Stop Previous Sound]")).clicked() {
                        run(app, "transition.sound", json!({"stopPrevious": true}));
                    }
                    ui.separator();
                    for (id, label) in deckcraft_engine::sounds::BUILTIN {
                        if ui.button(label).clicked() {
                            let lp = cur.as_ref().is_some_and(|t| t.sound_loop);
                            run(app, "transition.sound", json!({"builtin": id, "loop": lp}));
                        }
                    }
                    ui.separator();
                    let can_pick = app.services.pick_open.is_some();
                    let r = ui.add_enabled(can_pick, egui::Button::new(tl!("Other Sound…")));
                    if r.clicked() {
                        app.pick_transition_sound();
                    }
                    r.on_disabled_hover_text(tl!("Choosing a sound file needs the desktop app"));
                    ui.separator();
                    let has = cur.as_ref().is_some_and(|t| t.sound.is_some());
                    let mut lp = cur.as_ref().is_some_and(|t| t.sound_loop);
                    if ui.add_enabled(has, egui::Checkbox::new(&mut lp, tl!("Loop Until Next Sound"))).changed() {
                        run(app, "transition.sound", json!({"loop": lp}));
                    }
                });
                let _ = combo.response.labelled_by(field.id);
            });
        });
        rows(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(tl!("Duration:"));
                let d = cur.as_ref().map(|t| t.duration_ms as f64 / 1000.0).unwrap_or(1.0);
                if let Some(v) = widgets::spinner(ui, "tdur", tl!("Duration"), d, 0.25, "", 46.0) {
                    run(app, "transition.timing", json!({"duration": (v.max(0.0) * 1000.0).round() as u64}));
                }
                ui.add_space(8.0);
                let mut click = cur.as_ref().is_none_or(|t| t.advance_on_click);
                if ui.checkbox(&mut click, tl!("On Mouse Click")).changed() {
                    run(app, "transition.timing", json!({"onClick": click}));
                }
            });
            ui.add_space((crate::ribbon_layout::ROW_GAP - ui.spacing().item_spacing.y).max(0.0));
            ui.horizontal(|ui| {
                let after = cur.as_ref().and_then(|t| t.advance_after_ms);
                let mut on = after.is_some();
                if ui.checkbox(&mut on, tl!("After:")).changed() {
                    run(app, "transition.timing", json!({"after": if on { json!(after.unwrap_or(0)) } else { Value::Null }}));
                }
                if let Some(v) = widgets::spinner(ui, "tafter", tl!("After"), after.unwrap_or(0) as f64 / 1000.0, 1.0, "", 46.0) {
                    run(app, "transition.timing", json!({"after": (v.max(0.0) * 1000.0).round() as u64}));
                }
            });
        });
        ui.add_space(6.0);
        big(app, ui, Icon::ApplyToAll, tl!("Apply\nTo All"), "transition.applyAll", json!({}));
    });
}

pub(super) fn option_label(o: &str) -> String {
    match o {
        "l" => "From Left".into(),
        "r" => "From Right".into(),
        "u" => "From Top".into(),
        "d" => "From Bottom".into(),
        "horz" => "Horizontal".into(),
        "vert" => "Vertical".into(),
        "in" => "In".into(),
        "out" => "Out".into(),
        other => {
            let mut s = String::new();
            for (i, c) in other.chars().enumerate() {
                if i == 0 {
                    s.extend(c.to_uppercase());
                } else if c.is_uppercase() {
                    s.push(' ');
                    s.push(c);
                } else {
                    s.push(c);
                }
            }
            s
        }
    }
}

pub fn paint_transition_tile(p: &egui::Painter, r: Rect, (id, label): (&str, &str)) {
    let t = Tokens::get(p.ctx());
    // The line style of the icons: a thin ink outline of the slide, the effect in pale gold.
    let pic = Rect::from_min_size(r.min + vec2(4.0, 2.0), vec2(r.width() - 8.0, r.height() - 18.0));
    let ink = t.text;
    let gold = krub_icons::gold(ink, krub_icons::GoldRole::Surface, 1.0);
    let line = Stroke::new(1.2, ink);
    let band = |x0: f32, x1: f32| Rect::from_min_max(pos2(pic.min.x + pic.width() * x0, pic.min.y), pos2(pic.min.x + pic.width() * x1, pic.max.y));
    match id {
        "none" => {}
        "fade" | "dissolve" | "flash" => {
            p.rect_filled(pic, CornerRadius::same(2), gold.gamma_multiply(0.6));
        }
        "push" | "cover" | "uncover" | "pan" | "conveyor" => {
            p.rect_filled(band(0.5, 1.0), CornerRadius::same(2), gold);
            p.line_segment([pos2(pic.min.x + pic.width() * 0.5, pic.min.y), pos2(pic.min.x + pic.width() * 0.5, pic.max.y)], line);
            let y = pic.center().y;
            let (a, b) = (pic.max.x - 6.0, pic.center().x - 6.0);
            p.line_segment([pos2(a, y), pos2(b, y)], line);
            p.line_segment([pos2(b + 3.5, y - 3.5), pos2(b, y)], line);
            p.line_segment([pos2(b + 3.5, y + 3.5), pos2(b, y)], line);
        }
        "wipe" | "reveal" => {
            p.rect_filled(band(0.0, 0.5), CornerRadius::same(2), gold);
            p.line_segment([pos2(pic.center().x, pic.min.y), pos2(pic.center().x, pic.max.y)], line);
        }
        "split" | "doors" | "window" | "curtains" => {
            let mid = band(0.3, 0.7);
            p.rect_filled(mid, CornerRadius::ZERO, gold);
            for x in [mid.min.x, mid.max.x] {
                p.line_segment([pos2(x, pic.min.y), pos2(x, pic.max.y)], line);
            }
        }
        "randomBar" | "blinds" | "comb" | "shred" => {
            for k in 0..5 {
                let x0 = k as f32 / 5.0;
                p.rect_filled(band(x0, x0 + 0.1), CornerRadius::ZERO, gold);
                p.line_segment([pos2(pic.min.x + pic.width() * (x0 + 0.1), pic.min.y), pos2(pic.min.x + pic.width() * (x0 + 0.1), pic.max.y)], line);
            }
        }
        "checker" | "honeycomb" | "glitter" | "fracture" => {
            for k in 0..12 {
                if k % 2 == (k / 4) % 2 {
                    let x = pic.min.x + (k % 4) as f32 * pic.width() / 4.0;
                    let y = pic.min.y + (k / 4) as f32 * pic.height() / 3.0;
                    p.rect_filled(Rect::from_min_size(pos2(x, y), vec2(pic.width() / 4.0, pic.height() / 3.0)), CornerRadius::ZERO, gold);
                }
            }
        }
        "shape" | "zoom" | "ripple" | "vortex" | "clock" => {
            p.circle_filled(pic.center(), pic.height() * 0.36, gold);
            p.circle_stroke(pic.center(), pic.height() * 0.36, line);
        }
        "morph" => {
            let c = pos2(pic.min.x + pic.width() * 0.3, pic.center().y);
            p.circle_filled(c, pic.height() * 0.24, gold);
            p.circle_stroke(c, pic.height() * 0.24, line);
            let sq = Rect::from_center_size(pos2(pic.max.x - pic.width() * 0.3, pic.center().y), vec2(pic.height() * 0.45, pic.height() * 0.45));
            p.rect(sq, CornerRadius::same(2), gold, line, egui::StrokeKind::Middle);
        }
        _ => {
            // A tilted card for the 3-D effects.
            let pts = vec![
                pos2(pic.min.x + 6.0, pic.min.y + 4.0),
                pos2(pic.max.x - 3.0, pic.min.y + 1.0),
                pos2(pic.max.x - 3.0, pic.max.y - 1.0),
                pos2(pic.min.x + 6.0, pic.max.y - 4.0),
            ];
            p.add(egui::Shape::convex_polygon(pts, gold, line));
        }
    }
    p.rect_stroke(pic, CornerRadius::same(2), line, egui::StrokeKind::Inside);
    p.text(pos2(r.center().x, r.max.y - 7.0), Align2::CENTER_CENTER, crate::i18n::t(label), theme::font(10.0), t.text);
}

/// One row of Transitions ▸ Effect Options: the stored option and whether it is the current one.
pub(super) fn transition_effect_rows(kind: &str, current: Option<&str>) -> Vec<(&'static str, bool)> {
    let opts = deckcraft_model::anim::TRANSITIONS.iter().find(|x| x.0 == kind).map(|x| x.4).unwrap_or(&[]);
    marked_options(opts, current)
}
