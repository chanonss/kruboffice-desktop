//! The "Enter formatting for number" box of Define New Multilevel List. A list level's number text
//! is kept as the file format has it (`%1.%2.`: `%` and a level stands for that level's number), but
//! the box shows each level's number as a grey chip with its sample number (`1`, `a`, `๑`), as Word
//! does, so nobody has to know about `%`. Typing adds plain text; a chip is added from Include level
//! number from, and Backspace or Delete takes a whole chip away.

use egui::{Event, Key, Response, Sense, Ui, vec2};

/// One piece of a level's number text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    /// A character shown as typed.
    Text(char),
    /// The number of a level, 1–9.
    Level(u8),
}

/// The pieces of `text`: `%` and a digit 1–9 is that level's number; anything else is text.
pub fn parse(text: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%'
            && let Some(d) = chars.peek().and_then(|d| d.to_digit(10)).filter(|d| (1..=9).contains(d))
        {
            chars.next();
            out.push(Piece::Level(d as u8));
        } else {
            out.push(Piece::Text(c));
        }
    }
    out
}

/// `pieces` back as the file format's text (`%1.%2.`).
pub fn to_text(pieces: &[Piece]) -> String {
    pieces
        .iter()
        .map(|p| match p {
            Piece::Text(c) => c.to_string(),
            Piece::Level(l) => format!("%{l}"),
        })
        .collect()
}

/// A stretch of the box: plain text laid out (and shaped) as one, or a level's number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Run {
    /// The text of `pieces[start..start + text.chars().count()]`.
    Text { start: usize, text: String },
    /// The level number at `pieces[at]`.
    Level { at: usize, level: u8 },
}

/// `pieces` as runs: each stretch of text between level numbers is one run, so a Thai consonant
/// and its vowel and tone marks are shaped together (one character at a time they float, each on
/// a dotted circle).
pub fn runs(pieces: &[Piece]) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    for (i, p) in pieces.iter().enumerate() {
        match p {
            Piece::Level(l) => out.push(Run::Level { at: i, level: *l }),
            Piece::Text(c) => match out.last_mut() {
                Some(Run::Text { text, .. }) => text.push(*c),
                _ => out.push(Run::Text { start: i, text: c.to_string() }),
            },
        }
    }
    out
}

/// Where the cursor may stop, as indices into `pieces`: before and after every level number, and
/// between the clusters of text ([`wordcraft_doc::cluster`], as in the document), never between a
/// consonant and its marks. Sorted, from 0 to `pieces.len()`.
pub fn stops(pieces: &[Piece]) -> Vec<usize> {
    let mut out = vec![0];
    for run in runs(pieces) {
        match run {
            Run::Level { at, .. } => out.extend([at, at + 1]),
            Run::Text { start, text } => {
                for b in wordcraft_doc::cluster::cluster_starts(&text) {
                    out.push(start + text.get(..b).map_or(0, |t| t.chars().count()));
                }
                out.push(start + text.chars().count());
            }
        }
    }
    out.push(pieces.len());
    out.sort_unstable();
    out.dedup();
    out
}

/// `cursor` moved back to the stop at or before it.
fn snap(pieces: &[Piece], cursor: usize) -> usize {
    stops(pieces).into_iter().rfind(|s| *s <= cursor).unwrap_or(0)
}

/// Type `s` at `cursor`. `%` is left out: followed by a digit it would turn the text into a level's
/// number, which only Include level number from adds. Line breaks and other controls too.
pub fn type_text(pieces: &mut Vec<Piece>, cursor: &mut usize, s: &str) -> bool {
    let mut changed = false;
    for c in s.chars().filter(|c| *c != '%' && !c.is_control()) {
        let at = (*cursor).min(pieces.len());
        pieces.insert(at, Piece::Text(c));
        *cursor = at + 1;
        changed = true;
    }
    changed
}

/// Put level `level`'s number at `cursor`.
pub fn insert_level(pieces: &mut Vec<Piece>, cursor: &mut usize, level: u8) {
    let at = snap(pieces, *cursor);
    pieces.insert(at, Piece::Level(level.clamp(1, 9)));
    *cursor = at + 1;
}

