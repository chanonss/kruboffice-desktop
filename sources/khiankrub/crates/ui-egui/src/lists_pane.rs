//! The Table of Contents pane (owner, 2026-11-22): one place for the table of contents, the
//! list of figures and the list of tables. References › Manage Table of Contents opens it. At
//! the top, which of the three the document has and on which page; then the one to work on,
//! its layout (general, or the Thai academic report layout) and its settings. A change to a list
//! in the document rebuilds it where it is at once (`references.list`), as the Styles and Table
//! panes change the document; for one not in the document yet the settings wait for Insert at
//! Cursor. It has the shared pane width and section headings.

use egui::{Ui, vec2};
use serde_json::{Value, json};
use wordcraft_engine::cmd::citations::LabelMode;
use wordcraft_engine::cmd::ref_lists::{Kind, ListInfo, leader_name, list_info};

use crate::WordApp;
use crate::table_pane::{buttons, section};

/// The pane's state: whether it is open, the list it shows, settings chosen for lists not in
/// the document yet (the parameters `references.list` will insert them with), and the list the
/// cursor was last in (the pane turns to a list when the cursor goes into it).
#[derive(Default)]
pub struct ListsPane {
    pub open: bool,
    pub kind: usize,
    pending: [serde_json::Map<String, Value>; 3],
    caret: Option<(usize, Option<Kind>)>,
}

impl ListsPane {
    /// Turn to the list the cursor has gone into (body block `block`): only when it enters one,
    /// so another list picked while the cursor stays put keeps showing.
    fn follow_caret(&mut self, session: &wordcraft_engine::Session) {
        let focus = &session.sel.focus;
        let block = (focus.story == wordcraft_doc::StoryRef::Body).then(|| focus.path.0.first().copied().map(|b| b as usize)).flatten();
        let Some(block) = block else {
            self.caret = None;
            return;
        };
        if self.caret.is_some_and(|(b, _)| b == block) {
            return;
        }
        let at = wordcraft_engine::cmd::ref_lists::list_at(session, block);
        let before = self.caret.and_then(|(_, k)| k);
        if let Some(k) = at.filter(|k| Some(*k) != before) {
            self.kind = Kind::ALL.iter().position(|x| *x == k).unwrap_or(self.kind);
        }
        self.caret = Some((block, at));
    }
}

impl ListsPane {
    /// Open on `kind` (or on the one it showed last).
    pub fn open_on(&mut self, kind: Option<Kind>) {
        self.open = true;
        if let Some(k) = kind {
            self.kind = Kind::ALL.iter().position(|x| *x == k).unwrap_or(0);
        }
    }
}

fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Contents => "Contents",
        Kind::Figures => "Figures",
        Kind::Tables => "Tables",
    }
}

fn kind_icon(k: Kind) -> &'static str {
    match k {
        Kind::Contents => "toc",
        Kind::Figures => "tableOfFigures",
        Kind::Tables => "tableOfTables",
    }
}

/// The settings shown: the list's own, with any chosen for one not inserted yet on top.
fn shown(info: &ListInfo, pending: &serde_json::Map<String, Value>) -> ListInfo {
    let mut v = info.clone();
    if info.block.is_some() {
        return v;
    }
    let s = |k: &str| pending.get(k).and_then(Value::as_str);
    if let Some(l) = s("layout") {
        v.report = l == "report";
        if v.report && v.kind != Kind::Contents {
            v.mode = LabelMode::Number;
            v.column_heading = true;
        }
        v.leader = if v.report { wordcraft_doc::props::TabLeader::None } else { wordcraft_doc::props::TabLeader::Dot };
    }
    if let Some(l) = s("leader") {
        v.leader = wordcraft_engine::cmd::references::leader(l);
    }
    if let Some(n) = pending.get("levels").and_then(Value::as_u64) {
        v.levels = n.clamp(1, 9) as u8;
    }
    if let Some(l) = s("label") {
        v.label = l.to_string();
    }
    if let Some(m) = s("show").and_then(LabelMode::from_name) {
        v.mode = m;
    }
    if let Some(c) = pending.get("columnHeading").and_then(Value::as_bool) {
        v.column_heading = c;
    }
    v
}

