//! Home › Font: a pin at the end of a font's row pins it to the top of the list (the list stays
//! open), a second click unpins it, and the pinned fonts are saved (desktop `ui.json`, the web's
//! `localStorage`).

use deckcraft_engine::Session;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable};
use serde_json::json;

use crate::{Services, SlideApp, UiState};

fn harness() -> Harness<'static, SlideApp> {
    let mut s = Session::with_new();
    s.execute("slide.new", &json!({"layout": "blank"})).unwrap();
    s.execute("insert.textBox", &json!({"rect": [40, 40, 400, 80], "text": "Hello fonts"})).unwrap();
    s.execute("text.exit", &json!({})).unwrap();
    s.execute("format.font", &json!({"family": "Comic Neue"})).unwrap();
    let mut h = Harness::builder().with_size(egui::vec2(1440.0, 900.0)).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SlideApp::new(s, Services::default()),
    );
    h.run_steps(3);
    h
}

/// The pin labels of the open list, top to bottom.
fn pins(h: &Harness<'_, SlideApp>, prefix: &str) -> Vec<String> {
    let mut v: Vec<(f32, String)> = h
        .query_all_by(|n| n.label().is_some_and(|l| l.starts_with(prefix)))
        .filter_map(|n| Some((n.rect().top(), n.accesskit_node().label()?.trim_start_matches(prefix).to_string())))
        .collect();
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    v.into_iter().map(|(_, l)| l).collect()
}

#[test]
fn fonts_pin_and_unpin_from_the_home_font_list() {
    let mut h = harness();
    h.get_by_label("Font").click();
    h.run_steps(3);
    // The two theme fonts come first; pin the first of the whole list.
    let font = pins(&h, "Pin to Top: ").get(2).cloned().expect("the list shows its fonts");
    h.get_by_label(&format!("Pin to Top: {font}")).click();
    h.run_steps(3);
    assert_eq!(h.state().ui.pinned_fonts, vec![font.clone()]);
    // Still open, the font now in the Pinned Fonts section as well as in the list.
    assert!(pins(&h, "Unpin: ").iter().filter(|f| **f == font).count() >= 2);
    h.query_all_by_label(&format!("Unpin: {font}")).min_by(|a, b| a.rect().top().total_cmp(&b.rect().top())).unwrap().click();
    h.run_steps(3);
    assert!(h.state().ui.pinned_fonts.is_empty());
}

#[test]
fn pinned_fonts_are_saved() {
    let ui = UiState { pinned_fonts: vec!["Sarabun".into(), "Tahoma".into()], ..Default::default() };
    // Desktop: ui.json.
    let back: UiState = serde_json::from_str(&serde_json::to_string(&ui).unwrap()).unwrap();
    assert_eq!(back.sanitized().pinned_fonts, ui.pinned_fonts);
    // Web: its own localStorage key.
    assert_eq!(UiState::pinned_fonts_from_json(Some(&ui.pinned_fonts_json())), ui.pinned_fonts);
    assert_eq!(UiState::pinned_fonts_from_json(Some(r#"[" Arial", "ARIAL", "", "Tahoma"]"#)), vec!["Arial".to_string(), "Tahoma".to_string()]);
    assert!(UiState::pinned_fonts_from_json(Some("{oops")).is_empty());
    assert!(UiState::pinned_fonts_from_json(None).is_empty());
}
