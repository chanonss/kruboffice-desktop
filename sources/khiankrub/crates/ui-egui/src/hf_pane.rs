//! The Header & Footer pane (owner, 2026-11-19): while a header or footer is being edited, a
//! side pane on the right holds what the Header & Footer tab had, in sections: the Header, Footer
//! and Page Number menus, Insert, Navigation (Go to Header/Footer, Previous/Next Section, Link to
//! Previous), Options and Position, then Close Header and Footer. It opens by itself when editing
//! starts and closes when it ends; its × hides it until the next time. It has the shared pane
//! width and the shared section headings (`krub_icons::pane_ui`).

use egui::Ui;
use serde_json::json;

use crate::WordApp;
use crate::options::MeasureExt as _;
use crate::table_pane::{GAP, col_w, faces, field, follow, ruled};

const CLOSED: &str = "hf_pane_closed";

/// Whether the pane shows: a header or footer is being edited and the pane wasn't closed in it.
pub fn wanted(app: &WordApp, ctx: &egui::Context) -> bool {
    follow(ctx, CLOSED, app.session.in_header_footer())
}

/// A grid button `w` wide that runs `id` with `params`, enabled while `enabled` and its command
/// allows; `on` draws it toggled.
#[allow(clippy::too_many_arguments)]
fn run_button(ui: &mut Ui, app: &mut WordApp, icon: &str, label: &str, id: &str, params: serde_json::Value, on: bool, enabled: bool, w: f32) {
    ui.add_enabled_ui(enabled, |ui| {
        crate::widgets::pane_button(ui, app, icon, label, id, Some(params), on, false, w);
    });
}

/// A grid tile `w` wide that opens the menu `items`.
fn tile_menu(ui: &mut Ui, app: &mut WordApp, icon: &str, label: &str, id: &str, w: f32, items: fn(&mut Ui, &mut WordApp)) {
    crate::table_pane::tile_menu(ui, app, icon, label, id, w, items);
}

/// One row of a pane grid: `add` lays its buttons left to right with the grid's gap.
fn row(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        add(ui);
    });
}

pub fn show(app: &mut WordApp, ui: &mut Ui) {
    if crate::panes::header(ui, "Header & Footer") {
        crate::table_pane::close_pane(ui.ctx(), CLOSED);
        return;
    }
    let st = wordcraft_engine::cmd::hf::state(&mut app.session);
    let page = wordcraft_engine::cmd::hf::context(&mut app.session).map_or(0, |c| c.page);
    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("hf_pane_scroll").show(ui, |ui| {
        // Even grids with the ribbon's button face, under hairline headings (owner, 2026-11-22).
        faces(ui, |ui| {
            ui.spacing_mut().item_spacing.y = GAP;
            // The three menus first, as tiles, with no heading over them (the pane's title says it).
            let w = col_w(ui, 3);
            row(ui, |ui| {
                tile_menu(ui, app, "header", "Header", "insert.header", w, crate::ribbon::header_items);
                tile_menu(ui, app, "footer", "Footer", "insert.footer", w, crate::ribbon::footer_items);
                tile_menu(ui, app, "pageNumber", "Page Number", "insert.pageNumber", w, crate::ribbon::page_number_items);
            });
            ruled(ui, "Insert", |ui| {
                let w = col_w(ui, 3);
                row(ui, |ui| {
                    crate::widgets::pane_tile(ui, app, "dateTime", "Date & Time", "insert.dateTime", Some(json!({})), false, false, w);
                    crate::widgets::pane_tile(ui, app, "hfField", "Field", "ui.dialog", Some(json!({"name": "field"})), false, false, w);
                    crate::widgets::pane_tile(ui, app, "hfPictureFile", "Picture from File", "insert.picture", Some(json!({})), false, false, w);
                });
            });
            ruled(ui, "Navigation", |ui| {
                let w = col_w(ui, 2);
                row(ui, |ui| {
                    run_button(ui, app, "hfGoHeader", "Go to Header", "insert.editHeader", json!({"page": page}), false, !st.header, w);
                    run_button(ui, app, "hfGoFooter", "Go to Footer", "insert.editFooter", json!({"page": page}), false, st.header, w);
                });
                row(ui, |ui| {
                    run_button(ui, app, "hfPrevious", "Previous Section", "hf.previous", json!({}), false, st.section > 0, w);
                    run_button(ui, app, "hfNext", "Next Section", "hf.next", json!({}), false, st.section + 1 < st.sections, w);
                });
                let full = ui.available_width().floor();
                ui.add_enabled_ui(st.can_link, |ui| {
                    crate::widgets::pane_button(ui, app, "hfLink", "Link to Previous", "layout.linkToPrevious", Some(json!({})), st.linked, false, full);
                })
                .response
                .on_hover_text(tl!("Use the previous section's header or footer; turn off to give this section its own"));
            });
            ruled(ui, "Options", |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                let mut first = st.different_first;
                if ui.checkbox(&mut first, tl!("Different First Page")).changed() {
                    let _ = app.run("layout.differentFirstPage", json!({"value": first}));
                }
                let mut odd = st.different_odd_even;
                if ui.checkbox(&mut odd, tl!("Different Odd & Even Pages")).changed() {
                    let _ = app.run("layout.differentOddEven", json!({"value": odd}));
                }
                // Landscape pages in a portrait report: the page number turns to where the bound
                // sheet has it (owner, 2026-11-22).
                let mut turn = st.turn_on_landscape;
                if ui
                    .checkbox(&mut turn, tl!("Turn on Landscape Pages"))
                    .on_hover_text(tl!("On a landscape page in a portrait report, the header and footer turn with the page, so when the report is bound the page number sits and reads where it does on every other page"))
                    .changed()
                {
                    let _ = app.run("hf.turnOnLandscape", json!({"value": turn}));
                }
                let mut text = st.show_document_text;
                if ui.checkbox(&mut text, tl!("Show Document Text")).changed() {
                    let _ = app.run("hf.showDocumentText", json!({"value": text}));
                }
            });
            ruled(ui, "Position", |ui| position(app, ui, st.header_from_top, st.footer_from_bottom));
            // Close across the pane, under a hairline of its own.
            let line = ui.visuals().widgets.noninteractive.bg_stroke;
            let y = ui.cursor().top() + 2.0;
            ui.painter().hline(ui.available_rect_before_wrap().x_range(), y, egui::Stroke::new(1.0, line.color));
            ui.add_space(8.0);
            let full = ui.available_width().floor();
            crate::widgets::pane_button(ui, app, "hfClose", "Close Header and Footer", "insert.closeHeader", Some(json!({})), false, false, full);
        });
    });
}

/// Header from Top and Footer from Bottom side by side, each named under its box, in the
/// options' unit; a drag is one undo step.
fn position(app: &mut WordApp, ui: &mut Ui, from_top: f32, from_bottom: f32) {
    let unit = crate::options::unit();
    let u = unit.pt_per_unit();
    let w = col_w(ui, 2);
    row(ui, |ui| {
        for (label, key, pt) in [("Header from Top", "header", from_top), ("Footer from Bottom", "footer", from_bottom)] {
            let mut v = pt / u;
            let r = field(ui, w, label, crate::numfield::NumField::new(&mut v).speed(0.05).range(0.0..=300.0 / u).measure(unit).max_decimals(2));
            if r.changed() {
                if r.dragged() && !r.drag_started() {
                    app.session.join_next_undo();
                }
                let _ = app.run("hf.position", json!({key: (v * u).clamp(0.0, 300.0)}));
            }
        }
    });
}
