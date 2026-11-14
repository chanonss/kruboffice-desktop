//! The two-size ribbon (owner, 2026-11-13; KhianKrub is the standard): every group fits in two
//! rows of small buttons, on every tab and every tab that comes with an object, in every
//! interface language.

use deckcraft_engine::Session;
use egui_kittest::Harness;
use serde_json::{Value, json};

use crate::{Services, SlideApp};

/// Render `tab` and return the groups drawn taller than the two rows, as "language tab @ x".
fn tall_groups(h: &mut Harness<'static, SlideApp>, tab: &str, lang: &str) -> Vec<String> {
    h.state_mut().ui.tab = tab.into();
    h.run_steps(2);
    crate::widgets::TALL_GROUPS.with(|g| g.borrow_mut().clear());
    h.run_steps(1);
    crate::widgets::TALL_GROUPS.with(|g| g.take()).into_iter().map(|g| format!("{lang} {tab} › {g}")).collect()
}

fn harness(session: Session) -> Harness<'static, SlideApp> {
    let app = SlideApp::new(session, Services::default());
    // Wide enough that no tab hides groups behind its scroll arrows.
    Harness::builder().with_size(egui::vec2(3200.0, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    )
}

fn png() -> String {
    deckcraft_engine::cmd::base64_encode(&crate::inbox_tests::png())
}

#[test]
fn ribbon_groups_fit_in_two_rows() {
    let mut tall = Vec::new();
    // The main tabs, Draw included.
    let mut h = harness(Session::with_new());
    h.state_mut().ui.show_draw_tab = true;
    for lang in ["en", "th"] {
        h.state_mut().run("ui.language", json!({"value": lang})).unwrap();
        for tab in super::TABS {
            tall.extend(tall_groups(&mut h, tab, lang));
        }
    }
    // The tabs that come with an object: insert it, select it, open its tabs.
    let cases: [(&str, Value, &[&str]); 5] = [
        ("shape.insert", json!({"preset": "rect", "rect": [80.0, 80.0, 200.0, 120.0]}), &["Shape Format"]),
        ("insert.picture", json!({"data": png(), "name": "p.png"}), &["Picture Format"]),
        ("insert.table", json!({"rows": 3, "cols": 3, "rect": [100, 150, 500, 200]}), &["Table Design", "Layout"]),
        ("insert.chart", json!({"type": "column", "rect": [80, 140, 600, 300]}), &["Chart Design"]),
        ("insert.smartArt", json!({"kind": "process"}), &["SmartArt Design"]),
    ];
    for (cmd, params, tabs) in cases {
        let mut s = Session::with_new();
        let id = s.execute(cmd, &params).unwrap().get("id").and_then(Value::as_u64);
        if let Some(id) = id {
            s.execute("edit.select", &json!({"ids": [id]})).unwrap();
        }
        let mut h = harness(s);
        for lang in ["en", "th"] {
            h.state_mut().run("ui.language", json!({"value": lang})).unwrap();
            for tab in tabs {
                assert!(super::contextual(h.state()).contains(tab), "{cmd} shows {tab}");
                tall.extend(tall_groups(&mut h, tab, lang));
            }
        }
    }
    tall.sort();
    tall.dedup();
    assert!(tall.is_empty(), "groups taller than the two rows: {tall:#?}");
}
