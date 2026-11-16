//! Which display shows the slides and which shows Presenter View (#433).
//!
//! Like PowerPoint's automatic choice: the presenter keeps the display the editor window is on
//! (failing that, the main display), and the slides fill another one. Swap Displays trades them.

use crate::DisplayList;

/// The displays a show uses, as indices into winit's `available_monitors()`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placement {
    pub audience: usize,
    pub presenter: usize,
    /// The displays' areas in desktop points, when the host lists them.
    pub audience_area: Option<[f32; 4]>,
    pub presenter_area: Option<[f32; 4]>,
}

/// Where a staged move of the show windows is (macOS only). AppKit enters full screen on the
/// display a window is on, so a window is moved first and enters full screen once it is there; a
/// swap leaves full screen first. Each stage waits for the window, with a time limit.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum Staging {
    #[default]
    Done,
    /// Leaving full screen for a swap (since `since`); then move the windows.
    Leave { since: f64 },
    /// Moving to the displays (since `since`); then enter full screen.
    Enter { since: f64 },
}

/// The point (in desktop points) to put a window's corner on a display: inside it, clear of the
/// menu bar.
pub(crate) fn inside(area: [f32; 4]) -> egui::Pos2 {
    egui::pos2(area[0] + area[2].min(120.0) * 0.5, area[1] + area[3].min(120.0) * 0.5)
}

/// Presenter View's corner and inner size (desktop points) filling `area` below the menu bar. A
/// plain frame, not macOS zoom: unzooming animates back to the old frame and undoes a move.
pub(crate) fn presenter_frame(area: [f32; 4]) -> (egui::Pos2, egui::Vec2) {
    const MENU_BAR: f32 = 40.0;
    const TITLE_BAR: f32 = 30.0;
    (egui::pos2(area[0], area[1] + MENU_BAR), egui::vec2(area[2], (area[3] - MENU_BAR - TITLE_BAR).max(300.0)))
}

/// `(audience, presenter)` display indices for a host that places the windows itself (the
/// browser's Window Management API): the same choice as the desktop show.
pub fn place_show(list: &DisplayList) -> Option<(usize, usize)> {
    Placement::choose(list).map(|p| (p.audience, p.presenter))
}

impl Placement {
    /// `None` with fewer than two distinct displays: the show then goes full screen wherever its
    /// window is, as before.
    pub fn choose(list: &DisplayList) -> Option<Self> {
        let mut seen = Vec::with_capacity(list.displays.len());
        for &d in &list.displays {
            if !seen.contains(&d) {
                seen.push(d);
            }
        }
        if seen.len() < 2 {
            return None;
        }
        let listed = |d: Option<usize>| d.filter(|d| seen.contains(d));
        let presenter = listed(list.here).or_else(|| listed(list.primary)).unwrap_or(seen[0]);
        // Prefer a display that is not the main one for the slides (the projector), then any other.
        let audience =
            seen.iter().copied().find(|&d| d != presenter && Some(d) != list.primary).or_else(|| seen.iter().copied().find(|&d| d != presenter))?;
        let area = |d: usize| list.areas.get(d).copied();
        Some(Placement { audience, presenter, audience_area: area(audience), presenter_area: area(presenter) })
    }

    pub fn swapped(self) -> Self {
        Placement { audience: self.presenter, presenter: self.audience, audience_area: self.presenter_area, presenter_area: self.audience_area }
    }

    /// Whether the windows move in stages: on macOS, when the display areas are known.
    pub fn staged(&self) -> bool {
        cfg!(target_os = "macos") && self.audience_area.is_some() && self.presenter_area.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(displays: &[usize], here: Option<usize>, primary: Option<usize>) -> DisplayList {
        DisplayList { displays: displays.to_vec(), here, primary, ..Default::default() }
    }

    #[test]
    fn one_display_has_no_placement() {
        assert_eq!(Placement::choose(&list(&[], None, None)), None);
        assert_eq!(Placement::choose(&list(&[0], Some(0), Some(0))), None);
        assert_eq!(Placement::choose(&list(&[0, 0], None, None)), None, "a display listed twice is one display");
    }

    #[test]
    fn presenter_stays_where_the_editor_is() {
        assert_eq!(Placement::choose(&list(&[0, 1], Some(0), Some(0))), Some(at(1, 0)));
        // The editor was moved to the projector: the presenter follows it, the slides take the laptop.
        assert_eq!(Placement::choose(&list(&[0, 1], Some(1), Some(0))), Some(at(0, 1)));
    }

    #[test]
    fn without_the_editor_display_the_main_display_presents() {
        assert_eq!(Placement::choose(&list(&[0, 1], None, Some(1))), Some(at(0, 1)));
        assert_eq!(Placement::choose(&list(&[0, 1], Some(7), None)), Some(at(1, 0)), "an unlisted display is ignored");
    }

    #[test]
    fn slides_avoid_the_main_display_with_three() {
        // Editor on an external monitor, laptop is main, projector third: slides go to the projector.
        assert_eq!(Placement::choose(&list(&[0, 1, 2], Some(1), Some(0))), Some(at(2, 1)));
    }

    fn at(audience: usize, presenter: usize) -> Placement {
        Placement { audience, presenter, audience_area: None, presenter_area: None }
    }

    #[test]
    fn swap_trades_displays_and_areas() {
        let list = DisplayList {
            displays: vec![0, 1],
            here: Some(0),
            primary: Some(0),
            areas: vec![[0.0, 0.0, 1792.0, 1120.0], [1792.0, 0.0, 1920.0, 1080.0]],
        };
        let p = Placement::choose(&list).unwrap();
        assert_eq!(p.audience_area, Some([1792.0, 0.0, 1920.0, 1080.0]));
        assert_eq!(p.presenter_area, Some([0.0, 0.0, 1792.0, 1120.0]));
        let s = p.swapped();
        assert_eq!((s.audience, s.presenter), (0, 1));
        assert_eq!(s.audience_area, p.presenter_area);
        assert_eq!(s.swapped(), p);
        assert_eq!(p.staged(), cfg!(target_os = "macos"));
        assert!(!at(1, 0).staged(), "no areas: one step");
    }

    #[test]
    fn a_window_goes_inside_the_display_below_the_menu_bar() {
        let p = inside([1792.0, -200.0, 1920.0, 1080.0]);
        assert_eq!(p, egui::pos2(1852.0, -140.0));
        let (at, size) = presenter_frame([-1920.0, 0.0, 1920.0, 1080.0]);
        assert_eq!((at, size), (egui::pos2(-1920.0, 40.0), egui::vec2(1920.0, 1010.0)));
        assert_eq!(presenter_frame([0.0, 0.0, 800.0, 200.0]).1.y, 300.0, "never smaller than a usable window");
    }
}
