//! The Thai academic report layout of generated lists: the table of contents (สารบัญ), the list
//! of figures (สารบัญภาพ) and the list of tables (สารบัญตาราง), as Thai university report and
//! thesis guides set them out.
//!
//! - The heading is centred and bold, four points larger than body text (20 pt over 16 pt TH
//!   Sarabun), with one blank body-text line under it and then the word "หน้า" ("page") at the
//!   right margin, above the column of page numbers.
//! - Each further page of the list starts with the heading again followed by "(ต่อ)"
//!   ("continued"), the blank line and "หน้า".
//!
//! Word has no such feature, and Thai writers type these lines by hand after a page break. The
//! engine does the same, as real paragraphs inside the list that every Update Table rebuilds
//! from the layout: the continued heading starts its page (page break before), so a list's pages
//! don't move when text before the list changes, and a .docx keeps exactly what is printed (in a
//! TOC they sit inside the field result, so Word's own Update Table writes its plain TOC). The
//! list keeps this layout while it holds a page label paragraph: that is how an update without
//! settings, or a reopened file, knows to keep doing it.

use wordcraft_doc::props::{Align, CharProps, NumRef, TabAlign, TabLeader, TabStop, TextColor};
use wordcraft_doc::{Block, ParaProps, Paragraph, Path, Pos, StoryRef, Style, StyleKind};

use crate::{CmdError, Session};

/// The styles of a list's continued headings and page labels. They start with "TOC" in a table
/// of contents, so the TOC (and the .docx field around its entries) takes them in.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Styles {
    pub cont: &'static str,
    cont_name: &'static str,
    pub label: &'static str,
    label_name: &'static str,
}

/// A table of contents' styles.
pub(crate) const TOC: Styles = Styles { cont: "TOCContinued", cont_name: "TOC Continued", label: "TOCPageLabel", label_name: "TOC Page Label" };
/// A list of figures' or tables' styles.
pub(crate) const LIST: Styles = Styles { cont: "ListContinued", cont_name: "List Continued", label: "ListPageLabel", label_name: "List Page Label" };

/// The label over the page numbers when none is given.
pub const PAGE_LABEL: &str = "หน้า";

/// A label is a short line of text.
const MAX_LABEL: usize = 64;

/// At most this many continued headings in one list (a list of thousands of pages is not a
/// report's): each costs a layout.
const MAX_PAGES: usize = 400;

/// The `pageLabel` parameter: trimmed, without control characters, at most 64 characters. A
/// tab may separate a column heading at the left margin from the page label (`ตารางที่\tหน้า`:
/// the report guides' lists of figures and tables head their numbers' column too).
pub(crate) fn clean_label(t: &str) -> String {
    let (head, page) = t.rsplit_once('\t').unwrap_or(("", t));
    let clean = |t: &str| {
        let t: String = t.chars().filter(|c| !c.is_control() && *c != wordcraft_doc::para::OBJ).take(MAX_LABEL).collect();
        t.trim().to_string()
    };
    let (head, page) = (clean(head), clean(page));
    if head.is_empty() || page.is_empty() { page } else { format!("{head}\t{page}") }
}

/// The column heading at the left of a page label line (`ตารางที่`), if it has one.
pub fn column_heading(label: &str) -> Option<&str> {
    label.split_once('\t').map(|(h, _)| h).filter(|h| !h.is_empty())
}

/// What follows the heading on its further pages: "(ต่อ)" under a Thai page label, else
/// "(continued)".
fn continued_word(label: &str) -> &'static str {
    if label.chars().any(|c| ('\u{0E00}'..='\u{0E7F}').contains(&c)) { "(ต่อ)" } else { "(continued)" }
}

/// True for a paragraph of the continued heading or page label styles `st`.
pub(crate) fn is_part(p: &Paragraph, st: Styles) -> bool {
    p.props.style.as_deref().is_some_and(|x| x == st.cont || x == st.label)
}

/// The page label of the list in body blocks `from..=to`: the text of its first non-empty page
/// label paragraph. `None` when the list doesn't have the report layout.
pub(crate) fn label_in(s: &Session, from: usize, to: usize, st: Styles) -> Option<String> {
    (from..=to).filter_map(|i| s.doc.body.get(i)?.as_para()).find_map(|p| {
        let t = clean_label(&p.plain_text());
        (p.props.style.as_deref() == Some(st.label) && !t.is_empty()).then_some(t)
    })
}

