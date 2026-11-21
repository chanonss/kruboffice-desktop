//! Constructors for the suite's usual harness shapes.
//!
//! The strict font and lifecycle setup stays in `crate::common`; these wrap it at the sizes and
//! step timings the `it` tests share. Keeping them here (rather than in `common`) leaves the
//! `keytips` test binary's copy of `common` free of helpers only this binary uses.

use crate::common::{Harness, app, builder, harness_with, prepare};
use gridcraft_engine::Session;
use gridcraft_ui_egui::{Services, SheetApp};

/// A blank workbook session, as most integration tests start.
pub fn blank_session() -> Session {
    let mut session = Session::new();
    session.new_workbook();
    session
}

/// The suite's usual 1200×800 harness around `session`, warmed for `steps` frames.
pub fn harness(session: Session, steps: usize) -> Harness {
    harness_with(builder().with_size(egui::vec2(1200.0, 800.0)), app(session), steps)
}

/// A blank workbook in the suite's usual 1200×800 harness.
pub fn blank_harness(steps: usize) -> Harness {
    harness(blank_session(), steps)
}

/// A 1200×800 harness advancing in 1/60 s frames, for drags and hover sequences.
pub fn stepped_harness(session: Session) -> Harness {
    harness_with(builder().with_step_dt(1.0 / 60.0).with_size(egui::vec2(1200.0, 800.0)), app(session), 4)
}

/// A harness around an app built with the caller's `Services` (e.g. a test inbox).
pub fn services_harness(services: Services, steps: usize) -> Harness {
    harness_with(builder().with_size(egui::vec2(1200.0, 800.0)), prepare(SheetApp::new(blank_session(), services)), steps)
}

/// A harness around an already-prepared `app`, at the suite's usual 1200×800 size.
pub fn app_harness(app: SheetApp, steps: usize) -> Harness {
    harness_with(builder().with_size(egui::vec2(1200.0, 800.0)), app, steps)
}

/// Show the Shape pane's sub-panel `tab` (owner, 2026-11-18), as a click on its slot does.
pub fn show_shape_tab(h: &mut crate::common::Harness, tab: krub_icons::shape_pane::Tab) {
    let tabs = krub_icons::shape_pane::Tab::shown(krub_icons::shape_pane::Caps::ALL);
    let i = tabs.iter().position(|t| *t == tab).unwrap_or(0);
    h.ctx.data_mut(|d| d.insert_persisted(krub_icons::pane_ui::id(("krub_shape_tab", tabs.len())), i));
    h.run_steps(3);
}
