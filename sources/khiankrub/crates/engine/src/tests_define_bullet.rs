//! Define New Bullet (#2786, ported by hand from WordCraft 83f18db1): `para.defineBullet` gives
//! the selection a list of its own, with the bullet's character, font, size, weight, colour and
//! alignment. Upstream's test, plus a Thai text check.

use serde_json::{Value, json};
use wordcraft_doc::Pos;

use crate::Session;

fn run(s: &mut Session, id: &str, v: Value) {
    s.run(id, &v).unwrap_or_else(|e| panic!("{id}: {e}"));
}

/// #2786: Define New Bullet puts the selection in a list of its own: the character, its font and
/// its alignment at the levels the paragraphs use, each paragraph keeping its level. A list that
/// already uses the same character is left alone, defining again never toggles the bullet off,
/// it survives a DOCX round trip, and bad params are errors that change nothing.
#[test]
fn define_bullet_gives_the_selection_a_bullet_of_its_own() {
    use wordcraft_doc::props::Align;
    let mut s = Session::new(wordcraft_doc::Document::from_text("One\nTwo\nThree"));
    let num_at = |s: &Session, i: usize| s.doc.para_at(&Pos::body(i, 0)).unwrap().props.numbering.unwrap();
    // "Three" already has a ♦ list, defined the ordinary way.
    s.sel = crate::Selection::caret(Pos::body(2, 0));
    run(&mut s, "para.bullets", json!({"kind": "♦"}));
    let other = num_at(&s, 2).num;
    let other_level = s.doc.numbering.level(other, 0).unwrap().clone();
    // "Two" sits at level 2 of a bulleted list before the definition.
    s.sel = crate::Selection::caret(Pos::body(1, 0));
    run(&mut s, "para.bullets", json!({}));
    run(&mut s, "para.listLevel", json!({"level": 1}));
    s.sel = crate::Selection { anchor: Pos::body(0, 0), focus: Pos::body(1, 3) };
    run(&mut s, "para.defineBullet", json!({"char": "♦", "font": "Segoe UI Symbol", "align": "center"}));
    let (one, two) = (num_at(&s, 0), num_at(&s, 1));
    assert_eq!(one.num, two.num, "one list for the selection");
    assert_ne!(one.num, other, "not the existing ♦ list");
    assert_eq!((one.level, two.level), (0, 1), "each paragraph keeps its level");
    for lvl in [0, 1] {
        let l = s.doc.numbering.level(one.num, lvl).unwrap();
        assert_eq!((l.text.as_str(), l.chr.font.as_deref(), l.align), ("♦", Some("Segoe UI Symbol"), Align::Center), "level {lvl}");
    }
    assert_eq!(s.doc.numbering.level(other, 0).unwrap(), &other_level, "the other ♦ list is unchanged");
    // Defining the same bullet again keeps it (para.bullets would toggle it off).
    run(&mut s, "para.defineBullet", json!({"char": "♦"}));
    assert_eq!(s.doc.numbering.level(num_at(&s, 0).num, 0).unwrap().text, "♦");
    // DOCX keeps the character, its font and the alignment.
    run(&mut s, "para.defineBullet", json!({"char": "✓", "font": "Segoe UI Symbol", "align": "right", "size": 14, "bold": true, "color": "C00000"}));
    let back = wordcraft_docx::read(&wordcraft_docx::write(&s.doc).unwrap()).unwrap();
    let n = back.para_at(&Pos::body(0, 0)).unwrap().props.numbering.unwrap();
    let l = back.numbering.level(n.num, 0).unwrap();
    assert_eq!((l.text.as_str(), l.chr.font.as_deref(), l.align), ("✓", Some("Segoe UI Symbol"), Align::Right));
    // The Font dialog's size, bold and colour reach the bullet too.
    assert_eq!((l.chr.size, l.chr.bold), (Some(14.0), Some(true)));
    assert_eq!(l.chr.color, wordcraft_doc::props::TextColor::parse("C00000"));
    // A paragraph numbered only by its style keeps the style's level (review of #2860).
    let mut s2 = Session::new(wordcraft_doc::Document::from_text("Styled"));
    let styled = s2.doc.numbering.try_add_list(wordcraft_doc::ListKind::Bullet).unwrap();
    s2.doc.styles.get_mut("ListParagraph").unwrap().para.numbering = Some(wordcraft_doc::props::NumRef { num: styled, level: 1 });
    s2.doc.para_mut(wordcraft_doc::StoryRef::Body, &wordcraft_doc::Path::top(0)).unwrap().props.style = Some("ListParagraph".into());
    assert!(s2.doc.para_at(&Pos::body(0, 0)).unwrap().props.numbering.is_none(), "no numbering of its own");
    s2.sel = crate::Selection::caret(Pos::body(0, 0));
    run(&mut s2, "para.defineBullet", json!({"char": "♦"}));
    let n = s2.doc.para_at(&Pos::body(0, 0)).unwrap().props.numbering.unwrap();
    assert_eq!(n.level, 1, "the style's level is kept");
    assert_eq!(s2.doc.numbering.level(n.num, 1).unwrap().text, "♦");
    // No params: ★, as before.
    s.sel = crate::Selection::caret(Pos::body(2, 0));
    run(&mut s, "para.defineBullet", json!({}));
    assert_eq!(s.doc.numbering.level(num_at(&s, 2).num, 0).unwrap().text, "★");
    let doc = s.doc.clone();
    let long = "x".repeat(200);
    for junk in [
        json!({"char": ""}),
        json!({"char": "ab"}),
        json!({"char": 5}),
        json!({"char": "\u{7}"}),
        json!({"char": "•", "align": "middle"}),
        json!({"char": "•", "align": 1}),
        json!({"char": "•", "font": 3}),
        json!({"char": "•", "font": long}),
        json!({"char": "•", "size": 0}),
        json!({"char": "•", "size": "big"}),
        json!({"char": "•", "bold": "yes"}),
        json!({"char": "•", "color": "purple"}),
    ] {
        assert!(s.run("para.defineBullet", &junk).is_err(), "{junk}");
    }
    assert_eq!(s.doc, doc);
}

/// A Thai list keeps its text: the bullet is the list's, never typed into the paragraph.
#[test]
fn a_thai_list_takes_the_new_bullet_and_keeps_its_text() {
    let mut s = Session::new(wordcraft_doc::Document::from_text("สวัสดีครับ\nขอบคุณครับ"));
    s.sel = crate::Selection { anchor: Pos::body(0, 0), focus: Pos::body(1, 2) };
    run(&mut s, "para.defineBullet", json!({"char": "❖", "size": 16, "color": "0F6E56"}));
    for (i, text) in ["สวัสดีครับ", "ขอบคุณครับ"].into_iter().enumerate() {
        let p = s.doc.para_at(&Pos::body(i, 0)).unwrap();
        assert_eq!(p.text, text);
        let n = p.props.numbering.unwrap();
        let l = s.doc.numbering.level(n.num, n.level).unwrap();
        assert_eq!((l.text.as_str(), l.chr.size), ("❖", Some(16.0)));
    }
}
