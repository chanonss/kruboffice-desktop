//! The two-size ribbon (owner, 2026-11-13; KhianKrub is the standard): every group fits in two
//! rows of small buttons, on every tab, in every interface language.

use deckcraft_engine::Session;
use egui_kittest::Harness;
use serde_json::json;

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
    // A shape's, picture's, table's, chart's and SmartArt graphic's commands are in the
    // right-hand pane now, not in tabs of their own (owner, 2026-11-16).
    tall.sort();
    tall.dedup();
    assert!(tall.is_empty(), "groups taller than the two rows: {tall:#?}");
}
