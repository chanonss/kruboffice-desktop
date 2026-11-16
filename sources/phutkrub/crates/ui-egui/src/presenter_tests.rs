//! JJ3: `show.presenter` answers the setting and where presenter view appears.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::json;

use crate::{Services, SlideApp};
use deckcraft_engine::Session;

#[test]
fn show_presenter_answers_on_and_window() {
    let mut app = SlideApp::new(Session::with_new(), Services::default());
    let v = app.run("show.presenter", json!({"on": true})).unwrap();
    assert_eq!(v, json!({"on": true, "window": true}));
    let v = app.run("show.presenter", json!({})).unwrap();
    assert_eq!(v["on"], false, "no `on` toggles: {v}");
    let doc = crate::UI_COMMANDS.iter().find(|c| c.0 == "show.presenter").unwrap().3;
    assert!(doc.contains("separate window"), "{doc}");
}

#[test]
fn presenter_view_needs_a_second_display() {
    // One display: the presenter window covered the show (or the show covered it) and the
    // hidden window stalled every frame.
    let presenter = |displays: Option<usize>, setting: bool| {
        let services = Services { displays: Some(Box::new(move || displays)), ..Default::default() };
        let mut app = SlideApp::new(Session::with_new(), services);
        app.ui.presenter_view = setting;
        app.start_show(0, false);
        app.show.as_ref().map(|s| s.presenter)
    };
    assert_eq!(presenter(Some(1), true), Some(false));
    assert_eq!(presenter(Some(2), true), Some(true));
    assert_eq!(presenter(None, true), Some(true), "a host that can't count displays keeps Presenter View");
    assert_eq!(presenter(Some(2), false), Some(false), "the setting still turns it off");
}

#[test]
fn two_displays_place_the_show_and_presenter_apart() {
    // #433: the presenter keeps the editor's display, the slides fill the other one.
    let start = |list: Option<crate::DisplayList>, setting: bool| {
        let services = Services { monitors: Some(Box::new(move || list.clone())), ..Default::default() };
        let mut app = SlideApp::new(Session::with_new(), services);
        app.ui.presenter_view = setting;
        app.start_show(0, false);
        app
    };
    let two = crate::DisplayList { displays: vec![0, 1], here: Some(0), primary: Some(0), ..Default::default() };
    let mut app = start(Some(two.clone()), true);
    assert_eq!(app.show.as_ref().unwrap().displays.map(|p| (p.audience, p.presenter)), Some((1, 0)));
    let v = app.run("show.swapDisplays", json!({})).unwrap();
    assert_eq!(v, json!({"swapped": true, "audience": 0, "presenter": 1}));
    assert_eq!(app.show.as_ref().unwrap().displays.map(|p| (p.audience, p.presenter)), Some((0, 1)));

    // One display (a laptop alone, or mirrored screens): only the slides, as on Linux (#1156).
    // On macOS the presenter window over the show crashed egui-wgpu.
    let one = crate::DisplayList { displays: vec![0, 0], here: Some(0), primary: Some(0), ..Default::default() };
    let mut app = start(Some(one), true);
    assert!(!app.show.as_ref().unwrap().presenter, "one display: no Presenter View");
    assert_eq!(app.show.as_ref().unwrap().displays, None);
    assert_eq!(app.run("show.swapDisplays", json!({})).unwrap(), json!({"swapped": false}));
    let none = crate::DisplayList::default();
    assert!(start(Some(none), true).show.as_ref().unwrap().presenter, "an empty list keeps Presenter View");

    assert_eq!(start(Some(two), false).show.as_ref().unwrap().displays, None, "Presenter View off: the show stays where it is");
    assert_eq!(start(None, true).show.as_ref().unwrap().displays, None, "a host that can't list displays keeps the old behaviour");

    let mut idle = SlideApp::new(Session::with_new(), Services::default());
    assert_eq!(idle.run("show.swapDisplays", json!({})).unwrap(), json!({"swapped": false}), "no show running");
}