/// Add the continued heading and page label styles `st` if the document doesn't have them.
pub(crate) fn add_styles(s: &mut Session, st: Styles) {
    if s.doc.styles.get(st.cont).is_none() {
        s.doc.styles.upsert(Style {
            id: st.cont.into(),
            name: st.cont_name.into(),
            kind: StyleKind::Paragraph,
            based_on: Some("TOCHeading".into()),
            para: ParaProps { page_break_before: Some(true), outline_level: Some(9), ..Default::default() },
            hidden: true,
            ..Default::default()
        });
    }
    if s.doc.styles.get(st.label).is_none() {
        s.doc.styles.upsert(Style {
            id: st.label.into(),
            name: st.label_name.into(),
            kind: StyleKind::Paragraph,
            based_on: Some("Normal".into()),
            para: ParaProps {
                align: Some(Align::Right),
                space_before: Some(0.0),
                space_after: Some(0.0),
                keep_next: Some(true),
                ..Default::default()
            },
            chr: CharProps { bold: Some(true), bold_cs: Some(true), ..Default::default() },
            hidden: true,
            ..Default::default()
        });
    }
}

/// The report look of a list heading: centred, bold, four points over body text, in the body
/// text's font, black, unnumbered (a heading style may number chapters), with no space around it
/// and kept with the list. Returns its paragraph and text formatting.
pub(crate) fn heading_look(s: &Session) -> (ParaProps, CharProps) {
    let body = s.doc.styles.resolve_char(Some("Normal"), &CharProps::default());
    let para = ParaProps {
        align: Some(Align::Center),
        space_before: Some(0.0),
        space_after: Some(0.0),
        keep_next: Some(true),
        numbering: Some(NumRef { num: 0, level: 0 }),
        indent_left: Some(0.0),
        indent_first: Some(0.0),
        ..Default::default()
    };
    let size = (body.size + 4.0).clamp(1.0, 1638.0);
    let size_cs = (body.size_cs + 4.0).clamp(1.0, 1638.0);
    let chr = CharProps {
        font: Some(body.font.clone()),
        font_cs: Some(body.font_cs.clone()),
        size: Some(size),
        size_cs: Some(size_cs),
        bold: Some(true),
        bold_cs: Some(true),
        color: Some(TextColor::Auto),
        ..Default::default()
    };
    (para, chr)
}

/// The blank line and the page label that go under a heading (one body-text line each). With a
/// column heading the line runs from the left margin, the page label at a right tab on the
/// margin (`width`, the text width). The label line keeps to the margins whatever its style
/// inherits from Normal: no indents, and the tab stops in `inherited` (the style chain's) cleared,
/// so a stop set in Normal can't catch the tab before the page label.
fn label_paras(label: &str, st: Styles, width: f32, inherited: &[f32]) -> [Paragraph; 2] {
    let mut line = Paragraph::with_text(label, CharProps::default()).styled(st.label);
    line.props.indent_left = Some(0.0);
    line.props.indent_first = Some(0.0);
    line.props.indent_right = Some(0.0);
    if column_heading(label).is_some() {
        let right = (width - 0.5).max(1.0);
        line.props.align = Some(Align::Left);
        let mut tabs: Vec<TabStop> = inherited
            .iter()
            .filter(|p| (*p - right).abs() >= 0.5)
            .take(64)
            .map(|p| TabStop { pos: *p, align: TabAlign::Clear, leader: TabLeader::None })
            .collect();
        tabs.push(TabStop { pos: right, align: TabAlign::Right, leader: TabLeader::None });
        line.props.tabs = Some(tabs);
    }
    [Paragraph::new().styled(st.label), line]
}

/// The text width of the document's current section, for the page label's right tab.
fn text_width(s: &Session) -> f32 {
    super::page::sect(s).text_width()
}

/// The tab stops a page label line gets from its style chain (Normal's included).
fn inherited_tabs(s: &Session, st: Styles) -> Vec<f32> {
    s.doc.styles.resolve_para(&ParaProps { style: Some(st.label.into()), ..Default::default() }).tabs.iter().map(|t| t.pos).collect()
}

