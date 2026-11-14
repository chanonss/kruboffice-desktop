#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

fn thai() -> Lang {
    Lang::from_code("th").expect("Thai is a built-in interface language")
}

/// The Thai catalog has no bad lines (each would show in English).
#[test]
fn the_thai_catalog_is_well_formed() {
    let (_, errors) = catalog::Catalog::parse(include_str!("th.tsv"));
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn locale_tags_resolve_to_thai_or_english() {
    for tag in ["th", "th-TH", "th_TH.UTF-8", "TH"] {
        assert_eq!(lang_from_tag(tag).map(Lang::code), Some("th"), "{tag}");
    }
    for tag in ["C", "POSIX", "en_US.UTF-8"] {
        assert_eq!(lang_from_tag(tag).map(Lang::code), Some("en"), "{tag}");
    }
    assert!(lang_from_tag("xx").is_none());
    assert_eq!(normalize_pref("TH"), Some("th"));
    assert_eq!(normalize_pref("auto"), Some(AUTO));
    assert_eq!(normalize_pref("klingon"), None);
}

#[test]
fn lookups_fall_back_to_english_and_fill_placeholders() {
    assert_eq!(tr(thai(), "A string nobody translated"), "A string nobody translated");
    assert_eq!(tr(Lang::EN, "Home"), "Home");
    assert_eq!(fmt("Slide {n} of {total}", &[("n", "2"), ("total", "9")]), "Slide 2 of 9");
    assert_eq!(fmtp("Slide {} of {}", &[&2, &"9"]), "Slide 2 of 9");
    assert_eq!(fmtp("{} {}", &[&"{}"]), "{} {}");
    assert_eq!(fmt("{n} {other}", &[("n", "{other}")]), "{other} {other}");
}

fn window() -> egui_kittest::Harness<'static, crate::SlideApp> {
    let app = crate::SlideApp::new(deckcraft_engine::Session::with_new(), crate::Services::default());
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1440.0, 900.0)).build_ui_state(
        |ui, app: &mut crate::SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(2);
    h
}

/// Shown as they are in Thai too: product, format and technology names, units and numbers.
const KEEP: [&str; 14] =
    ["SmartArt", "WordArt", "PhutKrub", "DeckCraft", "PDF", "PPTX", "PNG", "JPEG", "MIT OR Apache-2.0", "RRGGBB", "pt", "A+", "A-", "AaBbCcDd"];

/// What the ribbon tabs and the File pages draw, in Thai, minus [`KEEP`]: each string must have a
/// Thai entry, so a new button fails here until it is translated.
fn missing_on_tabs_and_file() -> Vec<String> {
    let mut h = window();
    h.state_mut().run("ui.language", serde_json::json!({"value": "th"})).unwrap();
    h.run_steps(3);
    record::start();
    for tab in ["Home", "Insert", "Draw", "Design", "Transitions", "Animations", "Slide Show", "Record", "Review", "View"] {
        h.state_mut().ui.tab = tab.into();
        h.run_steps(3);
    }
    for page in crate::backstage::Page::ALL {
        h.state_mut().open_backstage(page);
        h.run_steps(3);
    }
    let seen = record::stop();
    set_current(Lang::EN);
    let th = thai();
    let mut missing: Vec<String> = seen
        .into_iter()
        // Thai text looked up again (a translated label passing through a helper) is not an English key.
        .filter(|(_, s)| s.chars().any(|c| c.is_ascii_alphabetic()) && !s.chars().any(|c| ('\u{0E00}'..='\u{0E7F}').contains(&c)))
        .filter(|(_, s)| s.trim() == s && !KEEP.contains(&s.as_str()))
        .filter(|(ctx, s)| if ctx.is_empty() { !has(th, s) } else { trc(th, ctx, s) == s })
        .map(|(ctx, s)| format!("{ctx}\t{}", s.replace('\n', "\\n")))
        .collect();
    missing.dedup();
    missing
}

#[test]
fn the_most_seen_parts_are_in_thai() {
    let missing = missing_on_tabs_and_file();
    assert!(missing.is_empty(), "add these to th.tsv ({}):\n{}", missing.len(), missing.join("\n"));
}

/// Choosing Thai shows the ribbon's tabs in Thai and leaves the presentation untouched.
#[test]
fn choosing_thai_shows_the_ribbon_in_thai() {
    use egui_kittest::kittest::Queryable;
    let mut h = window();
    let before = serde_json::to_value(&h.state().session.active().unwrap().doc).unwrap();
    h.state_mut().run("ui.language", serde_json::json!({"value": "th"})).unwrap();
    h.run_steps(4);
    assert_eq!(h.state().ui.language, "th");
    for tab in ["หน้าแรก", "แทรก", "ออกแบบ", "การเปลี่ยนสไลด์", "ภาพเคลื่อนไหว", "ตรวจทาน", "มุมมอง"]
    {
        assert!(h.query_all_by_label(tab).next().is_some(), "the {tab} tab");
    }
    assert!(h.query_all_by_label("Transitions").next().is_none(), "no English tab name");
    assert_eq!(serde_json::to_value(&h.state().session.active().unwrap().doc).unwrap(), before);
    assert!(h.state_mut().run("ui.language", serde_json::json!({"value": "klingon"})).is_err());
    h.state_mut().run("ui.language", serde_json::json!({"value": "en"})).unwrap();
    h.run_steps(2);
    set_current(Lang::EN);
}