/// One key: Backspace and Delete take away the cluster before or after the cursor (a whole chip;
/// Backspace after a Thai or other Brahmic letter takes only its last character, a tone mark or
/// vowel, as in the document); the arrows, Home and End move it a cluster at a time. Whether the
/// text changed.
pub fn key(pieces: &mut Vec<Piece>, cursor: &mut usize, k: Key) -> bool {
    *cursor = snap(pieces, (*cursor).min(pieces.len()));
    let stops = stops(pieces);
    let prev = stops.iter().rev().find(|s| **s < *cursor).copied();
    let next = stops.iter().find(|s| **s > *cursor).copied();
    match k {
        Key::Backspace => {
            let Some(p) = prev else { return false };
            let from = match cursor.checked_sub(1).and_then(|i| pieces.get(i)) {
                Some(Piece::Text(c)) if wordcraft_doc::cluster::backspaces_by_char(*c) => *cursor - 1,
                _ => p,
            };
            pieces.drain(from..*cursor);
            *cursor = from;
            true
        }
        Key::Delete => {
            let Some(n) = next else { return false };
            pieces.drain(*cursor..n);
            true
        }
        Key::ArrowLeft => {
            *cursor = prev.unwrap_or(0);
            false
        }
        Key::ArrowRight => {
            *cursor = next.unwrap_or(*cursor);
            false
        }
        Key::Home => {
            *cursor = 0;
            false
        }
        Key::End => {
            *cursor = pieces.len();
            false
        }
        _ => false,
    }
}

fn cursor_id(id: egui::Id) -> egui::Id {
    id.with("cursor")
}

/// Put level `level`'s number where the cursor of the box `id` is (at the end if it has none yet).
pub fn insert_level_at_cursor(ui: &Ui, id: egui::Id, text: &mut String, level: u8) {
    let mut pieces = parse(text);
    let mut cursor = ui.data(|d| d.get_temp::<usize>(cursor_id(id))).unwrap_or(pieces.len()).min(pieces.len());
    insert_level(&mut pieces, &mut cursor, level);
    ui.data_mut(|d| d.insert_temp(cursor_id(id), cursor));
    *text = to_text(&pieces);
}

