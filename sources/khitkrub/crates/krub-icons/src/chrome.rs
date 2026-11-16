//! The heights of the band at the top of every KrubOffice app, the same in all three:
//!
//! | part | height |
//! |---|---|
//! | title bar | [`TITLE_H`] (macOS: [`MAC_TITLE_H`], the traffic lights centred in it) |
//! | tab strip | [`TAB_STRIP_H`] |
//! | ribbon (the buttons) | [`RIBBON_H`]: [`CONTENT_H`] of buttons and [`RIBBON_PAD`] above and below |
//! | group names (optional) | [`GROUP_NAMES_H`] more |
//!
//! So the tab strip and ribbon together are [`TABS_AND_RIBBON_H`] (100 points) in every app.

/// The title bar (Windows, Linux and the web).
pub const TITLE_H: f32 = 38.0;
/// The title bar on macOS: the traffic lights are centred in it (16 points from the top of the
/// window on current macOS).
pub const MAC_TITLE_H: f32 = 32.0;
/// The tab strip under the title bar.
pub const TAB_STRIP_H: f32 = 30.0;
/// The buttons' area: two 28-point rows and the gap between them, or a large button.
pub const CONTENT_H: f32 = 60.0;
/// Space above and below the buttons.
pub const RIBBON_PAD: f32 = 5.0;
/// The ribbon under the tab strip.
pub const RIBBON_H: f32 = CONTENT_H + 2.0 * RIBBON_PAD;
/// The row of group names under the buttons, where an app offers it.
pub const GROUP_NAMES_H: f32 = 18.0;
/// The tab strip and the ribbon together.
pub const TABS_AND_RIBBON_H: f32 = TAB_STRIP_H + RIBBON_H;

const _: () = assert!(TABS_AND_RIBBON_H == 100.0, "the KrubOffice ribbon is 100 points high");