pub fn show(app: &mut WordApp, ui: &mut Ui) {
    if crate::panes::header(ui, "Table of Contents") {
        app.lists_pane.open = false;
        return;
    }
    app.lists_pane.follow_caret(&app.session);
    let infos: Vec<ListInfo> = Kind::ALL.iter().map(|k| list_info(&mut app.session, *k)).collect();
    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("lists_pane_scroll").show(ui, |ui| {
        section(ui, "In This Document", |ui| {
            for (i, info) in infos.iter().enumerate() {
                if list_row(ui, info, app.lists_pane.kind == i) {
                    app.lists_pane.kind = i;
                }
            }
        });
        ui.add_space(6.0);
        let segments: Vec<krub_icons::pane_ui::Segment> =
            Kind::ALL.iter().map(|k| krub_icons::pane_ui::Segment { icon: kind_icon(*k), label: tl!(kind_name(*k)).to_string() }).collect();
        let mut kind = app.lists_pane.kind;
        krub_icons::pane_ui::segmented_at(ui, egui::Id::new("lists_pane_kind"), &segments, &mut kind);
        app.lists_pane.kind = kind.min(2);
        let kind = Kind::ALL[app.lists_pane.kind];
        let info = infos.get(app.lists_pane.kind).cloned().unwrap_or_else(|| list_info(&mut app.session, kind));
        let cur = shown(&info, &app.lists_pane.pending[app.lists_pane.kind]);
        ui.add_space(6.0);
        let mut change: Option<(&str, Value)> = None;
        section(ui, "Layout", |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
                let w = ((ui.available_width() - 6.0) / 2.0).max(60.0);
                if layout_card(ui, "General", false, !cur.report, w) {
                    change = Some(("layout", json!("general")));
                }
                if layout_card(ui, "Thai Academic Report", true, cur.report, w) {
                    change = Some(("layout", json!("report")));
                }
            });
        });
        section(ui, "Settings", |ui| {
            // Names in one column, the controls one width in the other (owner, 2026-11-22).
            let name_w = (ui.available_width() * 0.32).floor();
            let ctl_w = (ui.available_width() - name_w - crate::table_pane::GAP - 8.0).floor().max(60.0);
            egui::Grid::new("lists_pane_settings").num_columns(2).min_col_width(name_w).spacing(vec2(crate::table_pane::GAP, 6.0)).show(ui, |ui| {
                if kind == Kind::Contents {
                    ui.label(tl!("Show levels"));
                    let mut levels = cur.levels;
                    egui::ComboBox::from_id_salt("lists_pane_levels").selected_text(levels.to_string()).width(ctl_w).show_ui(ui, |ui| {
                        for n in 1..=9u8 {
                            ui.selectable_value(&mut levels, n, n.to_string());
                        }
                    });
                    if levels != cur.levels {
                        change = Some(("levels", json!(levels)));
                    }
                    ui.end_row();
                } else {
                    ui.label(tl!("Label"));
                    let options: &[&str] = if kind == Kind::Tables { &["ตารางที่", "Table"] } else { &["ภาพที่", "Figure"] };
                    let mut label = cur.label.clone();
                    egui::ComboBox::from_id_salt("lists_pane_label").selected_text(label.clone()).width(ctl_w).show_ui(ui, |ui| {
                        for o in options.iter().copied().chain((!options.contains(&cur.label.as_str())).then_some(cur.label.as_str())) {
                            ui.selectable_value(&mut label, o.to_string(), o);
                        }
                    });
                    if label != cur.label {
                        change = Some(("label", json!(label)));
                    }
                    ui.end_row();
                    ui.label(tl!("Each line"));
                    ui.vertical(|ui| {
                        let example = |mode: LabelMode| match mode {
                            LabelMode::Full => format!("{} 3.1", cur.label),
                            LabelMode::Number => "3.1".to_string(),
                            LabelMode::None => tl!("Title only").to_string(),
                        };
                        for (mode, name) in [(LabelMode::Full, "Label and number"), (LabelMode::Number, "Number only"), (LabelMode::None, "Neither")] {
                            let text = format!("{} ({})", tl!(name), example(mode));
                            if ui.radio(cur.mode == mode, text).clicked() && cur.mode != mode {
                                change = Some(("show", json!(mode.name())));
                            }
                        }
                    });
                    ui.end_row();
                    if cur.report {
                        ui.label(tl!("Column heading"));
                        let mut c = cur.column_heading;
                        let text = crate::i18n::fmt(tl!("{label} over the numbers"), &[("label", &cur.label)]);
                        if ui.checkbox(&mut c, text).changed() {
                            change = Some(("columnHeading", json!(c)));
                        }
                        ui.end_row();
                    }
                }
                ui.label(tl!("Leader"));
                let mut leader = leader_name(cur.leader).to_string();
                let names = [("none", "(none)"), ("dot", "Dots ……"), ("hyphen", "Dashes ------"), ("underscore", "Line ______")];
                let shown_name = names.iter().find(|(k, _)| *k == leader).map_or(leader.clone(), |(_, n)| tl!(n).to_string());
                egui::ComboBox::from_id_salt("lists_pane_leader").selected_text(shown_name).width(ctl_w).show_ui(ui, |ui| {
                    for (k, n) in names {
                        ui.selectable_value(&mut leader, k.to_string(), tl!(n));
                    }
                });
                if leader != leader_name(cur.leader) {
                    change = Some(("leader", json!(leader)));
                }
                ui.end_row();
            });
            if cur.report {
                ui.add_space(4.0);
                let note = if kind == Kind::Contents {
                    tl!("A centred heading, หน้า over the page numbers, and the heading again with (ต่อ) on each further page. TOC 2 is indented 1.27 cm.")
                } else {
                    tl!("A centred heading, หน้า over the page numbers, and the heading again with (ต่อ) on each further page.")
                };
                ui.label(egui::RichText::new(note).weak().small());
            }
            ui.add_space(4.0);
            // The styles the list is set in: each level of a table of contents, or the entries
            // of a list of figures or tables, in Modify Style.
            let styles: Vec<String> =
                if kind == Kind::Contents { (1..=cur.levels.max(1)).map(|l| format!("TOC{l}")).collect() } else { vec!["ListEntry".to_string()] };
            let mut modify = None;
            let full = ui.available_width().floor();
            buttons(ui, |ui| {
                let resp = crate::widgets::pane_button(ui, app, "properties", "Modify Style…", "ui.dialog", None, false, styles.len() > 1, full);
                if styles.len() == 1 {
                    if resp.clicked() {
                        modify = styles.first().cloned();
                    }
                } else {
                    egui::Popup::menu(&resp).show(|ui| {
                        for st in &styles {
                            let name = app.session.doc.styles.get(st).map_or_else(|| st.clone(), |x| x.name.clone());
                            if ui.button(name).clicked() {
                                modify = Some(st.clone());
                                ui.close();
                            }
                        }
                    });
                }
            });
            if let Some(st) = modify
                && let Some(d) = crate::dialogs::Dialog::modify_style(app, &st)
            {
                app.dialog = Some(d);
            }
        });
        if let Some((k, v)) = change {
            if info.block.is_some() {
                let mut params = serde_json::Map::new();
                params.insert("kind".into(), json!(kind.name()));
                params.insert(k.into(), v);
                let _ = app.run("references.list", Value::Object(params));
            } else {
                let pending = &mut app.lists_pane.pending[app.lists_pane.kind];
                if k == "layout" {
                    pending.remove("leader");
                    pending.remove("show");
                    pending.remove("columnHeading");
                }
                pending.insert(k.into(), v);
            }
        }
        ui.add_space(8.0);
        // Update and Remove in two even columns; Insert at Cursor across the pane.
        let (half, full) = (crate::table_pane::col_w(ui, 2), ui.available_width().floor());
        crate::table_pane::grid(ui, |ui| {
            if info.block.is_some() {
                crate::widgets::pane_button(ui, app, "update", "Update", "references.updateLists", Some(json!({})), false, false, half);
                crate::widgets::pane_button(ui, app, "delete", "Remove", "references.list", Some(json!({"kind": kind.name(), "remove": true})), false, false, half);
            } else {
                let mut params = app.lists_pane.pending[app.lists_pane.kind].clone();
                params.insert("kind".into(), json!(kind.name()));
                let resp = crate::widgets::pane_button(ui, app, kind_icon(kind), "Insert at Cursor", "references.list", Some(Value::Object(params)), false, false, full);
                if resp.clicked() {
                    app.lists_pane.pending[app.lists_pane.kind].clear();
                }
            }
        });
    });
}

