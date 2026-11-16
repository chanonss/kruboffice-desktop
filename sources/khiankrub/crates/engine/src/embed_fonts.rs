//! Fonts embedded when a Word document is saved (File › Options › Save › Embed fonts in the file,
//! [`Settings::embed_fonts`](wordcraft_doc::Settings)).
//!
//! The document is laid out and every face its text is drawn in is considered: a face is embedded
//! under the font name the text asks for when it *is* that font (a substitute shown for a font
//! that isn't installed, or a fallback for characters the font lacks, is not embedded under a
//! name it doesn't have). Common system fonts are left out unless `embed_system_fonts` is on, as
//! in Word. Each face's own licence decides the rest (`wordcraft_fonts::embed`). Faces that can't
//! be embedded are reported once, after the save ([`take_report`]).

use std::sync::Mutex;

use wordcraft_doc::Document;
use wordcraft_docx::EmbedFont;

/// Fonts that come with Windows, macOS or Office, which a reader has already: Word's "Do not
/// embed common system fonts" leaves them out. Compared case-insensitively.
const COMMON_SYSTEM_FONTS: &[&str] = &[
    "Arial",
    "Arial Black",
    "Calibri",
    "Calibri Light",
    "Cambria",
    "Cambria Math",
    "Candara",
    "Comic Sans MS",
    "Consolas",
    "Constantia",
    "Corbel",
    "Courier New",
    "Franklin Gothic Medium",
    "Gabriola",
    "Georgia",
    "Impact",
    "Lucida Console",
    "Lucida Sans Unicode",
    "Malgun Gothic",
    "Microsoft JhengHei",
    "Microsoft YaHei",
    "MS Gothic",
    "MS Mincho",
    "Palatino Linotype",
    "Segoe Print",
    "Segoe Script",
    "Segoe UI",
    "Segoe UI Symbol",
    "SimSun",
    "Symbol",
    "Tahoma",
    "Times New Roman",
    "Trebuchet MS",
    "Verdana",
    "Webdings",
    "Wingdings",
    "Yu Gothic",
    // Thai fonts that come with Windows and Office.
    "Angsana New",
    "AngsanaUPC",
    "Browallia New",
    "BrowalliaUPC",
    "Cordia New",
    "CordiaUPC",
    "DilleniaUPC",
    "EucrosiaUPC",
    "FreesiaUPC",
    "IrisUPC",
    "JasmineUPC",
    "KodchiangUPC",
    "Leelawadee",
    "Leelawadee UI",
    "LilyUPC",
    "Microsoft Sans Serif",
    // macOS.
    "Helvetica",
    "Helvetica Neue",
    "Menlo",
    "Thonburi",
];

