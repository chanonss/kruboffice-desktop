//! File › Options, laid out the same way in every KrubOffice app: a list of categories on the
//! left (icon and name), a divider, and the chosen category's settings on the right.
//!
//! The categories, their order and their icons are fixed here, so a setting the apps share sits
//! in the same place in each. An app passes the categories it has settings for; the others are
//! left out, and the rest keep their order. Names are passed in already translated.

use egui::{Color32, FontId, Rect, Response, RichText, Sense, Ui, pos2, vec2};

/// Every category, in the order the list shows them.
pub const CATEGORIES: [&str; 8] = ["general", "display", "proofing", "formulas", "save", "language", "accessibility", "advanced"];

/// The English name of category `id`, the key the apps translate.
pub fn name(id: &str) -> &'static str {
    match id {
        "display" => "Display",
        "proofing" => "Proofing",
        "formulas" => "Formulas",
        "save" => "Save",
        "language" => "Language",
        "accessibility" => "Accessibility",
        "advanced" => "Advanced",
        _ => "General",
    }
}

/// The line-set icon of category `id`.
pub fn icon(id: &str) -> &'static str {
    match id {
        "display" => "optionsDisplay",
        "proofing" => "optionsProofing",
        "formulas" => "optionsFormulas",
        "save" => "optionsSave",
        "language" => "optionsLanguage",
        "accessibility" => "optionsAccessibility",
        "advanced" => "optionsAdvanced",
        _ => "optionsGeneral",
    }
}

/// The app's colours and fonts for the page.
#[derive(Clone, Debug)]
pub struct Style {
    pub text: Color32,
    pub icon: Color32,
    /// The chosen category's row.
    pub selected: Color32,
    pub hover: Color32,
    pub border: Color32,
    pub regular: FontId,
    pub semibold: FontId,
    /// Names a row for assistive technology as a selectable item (`selected`, `name`); the
    /// apps run different egui versions, so each says it in its own API.
    pub announce: fn(&Response, bool, &str),
}

const LIST_WIDTH: f32 = 170.0;
const ROW_HEIGHT: f32 = 30.0;
const BODY_WIDTH: f32 = 640.0;

/// The category `current` names, or the first of `shown` when it names none of them.
pub fn resolve<'a>(shown: &[&'a str], current: &str) -> &'a str {
    shown.iter().copied().find(|id| *id == current).or_else(|| shown.first().copied()).unwrap_or("general")
}

/// The category list and the settings beside it. `shown` lists the app's categories (any
/// order: the list keeps [`CATEGORIES`] order); `label` translates a category's English name;
/// `current` is the chosen one and changes on a click; `body` draws the chosen category.
pub fn page(ui: &mut Ui, style: &Style, shown: &[&str], label: impl Fn(&str) -> String, current: &mut String, body: impl FnOnce(&mut Ui, &str)) {
    let shown: Vec<&str> = CATEGORIES.into_iter().filter(|id| shown.contains(id)).collect();
    let chosen = resolve(&shown, current);
    if *current != chosen {
        *current = chosen.to_string();
    }
    ui.horizontal_top(|ui| {
        let top = ui.cursor().min.y;
        ui.vertical(|ui| {
            ui.set_width(LIST_WIDTH);
            for id in &shown {
                if row(ui, style, id, &label(name(id)), *id == chosen) {
                    *current = (*id).to_string();
                }
            }
        });
        ui.add_space(12.0);
        let (line, _) = ui.allocate_exact_size(vec2(1.0, ROW_HEIGHT * shown.len() as f32), Sense::hover());
        ui.add_space(20.0);
        let bottom = ui
            .vertical(|ui| {
                ui.set_max_width(BODY_WIDTH);
                body(ui, chosen);
            })
            .response
            .rect
            .bottom();
        // The divider runs the length of the longer column.
        let line = Rect::from_min_max(pos2(line.min.x, top), pos2(line.max.x, bottom.max(line.bottom())));
        ui.painter().rect_filled(line, 0.0, style.border);
    });
}

/// One category row; `true` when clicked.
fn row(ui: &mut Ui, style: &Style, id: &str, label: &str, active: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(LIST_WIDTH, ROW_HEIGHT), Sense::click());
    (style.announce)(&resp, active, label);
    let p = ui.painter();
    if active {
        p.rect_filled(r, 4.0, style.selected);
    } else if resp.hovered() {
        p.rect_filled(r, 4.0, style.hover);
    }
    let icon_rect = Rect::from_center_size(pos2(r.min.x + 19.0, r.center().y), vec2(18.0, 18.0));
    if !crate::line::paint(p, icon_rect, icon(id), style.icon, false) {
        crate::paint_named(p, icon_rect, icon(id), style.icon, false, false);
    }
    let font = if active { style.semibold.clone() } else { style.regular.clone() };
    // Long translations stay inside the row; the widget info keeps the full name.
    p.with_clip_rect(r).text(pos2(r.min.x + 34.0, r.center().y), egui::Align2::LEFT_CENTER, label, font, style.text);
    resp.clicked()
}

/// A section heading inside a category (already translated).
pub fn section(ui: &mut Ui, style: &Style, title: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).font(FontId::new(15.0, style.semibold.family.clone())).color(style.text));
    ui.add_space(2.0);
}

/// Small, quiet text under a setting (already translated).
pub fn note(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().weak());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_category_has_its_own_icon_in_the_line_set() {
        let ctx = egui::Context::default();
        ctx.begin_pass(egui::RawInput::default());
        let p = ctx.layer_painter(egui::LayerId::background());
        let r = Rect::from_min_size(pos2(0.0, 0.0), vec2(24.0, 24.0));
        let mut seen = std::collections::HashSet::new();
        for id in CATEGORIES {
            assert!(crate::line::paint(&p, r, icon(id), Color32::BLACK, false), "{id}");
            assert!(seen.insert(icon(id)), "{id} shares an icon");
        }
        ctx.end_pass().textures_delta.clear();
    }

    #[test]
    fn an_unknown_or_hidden_category_falls_back_to_the_first() {
        assert_eq!(resolve(&["general", "save"], "save"), "save");
        assert_eq!(resolve(&["general", "save"], "proofing"), "general");
        assert_eq!(resolve(&[], "save"), "general");
    }
}
