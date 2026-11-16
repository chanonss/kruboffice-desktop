//! Fonts embedded when a presentation is saved as PowerPoint (File › Options › Save › Embed fonts
//! in the file, [`Presentation::embed_fonts`]).
//!
//! Every slide is laid out as it is drawn and each face its text is drawn in is considered: a
//! face is embedded under the font name the text asks for when it *is* that font (a substitute
//! drawn for a font that isn't installed, or a fallback for characters the font lacks, is not
//! embedded under a name it doesn't have). With "Embed only the characters used" (the default)
//! only the glyphs drawn are kept. Each face's own licence decides the rest
//! (`deckcraft_fonts::embed`). Fonts the file embedded that can't be made again here (a
//! MicroType Express subset, which can't be loaded) are kept while the slides still name them.
//! With the option off, saving drops the embedded fonts. Faces that can't be embedded are
//! reported once, after the save ([`take_report`]).

use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use deckcraft_fonts::FontFace;
use deckcraft_model::{EmbeddedFont, Presentation, ShapeKind};

/// Most faces embedded in one presentation (the reader takes as many).
const MAX_FACES: usize = 64;
/// Most bytes of fonts embedded in one presentation.
const MAX_FONT_BYTES: usize = 256 * 1024 * 1024;
/// Most font names gathered from the presentation.
const MAX_NAMES: usize = 4096;

/// What the last save left out (font, why), for the status line.
static REPORT: Mutex<Vec<(String, &'static str)>> = Mutex::new(Vec::new());

/// A face to embed: the name the text asks for, its slot and the font part's bytes.
pub struct Embed {
    pub family: String,
    pub bold: bool,
    pub italic: bool,
    /// An Embedded OpenType file.
    pub data: Vec<u8>,
}

/// The font names the presentation's text asks for: theme fonts, and the Latin, East Asian and
/// complex-script fonts its runs name (theme references resolved).
fn named_fonts(p: &Presentation) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |f: &str| {
        let f = f.trim();
        if !f.is_empty() && !f.starts_with('+') && out.len() < MAX_NAMES && !out.iter().any(|o| o.eq_ignore_ascii_case(f)) {
            out.push(f.to_string());
        }
    };
    let masters = p.masters.iter().chain(p.notes_master.iter()).chain(p.handout_master.iter());
    for m in masters {
        for set in [&m.theme.fonts.major, &m.theme.fonts.minor] {
            for f in [&set.latin, &set.ea, &set.cs].into_iter().chain(set.scripts.iter().map(|s| &s.typeface)) {
                add(f);
            }
        }
    }
    for f in crate::fonts::used_fonts(p) {
        add(&f);
    }
    for slide in &p.slides {
        let theme = p.masters.first().map(|m| &m.theme);
        let mut run_fonts = |body: &deckcraft_model::text::TextBody| {
            for run in body.paragraphs.iter().flat_map(|para| &para.runs) {
                for f in [&run.props.font, &run.props.font_ea, &run.props.font_cs].into_iter().flatten() {
                    match theme {
                        Some(t) => add(&t.font(f)),
                        None => add(f),
                    }
                }
            }
        };
        deckcraft_model::walk(&slide.shapes, &mut |s, _| {
            if let Some(body) = &s.text {
                run_fonts(body);
            }
            if let ShapeKind::Table(t) = &s.kind {
                for cell in t.rows.iter().flat_map(|r| &r.cells) {
                    run_fonts(&cell.text);
                }
            }
        });
    }
    out
}

/// The faces the slides' text is drawn in, with the glyphs drawn, in the order first met.
fn used_faces(p: &Presentation) -> Vec<(Arc<FontFace>, BTreeSet<u32>)> {
    let mut at: HashMap<u32, usize> = HashMap::new();
    let mut out: Vec<(Arc<FontFace>, BTreeSet<u32>)> = Vec::new();
    for i in 0..p.slides.len() {
        let placed = deckcraft_render::place_slide(p, i);
        for run in placed.texts.iter().flat_map(|t| t.layout.runs.iter().chain(&t.layout.empty_bullets)) {
            let id = run.face.id();
            let i = match at.get(&id) {
                Some(&i) => i,
                None if out.len() < 256 => {
                    at.insert(id, out.len());
                    out.push((run.face.clone(), BTreeSet::new()));
                    out.len() - 1
                }
                None => continue,
            };
            if let Some((_, gids)) = out.get_mut(i) {
                gids.extend(run.glyphs.iter().map(|g| g.0));
            }
        }
    }
    out
}

