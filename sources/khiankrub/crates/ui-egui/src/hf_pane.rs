//! The Header & Footer pane (owner, 2026-11-19): while a header or footer is being edited, a
//! side pane on the right holds what the Header & Footer tab had, in sections: the Header, Footer
//! and Page Number menus, Insert, Navigation (Go to Header/Footer, Previous/Next Section, Link to
//! Previous), Options and Position, then Close Header and Footer. It opens by itself when editing
//! starts and closes when it ends; its × hides it until the next time. It has the shared pane
//! width and the shared section headings (`krub_icons::pane_ui`).

use egui::{Ui, vec2};
use serde_json::json;

use crate::WordApp;
use crate::options::MeasureExt as _;
use crate::table_pane::{buttons, follow, menu, section};
use crate::widgets::labeled_button;

const CLOSED: &str = "hf_pane_closed";

/// Whether the pane shows: a header or footer is being edited and the pane wasn't closed in it.
pub fn wanted(app: &WordApp, ctx: &egui::Context) -> bool {
    follow(ctx, CLOSED, app.session.in_header_footer())
}

/// A labeled button that runs `id` with `params`.
fn run_button(ui: &mut Ui, app: &mut WordApp, icon: &str, label: &str, id: &str, params: serde_json::Value, on: bool) {
    labeled_button(ui, app, icon, label, id, Some(params), on, false, 0.0);
}

pub fn show(app: &mut WordApp, ui: &mut Ui) {
    if crate::panes::header(ui, "Header & Footer") {
        crate::table_pane::close_pane(ui.ctx(), CLOSED);
        return;
    }
    let st = wordcraft_engine::cmd::hf::state(&mut app.session);
    let page = wordcraft_engine::cmd::hf::context(&mut app.session).map_or(0, |c| c.page);
    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("hf_pane_scroll").show(ui, |ui| {
        section(ui, "Header & Footer", |ui| {
            buttons(ui, |ui| {
                menu(ui, app, "header", "Header", "insert.header", crate::ribbon::header_items);
                menu(ui, app, "footer", "Footer", "insert.footer", crate::ribbon::footer_items);
                menu(ui, app, "pageNumber", "Page Number", "insert.pageNumber", crate::ribbon::page_number_items);
            });
        });
        section(ui, "Insert", |ui| {
            buttons(ui, |ui| {
                run_button(ui, app, "dateTime", "Date & Time", "insert.dateTime", json!({}), false);
                run_button(ui, app, "hfField", "Field", "ui.dialog", json!({"name": "field"}), false);
                run_button(ui, app, "hfPictureFile", "Picture from File", "insert.picture", json!({}), false);
            });
        });
        section(ui, "Navigation", |ui| {
            // Two to a row: a button in its own enabled scope doesn't wrap, so a row of four
            // would push the pane past the window's edge.
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                ui.add_enabled_ui(!st.header, |ui| {
                    run_button(ui, app, "hfGoHeader", "Go to Header", "insert.editHeader", json!({"page": page}), false)
                });
                ui.add_enabled_ui(st.header, |ui| {
                    run_button(ui, app, "hfGoFooter", "Go to Footer", "insert.editFooter", json!({"page": page}), false)
                });
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                ui.add_enabled_ui(st.section > 0, |ui| run_button(ui, app, "hfPrevious", "Previous Section", "hf.previous", json!({}), false));
                ui.add_enabled_ui(st.section + 1 < st.sections, |ui| run_button(ui, app, "hfNext", "Next Section", "hf.next", json!({}), false));
            });
            ui.add_enabled_ui(st.can_link, |ui| {
                buttons(ui, |ui| run_button(ui, app, "hfLink", "Link to Previous", "layout.linkToPrevious", json!({}), st.linked));
            })
            .response
            .on_hover_text(tl!("Use the previous section's header or footer; turn off to give this section its own"));
        });
        section(ui, "Options", |ui| {
            let mut first = st.different_first;
            if ui.checkbox(&mut first, tl!("Different First Page")).changed() {
                let _ = app.run("layout.differentFirstPage", json!({"value": first}));
            }
            let mut odd = st.different_odd_even;
            if ui.checkbox(&mut odd, tl!("Different Odd & Even Pages")).changed() {
                let _ = app.run("layout.differentOddEven", json!({"value": odd}));
            }
            let mut text = st.show_document_text;
            if ui.checkbox(&mut text, tl!("Show Document Text")).changed() {
                let _ = app.run("hf.showDocumentText", json!({"value": text}));
            }
        });
        section(ui, "Position", |ui| position(app, ui, st.header_from_top, st.footer_from_bottom));
        ui.add_space(4.0);
        buttons(ui, |ui| run_button(ui, app, "hfClose", "Close Header and Footer", "insert.closeHeader", json!({}), false));
    });
}

/// Header from Top and Footer from Bottom, in the options' unit; a drag is one undo step.
fn position(app: &mut WordApp, ui: &mut Ui, from_top: f32, from_bottom: f32) {
    let unit = crate::options::unit();
    let u = unit.pt_per_unit();
    egui::Grid::new("hf_pane_position")
        .num_columns(2)
        .spacing(vec2(8.0, krub_icons::pane_ui::ROW_GAP))
        .min_col_width(krub_icons::pane_ui::LABEL_W)
        .show(ui, |ui| {
            for (label, key, pt) in [("Header from Top:", "header", from_top), ("Footer from Bottom:", "footer", from_bottom)] {
                ui.label(tl!(label));
                let mut v = pt / u;
                let field = crate::numfield::NumField::new(&mut v).speed(0.05).range(0.0..=300.0 / u).measure(unit).max_decimals(2);
                let r = ui.add_sized(vec2(90.0, 20.0), field);
                if r.changed() {
                    if r.dragged() && !r.drag_started() {
                        app.session.join_next_undo();
                    }
                    let _ = app.run("hf.position", json!({key: (v * u).clamp(0.0, 300.0)}));
                }
                ui.end_row();
            }
        });
}
