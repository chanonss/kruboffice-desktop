//! Switching ribbon tabs from the keyboard, the same in every KrubOffice app (owner, 2026-11-19):
//! tap Alt (Option on macOS) on its own, or press F10, then the number printed on the tab: 1–9,
//! and 0 for the tenth. While the numbers show, ← and → step to the tab before or after and ↓
//! goes into the tab's commands; Esc or a click cancels.
//!
//! The Quick Access Toolbar's keytips, which used the digits (as in Office), become Q then the
//! number ([`qat_label`]), and the user icon (where Search lives) takes U ([`USER_LETTER`]).
//!
//! A tap is a press and release of Alt with nothing else in between, so Alt or ⌘/Ctrl with a
//! number (headings, line spacing, Format Cells…) and Option with a letter (accents on macOS)
//! keep working. The apps run different egui versions, so this uses only what they share.

/// The keytip that opens Search, on the user icon at the bar's right end.
pub const USER_LETTER: &str = "U";

/// The Quick Access Toolbar keytips' first letter.
pub const QAT_LETTER: &str = "Q";

/// The tab a number key picks: `Num1`…`Num9` the first nine numbered tabs, `Num0` the tenth.
/// Tabs are numbered from 1 after File, as the badges on them show.
pub fn tab_number(key: egui::Key) -> Option<usize> {
    use egui::Key::*;
    Some(match key {
        Num1 => 1,
        Num2 => 2,
        Num3 => 3,
        Num4 => 4,
        Num5 => 5,
        Num6 => 6,
        Num7 => 7,
        Num8 => 8,
        Num9 => 9,
        Num0 => 10,
        _ => return None,
    })
}

/// The badge a numbered tab shows while choosing a tab: its number, "0" for the tenth. Tabs past
/// the tenth have no number (they keep their letter, where they have one).
pub fn tab_badge(number: usize) -> Option<String> {
    match number {
        1..=9 => Some(number.to_string()),
        10 => Some("0".to_string()),
        _ => None,
    }
}

/// The Quick Access Toolbar keytip of the `index`th button among `count`: Q1–Q9, then Q01, Q02…
/// with equally wide numbers, so no badge is the start of another.
pub fn qat_label(index: usize, count: usize) -> String {
    if index < 9 {
        format!("{QAT_LETTER}{}", index + 1)
    } else {
        let width = count.saturating_sub(9).to_string().len();
        format!("{QAT_LETTER}0{:0width$}", index.saturating_sub(8))
    }
}

/// Where ← and → move from tab `current` among `count` numbered tabs (1-based), wrapping round.
pub fn step(current: usize, count: usize, forward: bool) -> usize {
    if count == 0 {
        return 0;
    }
    let current = current.clamp(1, count);
    if forward {
        if current == count { 1 } else { current + 1 }
    } else if current == 1 {
        count
    } else {
        current - 1
    }
}

/// Whether a press of Alt and its release, with `other_key` pressed in between, is a tap that
/// starts (or stops) choosing a tab. Every platform, Option on macOS too: a tap alone types
/// nothing, while Option with a letter is not a tap.
pub fn is_tap(released: bool, other_key: bool) -> bool {
    released && !other_key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_pick_tabs_and_zero_is_the_tenth() {
        assert_eq!(tab_number(egui::Key::Num1), Some(1));
        assert_eq!(tab_number(egui::Key::Num9), Some(9));
        assert_eq!(tab_number(egui::Key::Num0), Some(10));
        assert_eq!(tab_number(egui::Key::A), None);
        assert_eq!(tab_badge(3).as_deref(), Some("3"));
        assert_eq!(tab_badge(10).as_deref(), Some("0"));
        assert_eq!(tab_badge(11), None);
        assert_eq!(tab_badge(0), None, "File has no number");
    }

    #[test]
    fn toolbar_badges_start_with_q_and_never_prefix_each_other() {
        let labels: Vec<String> = (0..15).map(|i| qat_label(i, 15)).collect();
        assert_eq!(labels[0], "Q1");
        assert_eq!(labels[8], "Q9");
        assert_eq!(labels[9], "Q01");
        for (i, a) in labels.iter().enumerate() {
            for (j, b) in labels.iter().enumerate() {
                assert!(i == j || !b.starts_with(a.as_str()), "{a} starts {b}");
            }
        }
        assert_ne!(USER_LETTER, QAT_LETTER);
    }

    #[test]
    fn arrows_wrap_round_the_tabs() {
        assert_eq!(step(1, 7, true), 2);
        assert_eq!(step(7, 7, true), 1);
        assert_eq!(step(1, 7, false), 7);
        assert_eq!(step(0, 7, false), 7, "from File the left arrow goes to the last tab");
        assert_eq!(step(3, 0, true), 0);
        assert!(is_tap(true, false));
        assert!(!is_tap(true, true), "Option with a letter types an accent");
    }
}