/// A row of "In This Document": the list's icon and name, and its page or "Not inserted";
/// `true` when clicked.
fn list_row(ui: &mut Ui, info: &ListInfo, selected: bool) -> bool {
    let t = crate::theme::Tokens::get(ui.ctx());
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, 26.0), egui::Sense::click());
    let name = tl!(match info.kind {
        Kind::Contents => "Table of Contents",
        Kind::Figures => "List of Figures",
        Kind::Tables => "List of Tables",
    });
    let status = match &info.page {
        Some(p) => crate::i18n::fmt(tl!("Page {page}"), &[("page", p)]),
        None => tl!("Not inserted").to_string(),
    };
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, format!("{name}: {status}")));
    if selected {
        ui.painter().rect_filled(r, 5.0, t.checked);
    } else if resp.hovered() {
        ui.painter().rect_filled(r, 5.0, t.hover);
    }
    crate::icons::paint(
        ui.painter(),
        egui::Rect::from_center_size(egui::pos2(r.left() + 14.0, r.center().y), vec2(18.0, 18.0)),
        kind_icon(info.kind),
        t.icon,
        t.accent,
    );
    ui.painter().text(egui::pos2(r.left() + 30.0, r.center().y), egui::Align2::LEFT_CENTER, name, crate::theme::regular(13.0), t.text);
    let dim = if info.page.is_some() { t.text_dim } else { t.text_disabled };
    ui.painter().text(egui::pos2(r.right() - 6.0, r.center().y), egui::Align2::RIGHT_CENTER, status, crate::theme::regular(12.0), dim);
    resp.clicked()
}

