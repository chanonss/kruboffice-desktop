//! Choosing a ribbon tab from the keyboard (owner, 2026-11-19; `krub_icons::tab_keys`, the same
//! in the three KrubOffice apps): tap Alt (Option on macOS) on its own, or press F10, and the tabs'
//! numbers turn gold; then press a tab's number (0 for the tenth), or ← → to step between tabs.
//! ↓, Esc, another key or a click ends it.
//!
//! PhutKrub has no keytips for the ribbon's buttons, so choosing a tab is all this does. Alt held
//! with a number (the Quick Access Toolbar's Alt+1…9) is not a tap and keeps working.

use egui::{Event, Key, os::OperatingSystem};

/// What a key did while choosing a tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    /// The tab with this number: 1 the first after File, 10 the tenth.
    Tab(usize),
    /// ← (false) or → (true): the tab before or after the current one.
    Step(bool),
}

#[derive(Debug, Default)]
pub struct TabKeys {
    /// The tabs' numbers are lit and wait for a key.
    pub choosing: bool,
    /// Alt went down with nothing else since: its release is a tap.
    armed: bool,
}

impl TabKeys {
    /// Read this frame's keys, take the ones it handles out of the input and return the picks.
    pub fn process(&mut self, ctx: &egui::Context) -> Vec<Pick> {
        let mac = ctx.os() == OperatingSystem::Mac;
        let events = ctx.input(|i| i.events.clone());
        let mut picks = Vec::new();
        let mut consumed = vec![false; events.len()];
        let mut tap = false;
        let mut eat_text = false;
        for (i, event) in events.iter().enumerate() {
            match event {
                // Left Alt everywhere; right Option on macOS too (right Alt is AltGr elsewhere).
                Event::Key { key, pressed, repeat, modifiers, .. }
                    if (*key == Key::AltLeft || (mac && *key == Key::AltRight)) && !modifiers.command && !modifiers.ctrl && !modifiers.shift =>
                {
                    if *pressed {
                        if !repeat {
                            self.armed = true;
                        }
                    } else if std::mem::take(&mut self.armed) {
                        tap = true;
                    }
                }
                Event::Key { key, pressed: true, modifiers, .. } => {
                    self.armed = false;
                    if *key == Key::F10 && modifiers.is_none() {
                        self.choosing = !self.choosing;
                        consumed[i] = true;
                    } else if self.choosing {
                        consumed[i] = true;
                        eat_text = true;
                        match *key {
                            Key::ArrowLeft => picks.push(Pick::Step(false)),
                            Key::ArrowRight => picks.push(Pick::Step(true)),
                            Key::Escape | Key::ArrowDown => self.choosing = false,
                            k => {
                                self.choosing = false;
                                match krub_icons::tab_keys::tab_number(k) {
                                    Some(n) => picks.push(Pick::Tab(n)),
                                    // Any other key ends choosing and does its own work.
                                    None => {
                                        consumed[i] = false;
                                        eat_text = false;
                                    }
                                }
                            }
                        }
                    }
                }
                // The digit typed with the number key never reaches the slide.
                Event::Text(_) if self.choosing || eat_text => consumed[i] = true,
                Event::PointerButton { pressed: true, .. } => {
                    self.choosing = false;
                    self.armed = false;
                }
                _ => {}
            }
        }
        // macOS: an Option tap counts only if no text came with it (an accent being composed).
        if tap && !(mac && events.iter().any(|e| matches!(e, Event::Text(_) | Event::Ime(_)))) {
            self.choosing = !self.choosing;
        }
        if consumed.iter().any(|c| *c) {
            ctx.input_mut(|i| {
                i.events = events.into_iter().zip(consumed).filter_map(|(e, c)| (!c).then_some(e)).collect();
            });
        }
        picks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: Key, pressed: bool, modifiers: egui::Modifiers) -> Event {
        Event::Key { key, physical_key: None, pressed, repeat: false, modifiers }
    }

    fn frame(ctx: &egui::Context, tk: &mut TabKeys, events: Vec<Event>) -> (Vec<Pick>, Vec<Event>) {
        let mut picks = Vec::new();
        let mut left = Vec::new();
        let mut out = ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| {
            picks = tk.process(ui.ctx());
            left = ui.input(|i| i.events.clone());
        });
        out.textures_delta.clear();
        (picks, left)
    }

    #[test]
    fn a_tap_then_a_number_picks_the_tab_and_types_nothing() {
        let ctx = egui::Context::default();
        let mut tk = TabKeys::default();
        let alt = egui::Modifiers { alt: true, ..Default::default() };
        frame(&ctx, &mut tk, vec![key(Key::AltLeft, true, alt)]);
        frame(&ctx, &mut tk, vec![key(Key::AltLeft, false, egui::Modifiers::NONE)]);
        assert!(tk.choosing, "a tap lights the numbers");
        let (picks, left) = frame(&ctx, &mut tk, vec![key(Key::Num3, true, egui::Modifiers::NONE), Event::Text("3".into())]);
        assert_eq!(picks, [Pick::Tab(3)]);
        assert!(left.is_empty(), "neither the key nor its digit reaches the slide: {left:?}");
        assert!(!tk.choosing);
    }

    #[test]
    fn arrows_step_and_alt_with_a_number_is_not_a_tap() {
        let ctx = egui::Context::default();
        let mut tk = TabKeys::default();
        frame(&ctx, &mut tk, vec![key(Key::F10, true, egui::Modifiers::NONE)]);
        assert!(tk.choosing);
        let (picks, _) = frame(&ctx, &mut tk, vec![key(Key::ArrowRight, true, egui::Modifiers::NONE)]);
        assert_eq!(picks, [Pick::Step(true)]);
        assert!(tk.choosing, "still choosing after a step");
        frame(&ctx, &mut tk, vec![key(Key::Escape, true, egui::Modifiers::NONE)]);
        assert!(!tk.choosing);
        // Alt+1 (the toolbar's first button): Alt down, 1, Alt up is a chord, not a tap.
        let alt = egui::Modifiers { alt: true, ..Default::default() };
        let (_, left) =
            frame(&ctx, &mut tk, vec![key(Key::AltLeft, true, alt), key(Key::Num1, true, alt), key(Key::AltLeft, false, egui::Modifiers::NONE)]);
        assert!(!tk.choosing, "a chord lights nothing");
        assert!(left.iter().any(|e| matches!(e, Event::Key { key: Key::Num1, .. })), "and Alt+1 still reaches the toolbar");
    }
}