/// Put the blank line and page label under the heading at body block `head`. Returns how many
/// blocks went in.
pub(crate) fn insert_label(s: &mut Session, head: usize, label: &str, st: Styles) -> Result<usize, CmdError> {
    let paras = label_paras(label, st, text_width(s), &inherited_tabs(s, st));
    let n = paras.len();
    for (k, p) in paras.into_iter().enumerate() {
        s.doc.insert_block(StoryRef::Body, &Path::top(head + 1 + k), Block::Para(p))?;
    }
    Ok(n)
}

/// The continued heading for the list whose heading is body block `head`: the heading's text
/// and look, then "(ต่อ)", starting a new page.
fn continued_heading(s: &Session, head: usize, label: &str, st: Styles) -> Paragraph {
    let (text, para, chr) = match s.doc.body.get(head).and_then(|b| b.as_para()) {
        Some(h) => {
            let t: String = h.plain_text().chars().filter(|c| !c.is_control() && *c != wordcraft_doc::para::OBJ).collect();
            let chr = CharProps { link: None, ..h.props_of_char(0).clone() };
            (t.trim().to_string(), h.props.clone(), chr)
        }
        None => (String::new(), ParaProps::default(), CharProps::default()),
    };
    let word = continued_word(label);
    let text = if text.is_empty() { word.to_string() } else { format!("{text} {word}") };
    let mut p = Paragraph::with_text(&text, chr.clone());
    p.mark = chr;
    p.props = ParaProps { style: Some(st.cont.into()), page_break_before: Some(true), fmt_change: None, num_change: None, ..para };
    p
}

/// The layout page holding `pos`.
fn page_of(l: &wordcraft_layout::DocLayout, i: usize, off: usize) -> Option<usize> {
    l.caret(&Pos { story: StoryRef::Body, path: Path::top(i), off }).map(|c| c.page)
}