/// A layout choice drawn as a small card: its name and a hint of how it looks; `true` when
/// picked.
fn layout_card(ui: &mut Ui, name: &str, report: bool, on: bool, w: f32) -> bool {
    let t = crate::theme::Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(w, 44.0), egui::Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, on, tl!(name)));
    let stroke = if on { egui::Stroke::new(2.0, t.accent) } else { egui::Stroke::new(1.0, t.border_strong) };
    if resp.hovered() && !on {
        ui.painter().rect_filled(r, 6.0, t.hover);
    }
    ui.painter().rect_stroke(r.shrink(1.0), 6.0, stroke, egui::StrokeKind::Inside);
    let clip = r.shrink(6.0);
    let p = ui.painter().with_clip_rect(clip);
    p.text(egui::pos2(r.left() + 8.0, r.top() + 14.0), egui::Align2::LEFT_CENTER, tl!(name), crate::theme::medium(12.0), t.text);
    let hint = if report { "หน้า · (ต่อ)" } else { "1.1 ………… 5" };
    p.text(egui::pos2(r.left() + 8.0, r.top() + 31.0), egui::Align2::LEFT_CENTER, hint, crate::theme::regular(11.0), t.text_dim);
    resp.clicked() && !on
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordcraft_doc::{Path, Pos, StoryRef};
    use wordcraft_engine::{Selection, Session};

    fn caret_at(s: &mut Session, block: usize) {
        s.sel = Selection::caret(Pos { story: StoryRef::Body, path: Path::top(block), off: 0 });
    }

    /// The pane turns to the list the cursor goes into, and keeps a list picked by hand while the
    /// cursor stays where it is.
    #[test]
    fn the_pane_follows_the_list_the_cursor_is_in() {
        let mut s = Session::new(wordcraft_doc::Document::from_text(""));
        for t in ["ตารางที่", "ภาพที่"] {
            s.run("references.caption", &json!({"label": t, "text": "ก"})).unwrap();
            s.run("text.newParagraph", &json!({})).unwrap();
        }
        s.run("caret.docStart", &json!({})).unwrap();
        s.run("references.list", &json!({"kind": "figures", "layout": "report"})).unwrap();
        s.run("caret.docStart", &json!({})).unwrap();
        s.run("references.list", &json!({"kind": "tables", "layout": "report"})).unwrap();
        let fig = list_info(&mut s, Kind::Figures).block.unwrap();
        let tab = list_info(&mut s, Kind::Tables).block.unwrap();
        let mut pane = ListsPane::default();
        let kind = |p: &ListsPane| Kind::ALL[p.kind];

        caret_at(&mut s, fig);
        pane.follow_caret(&s);
        assert_eq!(kind(&pane), Kind::Figures);
        // Picked by hand while the cursor stays in the list of figures: kept.
        pane.kind = 0;
        caret_at(&mut s, fig + 1);
        pane.follow_caret(&s);
        assert_eq!(kind(&pane), Kind::Contents);
        // Into the list of tables: turns to it.
        caret_at(&mut s, tab);
        pane.follow_caret(&s);
        assert_eq!(kind(&pane), Kind::Tables);
        // Out of the lists: unchanged.
        let last = s.doc.body.len() - 1;
        caret_at(&mut s, last);
        pane.follow_caret(&s);
        assert_eq!(kind(&pane), Kind::Tables);
    }
}
