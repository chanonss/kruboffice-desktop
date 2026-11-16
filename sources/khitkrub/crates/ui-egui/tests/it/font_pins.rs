//! Home ▸ Font: a pin at the end of a font's row pins it to the top of the list (the list stays
//! open), a second click unpins it, and the pinned fonts are saved with the interface settings.

use egui_kittest::kittest::{NodeT as _, Queryable as _};
use gridcraft_engine::Session;
use gridcraft_ui_egui::{SheetApp, UiState};
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn harness() -> Harness {
    let mut s = Session::new();
    s.new_workbook();
    s.execute("selection.set", json!({"range": "A1"})).unwrap();
    let mut h = crate::common::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        crate::common::app(s),
    );
    h.run_steps(4);
    h
}

fn a1_font(h: &Harness) -> String {
    let d = h.state().session.active().unwrap();
    let id = d.wb.active().unwrap().style_id(gridcraft_engine::core::CellRef::parse("A1").unwrap());
    d.wb.styles.get(id).font.name.clone()
}

/// The font of the list's top row that has a pin (the list scrolls; rows below aren't clickable).
fn top_font(h: &Harness) -> String {
    h.query_all_by(|n| n.label().is_some_and(|l| l.starts_with("Pin to Top: ")))
        .min_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
        .and_then(|n| n.accesskit_node().label())
        .map(|l| l.trim_start_matches("Pin to Top: ").to_string())
        .unwrap()
}

#[test]
fn fonts_pin_and_unpin_from_the_font_list() {
    let mut h = harness();
    let name = a1_font(&h);
    h.get_by_label(&name).click();
    h.run_steps(4);
    let font = top_font(&h);
    h.get_by_label(&format!("Pin to Top: {font}")).click();
    h.run_steps(4);
    assert_eq!(h.state().ui.pinned_fonts, vec![font.clone()]);
    // The list is still open, with the font at the top and in its place in the list.
    let unpin = format!("Unpin: {font}");
    assert_eq!(h.query_all_by_label(&unpin).count(), 2);
    let top = h.query_all_by_label(&unpin).min_by(|a, b| a.rect().top().total_cmp(&b.rect().top())).unwrap();
    top.click();
    h.run_steps(4);
    assert!(h.state().ui.pinned_fonts.is_empty());
    assert_eq!(a1_font(&h), name, "pinning doesn't change the cell's font");
    // A click on the name still applies the font and closes the list.
    let pick = top_font(&h);
    h.get_by_label(&pick).click();
    h.run_steps(4);
    assert_eq!(a1_font(&h), pick);
    assert!(h.query_by_label(&format!("Pin to Top: {pick}")).is_none(), "the list closed");
}

#[test]
fn pinned_fonts_are_saved_with_the_interface_settings() {
    let ui = UiState { pinned_fonts: vec!["Sarabun".into(), "Tahoma".into()], ..Default::default() };
    let saved = serde_json::to_string(&ui).unwrap();
    let back = serde_json::from_str::<UiState>(&saved).unwrap().at_startup();
    assert_eq!(back.pinned_fonts, ui.pinned_fonts);
    let damaged = serde_json::from_str::<UiState>(r#"{"pinnedFonts": [" Arial", "ARIAL", "", "Tahoma"]}"#).unwrap().at_startup();
    assert_eq!(damaged.pinned_fonts, vec!["Arial".to_string(), "Tahoma".to_string()]);
}