/// Where the list (heading at body block `head`, its blocks up to `end`) goes on to a new page,
/// start that page with the continued heading, blank line and page label. An entry that would
/// be split over two pages moves whole to the new page; one that is the first under a heading
/// stays where it is (it can't go anywhere better). Returns the list's new last block.
pub(crate) fn paginate(s: &mut Session, head: usize, mut end: usize, label: &str, st: Styles) -> Result<usize, CmdError> {
    for _ in 0..MAX_PAGES {
        s.touch();
        let l = s.layout();
        let Some(mut page) = page_of(&l, head, 0) else { break };
        let mut after_entry = false;
        let mut cut = None;
        for i in head + 1..=end {
            let Some(p) = s.doc.body.get(i).and_then(|b| b.as_para()) else { break };
            match p.props.style.as_deref() {
                Some(x) if x == st.cont => {
                    page = page_of(&l, i, 0).unwrap_or(page);
                    after_entry = false;
                }
                Some(x) if x == st.label => after_entry = false,
                _ => {
                    let last = page_of(&l, i, p.len()).unwrap_or(page);
                    if last > page && after_entry {
                        cut = Some(i);
                        break;
                    }
                    page = page.max(last);
                    after_entry = true;
                }
            }
        }
        let Some(i) = cut else { break };
        let mut paras = vec![continued_heading(s, head, label, st)];
        paras.extend(label_paras(label, st, text_width(s), &inherited_tabs(s, st)));
        for (k, p) in paras.into_iter().enumerate() {
            s.doc.insert_block(StoryRef::Body, &Path::top(i + k), Block::Para(p))?;
            end += 1;
        }
    }
    s.touch();
    Ok(end)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wordcraft_doc::{Document, Paragraph};

    use super::*;

    /// A report: an empty first paragraph for the list, then `chapters` chapters of `sections`
    /// sections, each heading with a paragraph and a figure caption under it, a page apart.
    fn report(chapters: usize, sections: usize) -> Session {
        let mut doc = Document::from_text("");
        let mut push = |p: Paragraph| doc.body.push(Block::Para(p).into());
        for c in 1..=chapters {
            let mut h = Paragraph::with_text(&format!("บทที่ {c} บทนำ"), CharProps::default()).styled("Heading1");
            h.props.page_break_before = Some(true);
            push(h);
            for k in 1..=sections {
                push(Paragraph::with_text(&format!("หัวข้อใหญ่ {c}.{k}"), CharProps::default()).styled("Heading2"));
                push(Paragraph::with_text("เนื้อหาของหัวข้อ", CharProps::default()));
                push(Paragraph::with_text(&format!("ภาพที่ {c}.{k} ภาพประกอบ"), CharProps::default()).styled("Caption"));
            }
        }
        let mut s = Session::new(doc);
        s.run("caret.docStart", &json!({})).unwrap();
        s
    }

    /// The styles and texts of the body blocks.
    fn blocks(s: &Session) -> Vec<(String, String)> {
        s.doc.body.iter().filter_map(|b| b.as_para()).map(|p| (p.props.style.clone().unwrap_or_default(), p.plain_text())).collect()
    }

    /// The layout page of body block `i`'s first character.
    fn page(s: &mut Session, i: usize) -> usize {
        let l = s.layout();
        page_of(&l, i, 0).unwrap()
    }

    /// The first block on each page the list (heading `head` through `end`) covers, and whether
    /// each later page starts with a continued heading followed by the blank line and label.
    fn check_pages(s: &mut Session, head: usize, st: Styles, title: &str) -> usize {
        let b = blocks(s);
        let end = (head + 1..b.len())
            .take_while(|i| {
                b[*i].0.starts_with("TOC") && b[*i].0 != "TOCHeading" || b[*i].0 == "ListEntry" || b[*i].0 == st.cont || b[*i].0 == st.label
            })
            .last()
            .unwrap();
        let mut pages = 1;
        let mut at = page(s, head);
        for i in head + 1..=end {
            let p = page(s, i);
            if p != at {
                assert_eq!(p, at + 1, "block {i} {:?} skips a page", b[i]);
                assert_eq!(b[i], (st.cont.to_string(), format!("{title} (ต่อ)")), "page {p} starts with the continued heading");
                assert_eq!(b[i + 1], (st.label.to_string(), String::new()), "then a blank line");
                assert_eq!(b[i + 2], (st.label.to_string(), PAGE_LABEL.to_string()), "then หน้า");
                at = p;
                pages += 1;
            } else {
                assert_ne!(b[i].0, st.cont, "a continued heading in the middle of page {p}");
            }
        }
        pages
    }

    /// Thai report guides: under สารบัญ a blank line and "หน้า" at the right margin, and on each
    /// further page "สารบัญ (ต่อ)", the blank line and "หน้า" again; TOC 2 indented 1.27 cm, the
    /// page numbers at the right margin with no leader.
    #[test]
    fn a_report_toc_repeats_its_heading_on_each_page() {
        let mut s = report(8, 6);
        s.run("references.toc", &json!({"levels": 2, "continued": true})).unwrap();
        let b = blocks(&s);
        assert_eq!(b[0].1, "สารบัญ");
        assert_eq!(b[1], ("TOCPageLabel".into(), String::new()));
        assert_eq!(b[2], ("TOCPageLabel".into(), "หน้า".into()));
        assert!(b[3].1.starts_with("บทที่ 1 บทนำ\t"), "{:?}", b[3]);
        // The heading's look.
        let head = s.doc.body[0].as_para().unwrap();
        assert_eq!(head.props.align, Some(Align::Center));
        let c = s.doc.styles.resolve_char(head.props.style.as_deref(), head.props_of_char(0));
        let body = s.doc.styles.resolve_char(Some("Normal"), &CharProps::default());
        assert!(c.bold && c.bold_cs && (c.size - body.size - 4.0).abs() < 0.01);
        assert_eq!(s.doc.styles.resolve_para(&ParaProps { style: Some("TOCPageLabel".into()), ..Default::default() }).align, Align::Right);
        // TOC 2 at 1.27 cm, right tab without leader.
        assert_eq!(s.doc.styles.get("TOC2").unwrap().para.indent_left, Some(36.0));
        let entry = s.doc.body[4].as_para().unwrap();
        assert_eq!(entry.props.style.as_deref(), Some("TOC2"));
        let tab = entry.props.tabs.as_ref().unwrap().last().unwrap();
        assert_eq!((tab.align, tab.leader), (wordcraft_doc::props::TabAlign::Right, wordcraft_doc::props::TabLeader::None));
        let pages = check_pages(&mut s, 0, TOC, "สารบัญ");
        assert!(pages >= 2, "the TOC runs over {pages} page(s)");
        // Page numbers follow the headings, the TOC's own pages counted.
        let chapter = s.doc.body.iter().position(|b| b.as_para().is_some_and(|p| p.props.style.as_deref() == Some("Heading1"))).unwrap();
        let first = page(&mut s, chapter) + 1;
        assert!(b[3].1.ends_with(&format!("\t{first}")), "{:?} vs page {first}", b[3]);
        // Updating keeps it, without adding more.
        let before = blocks(&s);
        s.run("references.updateToc", &json!({})).unwrap();
        assert_eq!(blocks(&s), before);
    }

    /// A heading number is set off by one space, not a tab.
    #[test]
    fn report_toc_entries_put_a_space_after_the_number() {
        let mut s = report(1, 2);
        s.run("select.text", &json!({"text": "หัวข้อใหญ่ 1.1"})).unwrap();
        s.run("para.multilevel", &json!({"kind": "legal"})).unwrap();
        s.run("para.listLevel", &json!({"level": 1})).unwrap();
        s.run("caret.docStart", &json!({})).unwrap();
        s.run("references.toc", &json!({"continued": true})).unwrap();
        let entry = blocks(&s).into_iter().find(|(_, t)| t.contains("หัวข้อใหญ่ 1.1")).unwrap().1;
        let (text, _page) = entry.rsplit_once('\t').unwrap();
        assert!(!text.contains('\t') && text.contains(". หัวข้อใหญ่ 1.1"), "{entry:?}");
    }

    /// The continued headings follow the list: more entries move them, fewer remove them, and an
    /// update without settings keeps the layout (it is in the table, not in the command).
    #[test]
    fn continued_headings_follow_updates() {
        let mut s = report(8, 6);
        s.run("references.toc", &json!({"levels": 2, "continued": true})).unwrap();
        let conts = |s: &Session| blocks(s).iter().filter(|(st, _)| st == TOC.cont).count();
        assert!(conts(&s) >= 1);
        // Shorter: one chapter left in the TOC.
        for st in ["Heading1", "Heading2"] {
            for b in s.doc.body.iter_mut().skip(40) {
                if let Block::Para(p) = std::sync::Arc::make_mut(b)
                    && p.props.style.as_deref() == Some(st)
                {
                    p.props.style = Some("Normal".into());
                }
            }
        }
        s.touch();
        s.run("references.updateToc", &json!({})).unwrap();
        let toc_len = blocks(&s).iter().filter(|(st, _)| st.starts_with("TOC") && st != "TOCHeading").count();
        assert!(toc_len < 20, "{:?}", blocks(&s));
        assert_eq!(conts(&s), 0, "one page needs no continued heading");
        assert_eq!(blocks(&s)[2], ("TOCPageLabel".into(), "หน้า".into()), "the label stays");
        // Without the layout: replacing the table with plain settings drops it.
        s.run("references.toc", &json!({"levels": 2})).unwrap();
        assert!(blocks(&s).iter().all(|(st, _)| st != TOC.label && st != TOC.cont));
    }

    /// Saved to .docx and opened again, the table keeps its layout, and updating it there gives
    /// the same blocks.
    #[test]
    fn a_report_toc_round_trips_through_docx() {
        let mut s = report(8, 6);
        s.run("references.toc", &json!({"levels": 2, "continued": true})).unwrap();
        let before = blocks(&s);
        let bytes = wordcraft_docx::write(&s.doc).unwrap();
        let mut back = Session::new(wordcraft_docx::read(&bytes).unwrap());
        assert_eq!(blocks(&back), before);
        back.run("references.updateToc", &json!({})).unwrap();
        assert_eq!(blocks(&back), before);
        check_pages(&mut back, 0, TOC, "สารบัญ");
    }

    /// สารบัญภาพ and สารบัญตาราง: the same layout for tables of figures and tables.
    #[test]
    fn a_report_list_of_figures_repeats_its_heading() {
        let mut s = report(8, 6);
        s.run("references.tableOfFigures", &json!({"continued": true})).unwrap();
        let b = blocks(&s);
        assert_eq!(b[0].1, "สารบัญภาพ");
        assert_eq!(b[2], ("ListPageLabel".into(), "หน้า".into()));
        assert!(b[3].1.starts_with("ภาพที่ 1.1 ภาพประกอบ\t"), "{:?}", b[3]);
        let pages = check_pages(&mut s, 0, LIST, "สารบัญภาพ");
        assert!(pages >= 2);
        // The page numbers count the list's own pages.
        let cap = s.doc.body.iter().position(|b| b.as_para().is_some_and(|p| p.props.style.as_deref() == Some("Caption"))).unwrap();
        let first = page(&mut s, cap) + 1;
        assert!(b[3].1.ends_with(&format!("\t{first}")), "{:?} vs page {first}", b[3]);
        let before = blocks(&s);
        s.run("references.updateFigures", &json!({"kind": "figure"})).unwrap();
        assert_eq!(blocks(&s), before);
        let bytes = wordcraft_docx::write(&s.doc).unwrap();
        let mut back = Session::new(wordcraft_docx::read(&bytes).unwrap());
        back.run("references.updateFigures", &json!({})).unwrap();
        assert_eq!(blocks(&back), before);
        // Tables: สารบัญตาราง, with an English label "(continued)".
        let mut s = report(1, 1);
        s.run("references.tableOfFigures", &json!({"kind": "table", "continued": true, "pageLabel": "Page", "title": "List of Tables"})).unwrap();
        assert_eq!(blocks(&s)[0].1, "List of Tables");
        assert_eq!(continued_word("Page"), "(continued)");
        assert_eq!(label_in(&s, 1, 2, LIST).as_deref(), Some("Page"));
    }

    /// Hostile labels are cut to a short line; an empty one is หน้า.
    #[test]
    fn page_labels_are_short_lines() {
        assert_eq!(clean_label(&"ก".repeat(5000)).chars().count(), MAX_LABEL);
        assert_eq!(clean_label(" a\u{0}\nb "), "ab");
        let mut s = report(1, 1);
        s.run("references.toc", &json!({"continued": true, "pageLabel": "\n"})).unwrap();
        assert_eq!(blocks(&s)[2].1, PAGE_LABEL);
    }

    /// A tab stop and a first-line indent set in Normal (which the page label's style is based
    /// on) don't move the column heading or the page label: "ภาพที่" stays at the left margin and
    /// "หน้า" ends at the right one, after updating and after a .docx round trip too.
    #[test]
    fn normal_tab_stops_and_indents_leave_the_page_label_line_alone() {
        let mut s = report(1, 1);
        if let Some(n) = s.doc.styles.get("Normal").cloned() {
            let mut n = n;
            n.para.tabs = Some(vec![TabStop { pos: 28.35, align: TabAlign::Left, leader: TabLeader::None }]);
            n.para.indent_first = Some(36.0);
            s.doc.styles.upsert(n);
        }
        s.run("references.tableOfFigures", &json!({"continued": true, "pageLabel": "ภาพที่\tหน้า"})).unwrap();
        let check = |s: &mut Session, step: &str| {
            let width = super::text_width(s);
            let left = super::super::page::sect(s).margin_left;
            let i = (0..s.doc.body.len())
                .find(|i| s.doc.body[*i].as_para().is_some_and(|p| p.plain_text() == "ภาพที่\tหน้า"))
                .unwrap_or_else(|| panic!("{step}: no label line"));
            let len = s.doc.body[i].as_para().unwrap().len();
            let l = s.layout();
            let at = |off| l.caret(&Pos { story: StoryRef::Body, path: Path::top(i), off }).unwrap().x;
            assert!((at(0) - left).abs() < 0.5, "{step}: ภาพที่ starts at {} (margin {left})", at(0));
            assert!(at(len) - left > width - 2.0, "{step}: หน้า ends at {} (margin {left}, width {width})", at(len));
        };
        check(&mut s, "inserted");
        s.run("references.updateLists", &json!({})).unwrap();
        check(&mut s, "updated");
        let bytes = wordcraft_docx::write(&s.doc).unwrap();
        let mut back = Session::new(wordcraft_docx::read(&bytes).unwrap());
        check(&mut back, "reopened");
    }
}