/// The fonts to embed in `p`, and the fonts that couldn't be (name, why).
pub fn collect(p: &Presentation) -> (Vec<Embed>, Vec<(String, &'static str)>) {
    let named = named_fonts(p);
    let mut fonts: Vec<Embed> = Vec::new();
    let mut skipped: Vec<(String, &'static str)> = Vec::new();
    for (face, gids) in used_faces(p) {
        let legacy = face.legacy.as_ref().map(|l| l.0.trim());
        let Some(name) = named.iter().find(|n| n.eq_ignore_ascii_case(face.family.trim()) || legacy.is_some_and(|l| n.eq_ignore_ascii_case(l)))
        else {
            continue;
        };
        let bold = face.weight >= 600.0;
        let italic = face.italic;
        if fonts.iter().any(|f| f.family.eq_ignore_ascii_case(name) && f.bold == bold && f.italic == italic) {
            continue;
        }
        let keep = (!p.embed_all_characters).then_some(&gids);
        match deckcraft_fonts::embed::prepare(&face, keep).and_then(|ok| deckcraft_fonts::embed::eot(&ok.data)) {
            Ok(data) => fonts.push(Embed { family: name.clone(), bold, italic, data }),
            Err(why) => {
                if !skipped.iter().any(|(f, _)| f.eq_ignore_ascii_case(name)) {
                    skipped.push((name.clone(), why.reason()));
                }
            }
        }
    }
    (fonts, skipped)
}

/// `p` as it is written to a PowerPoint file: with the option on, the fonts its slides are drawn
/// in embedded; with it off, no embedded fonts.
pub fn for_saving(p: &Presentation) -> Cow<'_, Presentation> {
    if !p.embed_fonts {
        if p.embedded_fonts.is_empty() {
            return Cow::Borrowed(p);
        }
        let mut out = p.clone();
        out.embedded_fonts.clear();
        return Cow::Owned(out);
    }
    let (fonts, skipped) = collect(p);
    if let Ok(mut r) = REPORT.lock() {
        *r = skipped;
    }
    let named = named_fonts(p);
    let mut out = p.clone();
    // Fonts the file embedded that weren't made again, while the slides still name them.
    let mut list: Vec<EmbeddedFont> = p
        .embedded_fonts
        .iter()
        .filter(|e| !fonts.iter().any(|f| f.family.eq_ignore_ascii_case(e.typeface.trim())))
        .filter(|e| named.iter().any(|n| n.eq_ignore_ascii_case(e.typeface.trim())))
        .cloned()
        .collect();
    let mut bytes = 0usize;
    let mut faces = list.iter().map(|e| e.faces().iter().flatten().count()).sum::<usize>();
    for f in fonts {
        if faces >= MAX_FACES || bytes.saturating_add(f.data.len()) > MAX_FONT_BYTES {
            log::warn!("pptx: font embedding limit reached; {} is not embedded", f.family);
            continue;
        }
        bytes += f.data.len();
        let Some(id) = out.add_media("font.fntdata", "application/x-fontdata", f.data) else { continue };
        faces += 1;
        let at = match list.iter().position(|e| e.typeface.eq_ignore_ascii_case(&f.family)) {
            Some(i) => i,
            None => {
                list.push(EmbeddedFont { typeface: f.family.clone(), ..Default::default() });
                list.len() - 1
            }
        };
        if let Some(e) = list.get_mut(at) {
            let slot = match (f.bold, f.italic) {
                (false, false) => &mut e.regular,
                (true, false) => &mut e.bold,
                (false, true) => &mut e.italic,
                (true, true) => &mut e.bold_italic,
            };
            *slot = Some(id);
        }
    }
    out.embedded_fonts = list;
    Cow::Owned(out)
}

/// The fonts the last save couldn't embed, as a status line (`None`: all were). Taking it clears
/// it.
pub fn take_report() -> Option<String> {
    let skipped = REPORT.lock().map(|mut r| std::mem::take(&mut *r)).unwrap_or_default();
    match skipped.as_slice() {
        [] => None,
        [(font, why)] => Some(format!("{font} isn't embedded: {why}")),
        [first, rest @ ..] => Some(format!("{} isn't embedded: {}; {} more font(s) aren't either", first.0, first.1, rest.len())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Session;
    use serde_json::json;

    /// Saving sets the report the report test reads: they take turns.
    static TURN: Mutex<()> = Mutex::new(());

    fn turn() -> std::sync::MutexGuard<'static, ()> {
        TURN.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// A presentation whose text is drawn in Ubuntu (compiled into every build), with the
    /// option on, embedding only the characters used unless `all`.
    fn ubuntu_deck(all: bool) -> Session {
        let mut s = Session::with_new();
        s.execute("slide.new", &json!({"layout": "titleAndContent", "title": "Hello fonts", "body": "Only these letters"})).unwrap();
        let used = crate::fonts::used_fonts(&s.doc().unwrap().doc);
        for f in used {
            s.execute("font.replace", &json!({"from": f, "to": deckcraft_fonts::LAST_RESORT_FAMILY})).unwrap();
        }
        s.execute("file.embedFonts", &json!({"embed": true, "allCharacters": all})).unwrap();
        s
    }

    fn saved(s: &Session) -> Presentation {
        let bytes = crate::cmd::file::save_bytes(&s.doc().unwrap().doc, "pptx").unwrap();
        deckcraft_pptx::import(&bytes).unwrap()
    }

    fn font_part<'a>(p: &'a Presentation, typeface: &str) -> &'a [u8] {
        let e = p.embedded_fonts.iter().find(|e| e.typeface == typeface).unwrap_or_else(|| panic!("{typeface} embedded: {:?}", p.embedded_fonts));
        p.media(e.regular.unwrap()).unwrap().data.as_slice()
    }

    #[test]
    fn the_font_the_text_is_in_is_embedded_as_embedded_open_type() {
        let _turn = turn();
        let back = saved(&ubuntu_deck(false));
        assert!(back.embed_fonts && !back.embed_all_characters, "the settings are read back");
        let part = font_part(&back, deckcraft_fonts::LAST_RESORT_FAMILY);
        assert_eq!(part.get(34..36), Some(&[0x4C, 0x50][..]), "an Embedded OpenType part");
        let sfnt = deckcraft_fonts::embedded_sfnt(part).expect("a font inside");
        assert!(sfnt.starts_with(&[0, 1, 0, 0]), "a TrueType font");
    }

    #[test]
    fn only_the_characters_used_make_a_smaller_part() {
        let _turn = turn();
        let sub = saved(&ubuntu_deck(false));
        let whole = saved(&ubuntu_deck(true));
        assert!(whole.embed_all_characters);
        let (a, b) = (font_part(&sub, "Ubuntu").len(), font_part(&whole, "Ubuntu").len());
        assert!(a * 2 < b, "{a} vs {b}");
    }

    #[test]
    fn a_substitute_is_not_embedded_under_another_name() {
        let mut s = ubuntu_deck(false);
        s.execute("font.replace", &json!({"from": "Ubuntu", "to": "No Such Font Anywhere"})).unwrap();
        let (fonts, skipped) = collect(&s.doc().unwrap().doc);
        assert!(fonts.iter().all(|f| !f.family.eq_ignore_ascii_case("No Such Font Anywhere")));
        assert!(fonts.iter().all(|f| !f.family.eq_ignore_ascii_case("Ubuntu")), "the stand-in isn't embedded either");
        assert!(skipped.is_empty());
    }

    #[test]
    fn turning_the_option_off_drops_the_fonts() {
        let _turn = turn();
        let mut s = ubuntu_deck(false);
        let bytes = crate::cmd::file::save_bytes(&s.doc().unwrap().doc, "pptx").unwrap();
        let data = crate::cmd::base64_encode(&bytes);
        s.execute("file.openBytes", &json!({"name": "embedded.pptx", "data": data})).unwrap();
        assert_eq!(s.execute("file.fontEmbedding", &json!({})).unwrap(), json!({"embed": true, "allCharacters": false}), "opening keeps the setting");
        s.execute("file.embedFonts", &json!({"embed": false})).unwrap();
        let back = saved(&s);
        assert!(back.embedded_fonts.is_empty() && !back.embed_fonts);
    }

    #[test]
    fn the_command_needs_a_setting_and_undoes() {
        let mut s = Session::with_new();
        assert!(s.execute("file.embedFonts", &json!({})).is_err());
        assert_eq!(s.execute("file.embedFonts", &json!({"embed": true})).unwrap(), json!({"embed": true, "allCharacters": false}));
        s.execute("edit.undo", &json!({})).unwrap();
        assert_eq!(s.execute("file.fontEmbedding", &json!({})).unwrap(), json!({"embed": false, "allCharacters": false}));
    }

    #[test]
    fn the_report_names_what_was_left_out() {
        let _turn = turn();
        if let Ok(mut r) = REPORT.lock() {
            *r = vec![("Secret Sans".into(), "its licence doesn't allow embedding")];
        }
        assert_eq!(take_report().as_deref(), Some("Secret Sans isn't embedded: its licence doesn't allow embedding"));
        assert_eq!(take_report(), None, "taking it clears it");
    }
}
