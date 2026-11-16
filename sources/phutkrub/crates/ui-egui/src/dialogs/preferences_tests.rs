//! The Preferences (Options) dialog: controls apply as they change and Cancel, Esc and the
//! window's close button put the settings back the way they were (#752, reported as #878).

use deckcraft_engine::Session;
use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, Node};
use serde_json::json;

use crate::theme::Brightness;
use crate::{Services, SlideApp};

/// The Options category `name` in the list on the left.
fn category<'a>(h: &'a Harness<'_, SlideApp>, name: &'a str) -> Node<'a> {
    h.get_by_role_and_label(egui::accesskit::Role::Tab, name)
}

/// Choose `item` from the dropdown labelled `label`.
pub(crate) fn pick(h: &mut Harness<'_, SlideApp>, label: &str, item: &str) {
    // A popup egui still remembers as open closes on the first click: open it again then.
    for _ in 0..2 {
        h.get_by_role_and_label(egui::accesskit::Role::ComboBox, label).click();
        h.run_steps(3);
        if h.query_all_by_label(item).count() > 0 {
            break;
        }
    }
    h.query_all_by_label(item).last().unwrap_or_else(|| panic!("{item} is in the {label} list")).click();
    h.run_steps(2);
}

fn harness() -> Harness<'static, SlideApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SlideApp::new(Session::with_new(), Services::default()),
    );
    // The app keeps asking for frames, so step instead of run().
    h.run_steps(3);
    h
}

fn open_options(h: &mut Harness<'_, SlideApp>, pane: &str) {
    h.state_mut().run("app.preferences", json!({"pane": pane})).expect("open Options");
    h.run_steps(3);
}

fn dark(app: &SlideApp) -> bool {
    app.ui.brightness == Brightness::Dark
}

/// A checkbox applies as it is toggled and Cancel puts the old value back.
#[test]
fn options_cancel_undoes_the_changes() {
    let mut h = harness();
    let was = h.state().session.prefs.smart_guides;
    open_options(&mut h, "advanced");

    h.query_by_label("Show smart guides").expect("the checkbox is there").click();
    h.run_steps(2);
    assert_eq!(h.state().session.prefs.smart_guides, !was, "the change applies as it is made");

    h.query_by_label("Cancel").expect("the Cancel button is there").click();
    h.run_steps(2);
    assert_eq!(h.state().session.prefs.smart_guides, was, "Cancel restores the setting");
    assert!(h.state().dialog.is_none(), "and the dialog closes");
}

/// Every Editing setting and the name field go back on Cancel, not only the first checkbox: a
/// reporter who changed a different control and saw Cancel do nothing has the same answer.
#[test]
fn options_cancel_undoes_every_editing_setting() {
    let mut h = harness();
    let before = h.state().session.prefs.clone();
    open_options(&mut h, "advanced");

    for (pane, label) in
        [("Advanced", "Snap objects to grid"), ("Proofing", "Replace straight quotes with smart quotes"), ("Proofing", "AutoCorrect as you type")]
    {
        category(&h, pane).click();
        h.run_steps(2);
        h.query_by_label(label).unwrap_or_else(|| panic!("the {label} checkbox is there")).click();
        h.run_steps(1);
    }
    // The name field, under General: it writes into the settings as it is typed.
    category(&h, "General").click();
    h.run_steps(2);
    // The name field is the last text input of the dialog (the label is a separate widget).
    let name = h.get_all_by_role(egui::accesskit::Role::TextInput).last().expect("the name field is there");
    name.focus();
    h.run_steps(1);
    h.event(egui::Event::Text("Z".into()));
    h.run_steps(1);
    let changed = h.state().session.prefs.clone();
    assert_ne!(
        (changed.snap_to_grid, changed.smart_quotes, changed.autocorrect),
        (before.snap_to_grid, before.smart_quotes, before.autocorrect),
        "the settings apply as they are changed"
    );
    assert_ne!(changed.author, before.author, "the name applies as it is typed: {:?}", changed.author);

    h.query_by_label("Cancel").expect("the Cancel button is there").click();
    h.run_steps(2);
    let after = h.state().session.prefs.clone();
    assert_eq!(after.snap_to_grid, before.snap_to_grid, "snap to grid is restored");
    assert_eq!(after.smart_quotes, before.smart_quotes, "smart quotes are restored");
    assert_eq!(after.autocorrect, before.autocorrect, "autocorrect is restored");
    assert_eq!(after.author, before.author, "the name is restored");
}

/// The interface theme and size go back on Cancel, too; Esc acts as Cancel.
#[test]
fn options_cancel_puts_the_appearance_and_scale_back() {
    let mut h = harness();
    let (dark_before, scale_before) = (dark(h.state()), h.state().ui.ui_scale);
    open_options(&mut h, "general");

    pick(&mut h, "Interface theme:", "Dark");
    assert!(dark(h.state()), "the appearance applies as it is chosen");

    pick(&mut h, "Interface size:", "125%");
    let scale_in_dialog = h.state().ui.ui_scale;
    assert!((scale_in_dialog - 1.25).abs() < 1e-3, "the size applies as it is chosen: {scale_in_dialog}");

    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert_eq!(dark(h.state()), dark_before, "Esc restores the appearance");
    assert!((h.state().ui.ui_scale - scale_before).abs() < 1e-3, "Esc restores the size");
    assert!(h.state().dialog.is_none(), "and the dialog closes");
}

/// The window's close button is the same as Cancel: the settings go back.
#[test]
fn options_close_button_restores_the_settings() {
    let mut h = harness();
    let was = h.state().session.prefs.smart_guides;
    open_options(&mut h, "advanced");

    h.query_by_label("Show smart guides").expect("the checkbox is there").click();
    h.run_steps(2);
    assert_eq!(h.state().session.prefs.smart_guides, !was, "the change applies as it is made");

    h.get_all_by_label("Close window").next().expect("the window's close button is there").click();
    h.run_steps(2);
    assert_eq!(h.state().session.prefs.smart_guides, was, "the close button restores the setting");
    assert!(h.state().dialog.is_none(), "and the dialog closes");
}

/// Save › Embed fonts in the file changes the presentation (`file.embedFonts`); the choice of
/// characters is offered once it is on.
#[test]
fn the_save_category_embeds_fonts_in_the_presentation() {
    let mut h = harness();
    open_options(&mut h, "save");
    let doc = |h: &Harness<'_, SlideApp>| {
        let d = &h.state().session.active().expect("a presentation").doc;
        (d.embed_fonts, d.embed_all_characters)
    };
    assert_eq!(doc(&h), (false, false));
    h.query_by_label("Embed fonts in the file").expect("the checkbox is there").click();
    h.run_steps(2);
    assert_eq!(doc(&h), (true, false), "only the characters used, by default");
    h.query_by_label("Embed all characters (best for editing by other people)").expect("the choice is there").click();
    h.run_steps(2);
    assert_eq!(doc(&h), (true, true));
    h.state_mut().run("edit.undo", json!({})).expect("undo");
    assert_eq!(doc(&h), (true, false), "each change is an edit of the presentation");
}
