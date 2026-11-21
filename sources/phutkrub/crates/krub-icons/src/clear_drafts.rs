//! "Delete all drafts", the same in every KrubOffice web app (owner, 2026-11-21): a red button
//! under the drafts list and two confirmations, the second one clearly final. Only after both
//! does the app delete its drafts; Cancel, Escape or a click outside at either step deletes
//! nothing.
//!
//! The app draws [`button`] where it lists its drafts and calls [`dialog`] once a frame; `dialog`
//! returns `true` on the one frame the person confirmed twice, and the app then clears its own
//! storage (each app keeps its own). The red comes from the app's theme tokens. The steps follow
//! [`advance`], which the tests pin. The apps run different egui versions, so this uses only what
//! they share.

use egui::{Color32, FontId, RichText, Ui, vec2};

/// The dialog's width.
pub const WIDTH: f32 = 420.0;

/// The English strings the button and dialog show, for the apps' translation tables.
pub const NAMES: [&str; 8] = [BUTTON, FIRST_TITLE, FIRST_BODY, FIRST_GO, LAST_TITLE, LAST_BODY, LAST_GO, "Cancel"];

const BUTTON: &str = "Delete all drafts";
const FIRST_TITLE: &str = "Delete all your drafts?";
const FIRST_BODY: &str = "Just checking! This clears out every draft this app has saved in this browser on this device, unsaved changes to Google Drive files included. Your settings and the files on Google Drive itself stay right where they are. Want to keep going?";
const FIRST_GO: &str = "Yes, keep going";
const LAST_TITLE: &str = "Last chance!";
const LAST_BODY: &str = "Once they're gone, they're gone for good. This can't be undone. If there's anything you'd like to keep, download a copy first. Delete all your drafts now?";
const LAST_GO: &str = "Delete them all";

/// Which confirmation is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// "Delete all your drafts?"
    First,
    /// "Last chance!": the one that deletes.
    Last,
}

/// What the person chose at a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// The red button: go on.
    Go,
    /// Cancel, Escape or a click outside.
    Cancel,
}

/// The step after `choice` at `step`, and whether to delete now. Only going on at the last step
/// deletes; cancelling at either step closes the dialog and deletes nothing.
pub fn advance(step: Step, choice: Choice) -> (Option<Step>, bool) {
    match (step, choice) {
        (Step::First, Choice::Go) => (Some(Step::Last), false),
        (Step::Last, Choice::Go) => (None, true),
        (_, Choice::Cancel) => (None, false),
    }
}

fn state_id() -> egui::Id {
    crate::pane_ui::id("krub_clear_drafts")
}

/// Opens the dialog at `step` (screenshots and tests; the app opens it with [`button`]).
pub fn open(ctx: &egui::Context, step: Step) {
    set_step(ctx, Some(step));
}

/// The open step, if the dialog is showing.
pub fn step(ctx: &egui::Context) -> Option<Step> {
    ctx.data(|d| d.get_temp::<Step>(state_id()))
}

fn set_step(ctx: &egui::Context, step: Option<Step>) {
    ctx.data_mut(|d| {
        if let Some(s) = step {
            d.insert_temp(state_id(), s);
        } else {
            d.remove::<Step>(state_id());
        }
    });
}

/// Text on `fill`: white on a deep red, near-black on the light red of a dark theme.
pub fn ink_on(fill: Color32) -> Color32 {
    let luma = 0.299 * f32::from(fill.r()) + 0.587 * f32::from(fill.g()) + 0.114 * f32::from(fill.b());
    if luma > 160.0 { Color32::from_rgb(0x1A, 0x1A, 0x1A) } else { Color32::WHITE }
}

/// A button filled with `red`, its text in [`ink_on`].
fn red_button(ui: &mut Ui, label: &str, red: Color32) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(label).color(ink_on(red))).fill(red).min_size(vec2(84.0, 28.0)))
}

/// The red "Delete all drafts" button; a click opens the first confirmation. `enabled` is false
/// while there is nothing to delete or the app is busy.
pub fn button(ui: &mut Ui, red: Color32, enabled: bool, tr: &dyn Fn(&str) -> String) -> egui::Response {
    let resp = ui.add_enabled_ui(enabled, |ui| red_button(ui, &tr(BUTTON), red)).inner;
    if resp.clicked() {
        set_step(ui.ctx(), Some(Step::First));
    }
    resp
}

/// Draws the open confirmation, if any, for one frame. Returns `true` on the frame the person
/// confirmed the second time: the app deletes its drafts then.
pub fn dialog(ctx: &egui::Context, red: Color32, tr: &dyn Fn(&str) -> String) -> bool {
    let Some(step) = step(ctx) else { return false };
    let (title, body, go) = match step {
        Step::First => (FIRST_TITLE, FIRST_BODY, FIRST_GO),
        Step::Last => (LAST_TITLE, LAST_BODY, LAST_GO),
    };
    let mut choice = None;
    // One modal for both steps: a second modal layer would not be the top one on its first
    // frame (egui keeps the last top modal), so it would not show at once.
    let modal = egui::Modal::new(state_id().with("modal")).show(ctx, |ui| {
        ui.set_width(WIDTH);
        ui.label(RichText::new(tr(title)).font(FontId::proportional(16.0)).strong().color(red));
        ui.add_space(8.0);
        ui.label(tr(body));
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if red_button(ui, &tr(go), red).clicked() {
                    choice = Some(Choice::Go);
                }
                if ui.add(egui::Button::new(tr("Cancel")).min_size(vec2(84.0, 28.0))).clicked() {
                    choice = Some(Choice::Cancel);
                }
            });
        });
    });
    if choice.is_none() && modal.should_close() {
        choice = Some(Choice::Cancel);
    }
    let Some(choice) = choice else { return false };
    let (next, delete) = advance(step, choice);
    set_step(ctx, next);
    delete
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deletes_only_after_both_confirmations() {
        assert_eq!(advance(Step::First, Choice::Go), (Some(Step::Last), false), "the first yes only opens the last chance");
        assert_eq!(advance(Step::Last, Choice::Go), (None, true));
        for step in [Step::First, Step::Last] {
            assert_eq!(advance(step, Choice::Cancel), (None, false), "cancelling at {step:?} keeps everything");
        }
    }

    #[test]
    fn button_text_reads_on_every_red() {
        assert_eq!(ink_on(Color32::from_rgb(0xD1, 0x3B, 0x3B)), Color32::WHITE, "light theme red");
        assert_eq!(ink_on(Color32::from_rgb(0xC4, 0x2B, 0x1C)), Color32::WHITE);
        assert_ne!(ink_on(Color32::from_rgb(0xFF, 0x95, 0x85)), Color32::WHITE, "a pale dark-theme red takes dark text");
    }
}