/// What the last save left out, for the status line.
static REPORT: Mutex<Vec<(String, &'static str)>> = Mutex::new(Vec::new());

/// Whether `family` is one of [`COMMON_SYSTEM_FONTS`].
pub fn is_common_system_font(family: &str) -> bool {
    let family = family.trim();
    COMMON_SYSTEM_FONTS.iter().any(|f| f.eq_ignore_ascii_case(family))
}

/// The fonts to embed in `doc`, and the fonts that couldn't be (name, why).
pub fn collect(doc: &Document) -> (Vec<EmbedFont>, Vec<(String, &'static str)>) {
    let s = &doc.settings;
    let layout = wordcraft_layout::layout(doc, &mut wordcraft_layout::LayoutCache::new(), &wordcraft_layout::LayoutOptions::default());
    let mut fonts: Vec<EmbedFont> = Vec::new();
    let mut skipped: Vec<(String, &'static str)> = Vec::new();
    for used in layout.used_faces() {
        let face = used.face.get();
        // The name the text asks for that this face is; none: a substitute or fallback.
        let Some(name) = used.fonts.iter().find(|f| f.eq_ignore_ascii_case(face.family.trim()) || f.eq_ignore_ascii_case(face.legacy_family.trim()))
        else {
            continue;
        };
        if !s.embed_system_fonts && is_common_system_font(name) {
            continue;
        }
        let bold = face.weight >= 600.0;
        let italic = face.italic;
        if fonts.iter().any(|f| f.family.eq_ignore_ascii_case(name) && f.bold == bold && f.italic == italic) {
            continue;
        }
        match wordcraft_fonts::embed::prepare(face, s.embed_subset.then_some(&used.gids)) {
            Ok(p) => fonts.push(EmbedFont { family: name.clone(), bold, italic, data: p.data, subsetted: p.subsetted }),
            Err(why) => {
                if !skipped.iter().any(|(f, _)| f.eq_ignore_ascii_case(name)) {
                    skipped.push((name.clone(), why.reason()));
                }
            }
        }
    }
    (fonts, skipped)
}

/// Whether the font table `doc` was read with embeds fonts.
pub fn file_embeds_fonts(doc: &Document) -> bool {
    doc.passthrough.get("word/fontTable.xml").is_some_and(|t| t.windows(7).any(|w| w == b":embedR" || w == b":embedB" || w == b":embedI"))
}

/// Write `doc` as a Word package of `flavor`, embedding fonts as its settings say: with the
/// option on, the fonts it is drawn in; with it off, none (fonts the file embedded are dropped,
/// as Word does).
pub fn write(doc: &Document, flavor: wordcraft_docx::Flavor) -> Result<Vec<u8>, String> {
    if doc.settings.embed_fonts {
        let (fonts, skipped) = collect(doc);
        if let Ok(mut r) = REPORT.lock() {
            *r = skipped;
        }
        wordcraft_docx::write_as_embedding(doc, flavor, &fonts).map_err(|e| e.to_string())
    } else if file_embeds_fonts(doc) {
        wordcraft_docx::write_as_embedding(doc, flavor, &[]).map_err(|e| e.to_string())
    } else {
        wordcraft_docx::write_as(doc, flavor).map_err(|e| e.to_string())
    }
}

/// The fonts the last save couldn't embed, as a status line (`None`: all were). Taking it
/// clears it.
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

    fn thai_doc(subset: bool) -> Document {
        let mut s = Session::new(Document::default());
        s.run("text.insert", &json!({"text": "สวัสดีครับ Hello"})).unwrap();
        s.run("select.all", &json!({})).unwrap();
        s.run("format.font", &json!({"name": "Sarabun"})).unwrap();
        s.doc.settings.embed_fonts = true;
        s.doc.settings.embed_subset = subset;
        s.doc.clone()
    }

    fn unzip(bytes: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        (0..z.len())
            .map(|i| {
                let mut f = z.by_index(i).unwrap();
                let mut v = Vec::new();
                std::io::Read::read_to_end(&mut f, &mut v).unwrap();
                (f.name().to_string(), v)
            })
            .collect()
    }

    #[test]
    fn the_font_the_text_is_in_is_embedded_and_read_back() {
        let doc = thai_doc(false);
        let (fonts, skipped) = collect(&doc);
        assert!(skipped.is_empty(), "{skipped:?}");
        assert!(
            fonts.iter().any(|f| f.family == "Sarabun" && !f.bold && !f.italic && !f.subsetted),
            "{:?}",
            fonts.iter().map(|f| &f.family).collect::<Vec<_>>()
        );
        let bytes = write(&doc, wordcraft_docx::Flavor::Document).unwrap();
        let files = unzip(&bytes);
        let table = String::from_utf8(files["word/fontTable.xml"].clone()).unwrap();
        assert!(table.contains(r#"<w:font w:name="Sarabun">"#) && table.contains("w:embedRegular"), "{table}");
        assert!(files.keys().any(|k| k.starts_with("word/fonts/") && k.ends_with(".odttf")));
        let settings = String::from_utf8(files["word/settings.xml"].clone()).unwrap();
        assert!(settings.contains("<w:embedTrueTypeFonts/>"), "{settings}");
        let back = wordcraft_docx::read(&bytes).unwrap();
        assert!(back.settings.embed_fonts);
        let embedded = wordcraft_docx::embedded_fonts(&back);
        assert!(!embedded.is_empty());
        assert!(embedded.iter().all(|f| f.starts_with(&[0, 1, 0, 0])), "the fonts de-obfuscate to TrueType files");
    }

    #[test]
    fn a_subset_is_smaller_and_flagged() {
        let whole = collect(&thai_doc(false)).0;
        let sub = collect(&thai_doc(true)).0;
        let size = |v: &[EmbedFont]| v.iter().find(|f| f.family == "Sarabun").map(|f| f.data.len()).unwrap();
        assert!(sub.iter().all(|f| f.subsetted));
        assert!(size(&sub) * 2 < size(&whole), "{} vs {}", size(&sub), size(&whole));
        let bytes = write(&thai_doc(true), wordcraft_docx::Flavor::Document).unwrap();
        let files = unzip(&bytes);
        assert!(String::from_utf8_lossy(&files["word/settings.xml"]).contains("<w:saveSubsetFonts/>"));
        assert!(String::from_utf8_lossy(&files["word/fontTable.xml"]).contains(r#"w:subsetted="1""#));
    }

    #[test]
    fn common_system_fonts_are_left_out_unless_asked_for() {
        assert!(is_common_system_font("tahoma") && is_common_system_font(" Angsana New "));
        assert!(!is_common_system_font("Sarabun") && !is_common_system_font("TH Sarabun New"));
    }

    #[test]
    fn turning_the_option_off_drops_fonts_the_file_embedded() {
        let bytes = write(&thai_doc(false), wordcraft_docx::Flavor::Document).unwrap();
        let mut back = wordcraft_docx::read(&bytes).unwrap();
        assert!(file_embeds_fonts(&back));
        back.settings.embed_fonts = false;
        let files = unzip(&write(&back, wordcraft_docx::Flavor::Document).unwrap());
        assert!(!files.keys().any(|k| k.starts_with("word/fonts/")), "{:?}", files.keys().collect::<Vec<_>>());
        assert!(!String::from_utf8_lossy(&files["word/fontTable.xml"]).contains("w:embed"));
        assert!(!String::from_utf8_lossy(&files["word/settings.xml"]).contains("embedTrueTypeFonts"));
    }
}
