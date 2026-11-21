//! Insert › Diagram (ไดอะแกรม): the shared KrubOffice picker ([`krub_icons::diagram_picker`])
//! with PhutKrub's layouts, each drawn the way the Diagram Design gallery draws it.

use krub_icons::diagram_picker::{self, Layout, Outcome, State};

use super::{Dialog, SlideApp, Ui, Value, json};

/// PhutKrub's layouts for the picker, in the gallery's order.
pub(super) fn layouts() -> Vec<Layout> {
    use deckcraft_engine::cmd::smartart::{LAYOUTS, category, category_name, description};
    LAYOUTS.iter().map(|(k, name)| Layout { id: k, name, category: category_name(category(k)), description: description(k) }).collect()
}

/// The picker's state kept in the dialog's fields (`category`, `kind`, `text`).
fn state_of(d: &Dialog) -> State {
    let mut s = State::new(d.fields.get("kind").map_or("process", String::as_str));
    if let Some(c) = d.fields.get("category") {
        s.category.clone_from(c);
    }
    if let Some(t) = d.fields.get("text") {
        s.text.clone_from(t);
    }
    s
}

/// The `insert.smartArt` params for what the picker shows: the layout and the typed items as
/// nodes (none typed: the command's placeholder items).
pub(super) fn insert_params(s: &State) -> Value {
    let nodes: Vec<Value> = s.items().into_iter().map(|(text, level)| json!({"text": text, "level": level})).collect();
    let mut p = json!({"kind": s.layout});
    if !nodes.is_empty() {
        p["nodes"] = Value::Array(nodes);
    }
    p
}

/// The dialog; returns true to close it.
pub(super) fn diagram_dialog(app: &mut SlideApp, ui: &mut Ui, d: &mut Dialog) -> bool {
    let layouts = layouts();
    let mut state = state_of(d);
    let look = deckcraft_model::Diagram::default();
    let slide = app.session.active().map(|doc| doc.selection.slide).unwrap_or(0);
    let max_height = (ui.ctx().content_rect().height() - 220.0).max(100.0);
    let outcome = {
        let textures = &mut app.textures;
        let doc = app.session.active().map(|x| &x.doc);
        diagram_picker::show(ui, &|s| crate::i18n::t(s).to_string(), &layouts, &mut state, max_height, &mut |p, r, id| {
            let Some(doc) = doc else { return };
            let Some(tex) = textures.smartart(p.ctx(), doc, slide, (id, &look.style, &look.colors), None, 192) else { return };
            // Fit the picture in `r`, keeping its shape.
            let size = tex.size_vec2();
            let scale = (r.width() / size.x.max(1.0)).min(r.height() / size.y.max(1.0));
            let fit = egui::Rect::from_center_size(r.center(), size * scale);
            p.image(tex.id(), fit, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        })
    };
    d.fields.insert("category".into(), state.category.clone());
    d.fields.insert("kind".into(), state.layout.clone());
    d.fields.insert("text".into(), state.text.clone());
    match outcome {
        Outcome::Open => false,
        Outcome::Cancel => true,
        Outcome::Insert => {
            let _ = app.run("insert.smartArt", insert_params(&state));
            app.ui.pane = Some("smartart".into());
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_layout_is_offered_under_a_shared_category() {
        let l = layouts();
        assert_eq!(l.len(), deckcraft_engine::cmd::smartart::LAYOUTS.len());
        assert!(l.iter().all(|x| diagram_picker::CATEGORIES.contains(&x.category)));
        assert_eq!(diagram_picker::categories(&l).len(), diagram_picker::CATEGORIES.len());
    }

    #[test]
    fn the_dialog_is_all_in_thai() {
        let th = crate::i18n::Lang::from_code("th").unwrap();
        let l = layouts();
        let cats = diagram_picker::categories(&l).into_iter().skip(1);
        for s in diagram_picker::NAMES.into_iter().chain(cats).chain(l.iter().flat_map(|x| [x.name, x.description])) {
            assert_ne!(crate::i18n::tr(th, s), s, "missing Thai: {s}");
        }
    }

    #[test]
    fn typed_lines_become_nodes_with_tabs_as_sub_points() {
        let mut s = State::new("hierarchy");
        assert_eq!(insert_params(&s), json!({"kind": "hierarchy"}));
        s.text = "Lead\n\tTeam A\n\tTeam B".into();
        assert_eq!(
            insert_params(&s),
            json!({"kind": "hierarchy", "nodes": [{"text": "Lead", "level": 0}, {"text": "Team A", "level": 1}, {"text": "Team B", "level": 1}]})
        );
    }

    /// Tab in the text box indents the line (a sub-point); it never moves the focus away.
    #[test]
    fn tab_in_the_text_box_makes_a_sub_point() {
        use egui_kittest::kittest::Queryable;
        let app = crate::SlideApp::new(deckcraft_engine::Session::with_new(), crate::Services::default());
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1440.0, 900.0)).build_ui_state(
            |ui, app: &mut crate::SlideApp| {
                app.logic(ui.ctx());
                app.ui(ui);
            },
            app,
        );
        h.run_steps(3);
        h.state_mut().run("app.dialog", json!({"id": "smartart"})).unwrap();
        // The dialog's box is the text box highest on the screen (the notes box is at the bottom).
        fn text_box<'a>(h: &'a egui_kittest::Harness<'_, crate::SlideApp>) -> egui_kittest::Node<'a> {
            h.get_all_by_role(egui::accesskit::Role::MultilineTextInput).min_by(|a, b| a.rect().top().total_cmp(&b.rect().top())).unwrap()
        }
        h.run_steps(3);
        text_box(&h).click();
        h.run_steps(2);
        text_box(&h).type_text("Lead");
        h.run_steps(1);
        h.key_press(egui::Key::Enter);
        h.run_steps(1);
        h.key_press(egui::Key::Tab);
        h.run_steps(1);
        text_box(&h).type_text("Team");
        h.run_steps(2);
        let text = h.state().dialog.as_ref().and_then(|d| d.fields.get("text").cloned()).unwrap_or_default();
        assert_eq!(text, "Lead\n\tTeam");
    }
}