/// The box: `text` with level numbers as grey chips labelled by `chip` (a level's sample number).
/// `id` names it, so Include level number from can add a chip where the cursor is.
pub fn show(ui: &mut Ui, id: egui::Id, label: &str, text: &mut String, width: f32, chip: impl Fn(u8) -> String) -> Response {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let height = ui.spacing().interact_size.y.max(font.size + 8.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let mut resp = ui.interact(rect, id, Sense::click());
    // Read aloud as the number it makes (`1.1.`), named by `label`.
    let shown: String = parse(text)
        .iter()
        .map(|p| match p {
            Piece::Text(c) => c.to_string(),
            Piece::Level(l) => chip(*l),
        })
        .collect();
    resp.widget_info(|| {
        let mut info = egui::WidgetInfo::text_edit(ui.is_enabled(), shown.clone(), shown.clone(), "");
        info.label = Some(label.to_string());
        info
    });
    if resp.clicked() {
        resp.request_focus();
    }
    let focused = resp.has_focus();
    if focused {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(id, egui::EventFilter { horizontal_arrows: true, ..Default::default() });
        });
    }
    let mut pieces = parse(text);
    let stored = ui.data(|d| d.get_temp::<usize>(cursor_id(id))).unwrap_or(pieces.len());
    let mut cursor = snap(&pieces, stored.min(pieces.len()));

    // Lay the runs out: each stretch of text as one galley, so its clusters are shaped whole;
    // then the x of every cursor stop.
    let pad = 6.0;
    let gap = 2.0;
    let ink = ui.visuals().text_color();
    let laid: Vec<(Run, std::sync::Arc<egui::Galley>, f32)> = {
        let mut x = rect.left() + pad;
        runs(&pieces)
            .into_iter()
            .map(|r| {
                let g = match &r {
                    Run::Level { level, .. } => ui.painter().layout_no_wrap(chip(*level), font.clone(), ink),
                    Run::Text { text, .. } => ui.painter().layout_no_wrap(text.clone(), font.clone(), ink),
                };
                let x0 = x;
                x += match &r {
                    Run::Level { .. } => g.size().x + 8.0 + gap,
                    Run::Text { .. } => g.size().x,
                };
                (r, g, x0)
            })
            .collect()
    };
    let end_x = laid.last().map_or(rect.left() + pad, |(r, g, x0)| match r {
        Run::Level { .. } => x0 + g.size().x + 8.0 + gap,
        Run::Text { .. } => x0 + g.size().x,
    });
    let x_of = |stop: usize| -> f32 {
        for (r, g, x0) in &laid {
            match r {
                Run::Level { at, .. } if *at == stop => return *x0,
                Run::Text { start, text } if stop >= *start && stop < start + text.chars().count() => {
                    return x0 + g.pos_from_cursor(egui::text::CCursor::new(stop - start)).min.x;
                }
                _ => {}
            }
        }
        end_x
    };
    let xs: Vec<(usize, f32)> = stops(&pieces).into_iter().map(|s| (s, x_of(s))).collect();

    // A click puts the cursor at the nearest stop.
    if resp.clicked()
        && let Some(at) = resp.interact_pointer_pos()
    {
        cursor = xs.iter().min_by(|a, b| (a.1 - at.x).abs().total_cmp(&(b.1 - at.x).abs())).map_or(pieces.len(), |(s, _)| *s);
    }

    // Keys and typing while focused.
    let mut changed = false;
    if focused {
        for ev in ui.input(|i| i.events.clone()) {
            match ev {
                Event::Text(s) | Event::Paste(s) => changed |= type_text(&mut pieces, &mut cursor, &s),
                Event::Ime(egui::ImeEvent::Commit(s)) => changed |= type_text(&mut pieces, &mut cursor, &s),
                Event::Key { key: k, pressed: true, modifiers, .. } if !modifiers.command => changed |= key(&mut pieces, &mut cursor, k),
                _ => {}
            }
        }
    }
    ui.data_mut(|d| d.insert_temp(cursor_id(id), cursor));
    if changed {
        *text = to_text(&pieces);
        resp.mark_changed();
        // Draw what was just typed this frame.
        ui.ctx().request_repaint();
    }

    // Paint: the box, the text, the chips, the cursor.
    let v = ui.visuals();
    let stroke = if focused { v.selection.stroke } else { v.widgets.inactive.bg_stroke };
    let p = ui.painter_at(rect.expand(1.0));
    p.rect(rect, v.widgets.inactive.corner_radius, v.text_edit_bg_color(), stroke, egui::StrokeKind::Inside);
    if !changed {
        let mid = rect.center().y;
        let chip_fill = if v.dark_mode { egui::Color32::from_gray(80) } else { egui::Color32::from_gray(218) };
        for (r, g, x0) in &laid {
            match r {
                Run::Level { .. } => {
                    let r = egui::Rect::from_min_size(egui::pos2(*x0, mid - g.size().y / 2.0 - 1.0), vec2(g.size().x + 8.0, g.size().y + 2.0));
                    p.rect_filled(r, 3.0, chip_fill);
                    p.galley(egui::pos2(x0 + 4.0, mid - g.size().y / 2.0), g.clone(), ink);
                }
                Run::Text { .. } => p.galley(egui::pos2(*x0, mid - g.size().y / 2.0), g.clone(), ink),
            }
        }
        if focused && let Some((_, cx)) = xs.iter().rfind(|(s, _)| *s <= cursor) {
            let h = font.size + 2.0;
            p.line_segment([egui::pos2(*cx, mid - h / 2.0), egui::pos2(*cx, mid + h / 2.0)], egui::Stroke::new(1.5, ink));
        }
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_with_level_numbers_round_trips() {
        for t in ["%1.", "%1.%2.", "ข้อ %1", "(%1)", "100%", "%0x", "%%1", ""] {
            assert_eq!(to_text(&parse(t)), t, "{t}");
        }
        assert_eq!(parse("%1.%2."), [Piece::Level(1), Piece::Text('.'), Piece::Level(2), Piece::Text('.')]);
        // A % not followed by a level stays text.
        assert_eq!(parse("5%"), [Piece::Text('5'), Piece::Text('%')]);
    }

    /// Typing adds text where the cursor is but never a `%`; Backspace takes a whole level number
    /// at once; a level's number goes in at the cursor.
    #[test]
    fn editing_treats_a_level_number_as_one_piece() {
        let mut p = parse("%1.%2.");
        let mut c = p.len();
        assert!(key(&mut p, &mut c, Key::Backspace));
        assert!(key(&mut p, &mut c, Key::Backspace));
        assert_eq!(to_text(&p), "%1.", "Backspace took the dot, then the whole %2");
        assert!(!type_text(&mut p, &mut c, "%"), "% isn't typed");
        type_text(&mut p, &mut c, "-");
        insert_level(&mut p, &mut c, 2);
        assert_eq!(to_text(&p), "%1.-%2");
        key(&mut p, &mut c, Key::Home);
        type_text(&mut p, &mut c, "ข้อ ");
        assert_eq!(to_text(&p), "ข้อ %1.-%2");
        key(&mut p, &mut c, Key::End);
        assert!(!key(&mut p, &mut c, Key::Delete), "nothing after the end");
        let mut c = 99;
        assert!(!key(&mut p, &mut c, Key::ArrowRight));
        assert_eq!(c, p.len(), "a stale cursor is kept inside the text");
    }

    /// "บทที่ %1" (the owner's thesis chapters): the vowel and tone marks of ที่ stay with their
    /// consonant in one run, the space is kept, and the text goes back to the file unchanged.
    #[test]
    fn thai_marks_and_the_space_stay_in_one_run() {
        let p = parse("บทที่ %1");
        assert_eq!(to_text(&p), "บทที่ %1");
        assert_eq!(runs(&p), [Run::Text { start: 0, text: "บทที่ ".into() }, Run::Level { at: 6, level: 1 }]);
        // บ | ท | ที่ | space | chip: never between ท and ี or ี and ่.
        assert_eq!(stops(&p), [0, 1, 2, 5, 6, 7]);
        for t in ["ข้อ %1.%2", "%1 ก้", "é%1", "\u{928}\u{94D}\u{926}\u{940} %1"] {
            assert_eq!(to_text(&parse(t)), t, "{t}");
        }
    }

    /// The arrows step over a whole cluster; Backspace after Thai takes its last mark (as in the
    /// document); Delete takes the whole cluster; a stale cursor inside a cluster moves back.
    #[test]
    fn the_cursor_moves_and_deletes_by_cluster() {
        let mut p = parse("บทที่ %1");
        let mut c = 0;
        for want in [1, 2, 5, 6, 7, 7] {
            key(&mut p, &mut c, Key::ArrowRight);
            assert_eq!(c, want);
        }
        for want in [6, 5, 2] {
            key(&mut p, &mut c, Key::ArrowLeft);
            assert_eq!(c, want);
        }
        let mut c = 5;
        assert!(key(&mut p, &mut c, Key::Backspace));
        assert_eq!((to_text(&p).as_str(), c), ("บทที %1", 4), "Backspace took the tone mark");
        let mut c = 2;
        assert!(key(&mut p, &mut c, Key::Delete));
        assert_eq!((to_text(&p).as_str(), c), ("บท %1", 2), "Delete took ที whole");
        let mut c = 3;
        type_text(&mut p, &mut c, "x");
        assert_eq!(to_text(&p), "บท x%1");
        let mut p = parse("ที่%1");
        let mut c = 2;
        insert_level(&mut p, &mut c, 2);
        assert_eq!(to_text(&p), "%2ที่%1", "a chip never goes between a consonant and its marks");
        let mut c = 1;
        key(&mut p, &mut c, Key::ArrowRight);
        assert_eq!(c, 4);
        // A mark typed straight after a chip has no consonant: it is its own stop.
        let mut p = parse("%1\u{0E48}");
        assert_eq!(stops(&p), [0, 1, 2]);
        let mut c = 2;
        assert!(key(&mut p, &mut c, Key::Backspace));
        assert_eq!(to_text(&p), "%1");
    }
}
