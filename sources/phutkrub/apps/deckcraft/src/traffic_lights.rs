//! Moves AppKit's traffic lights down to the middle of the one bar (macOS).
//!
//! AppKit puts the close, minimise and zoom buttons about 16 pt from the top of the window, but
//! the one bar's tabs, File, the file pill and the user icon sit in the tab row under an 8 pt gap,
//! 23 pt from the top (`ribbon::MAC_LIGHTS_CENTRE_Y`), so the lights looked too high
//! (owner, 2026-11-27). This makes the title-bar container that holds them taller and moves the
//! buttons down, as Tauri's `trafficLightPosition` does. AppKit lays the title bar out again when
//! the window resizes or leaves full screen, so the app calls [`centre_at`] every frame; it only
//! touches the views when the lights are out of place. Every call is safe in the `objc2-app-kit`
//! bindings (our workspace forbids unsafe).

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSButton, NSView, NSWindow, NSWindowButton, NSWindowStyleMask};

/// Puts the traffic lights' centres `centre_y` points below the top of each window that draws
/// into its title bar (the full-size content view). Leaves full-screen windows, panels and sheets
/// alone.
pub fn centre_at(centre_y: f64) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    for window in NSApplication::sharedApplication(mtm).windows().to_vec() {
        let mask = window.styleMask();
        if mask.contains(NSWindowStyleMask::FullSizeContentView) && !mask.contains(NSWindowStyleMask::FullScreen) {
            place(&window, centre_y);
        }
    }
}

/// The button's centre, in points below the top of the window.
fn centre_from_top(window: &NSWindow, button: &NSButton) -> f64 {
    // Window coordinates count up from the bottom.
    let frame = button.convertRect_toView(button.bounds(), None);
    window.frame().size.height - (frame.origin.y + frame.size.height / 2.0)
}

fn place(window: &NSWindow, centre_y: f64) {
    let buttons: Vec<_> = [NSWindowButton::CloseButton, NSWindowButton::MiniaturizeButton, NSWindowButton::ZoomButton]
        .into_iter()
        .filter_map(|b| window.standardWindowButton(b))
        .collect();
    let Some(close) = buttons.first() else { return };
    let off = centre_y - centre_from_top(window, close);
    if !off.is_finite() || off.abs() < 0.25 {
        return;
    }
    // The title-bar container is the frame view's child that holds the buttons; the frame view is
    // what the buttons and the content view share. It is only as tall as the standard title bar,
    // so the lights would be cut off (and miss clicks) below it: make it reach past them, with as
    // much room below them as above.
    let Some(content) = window.contentView() else { return };
    let Some(frame_view) = close.ancestorSharedWithView(&content) else { return };
    let Some(container) = frame_view.subviews().to_vec().into_iter().find(|v| close.isDescendantOf(v)) else {
        return;
    };
    let light = close.frame().size.height;
    let height = (2.0 * centre_y).max(centre_y + light / 2.0 + 2.0);
    let mut bar = container.frame();
    if bar.size.height < height {
        // The frame view counts up from the bottom: keep the container at the top of the window.
        bar.origin.y += bar.size.height - height;
        bar.size.height = height;
        container.setFrame(bar);
        container.layoutSubtreeIfNeeded();
    }
    // Growing the container may have moved the buttons with it: measure again, then move them.
    let off = centre_y - centre_from_top(window, close);
    if off.abs() < 0.25 {
        return;
    }
    for button in &buttons {
        shift(button, off);
    }
    // The buttons' parent may count down from the top (a flipped view): then the move went the
    // wrong way, so go back and the other way.
    let after = centre_y - centre_from_top(window, close);
    if after.abs() > off.abs() {
        for button in &buttons {
            shift(button, -2.0 * off);
        }
    }
}

/// Moves a button down by `points` in an unflipped parent (up in a flipped one).
fn shift(button: &NSView, points: f64) {
    let mut origin = button.frame().origin;
    origin.y -= points;
    button.setFrameOrigin(origin);
}
