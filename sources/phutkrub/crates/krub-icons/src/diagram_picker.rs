//! Insert › Diagram (ไดอะแกรม), the same dialog in every KrubOffice app that has diagrams (owner,
//! 2026-11-20, from the approved draft): the categories on the left, the layouts of the category
//! as a grid of pictures in the middle, and on the right the picked layout drawn large, its name
//! and description, and a box for the diagram's items (one per line; a line that starts with Tab
//! is a sub-point). Double-clicking a picture inserts it at once.
//!
//! "SmartArt" is Microsoft's trademark, so the KrubOffice apps call the feature "Diagram"; files
//! still hold Office diagram data. Each app gives its own layouts and draws their pictures (the
//! `paint` callback); this returns what the person chose, and the app runs its own command. The
//! apps run different egui versions, so this uses only what they share.

use egui::{Align2, Color32, FontId, Painter, Rect, RichText, Stroke, StrokeKind, Ui, pos2, vec2};

/// The categories, in the order shown; a category no layout of the app has is left out. "All" is
/// shown as "All layouts" (a plain "All" means something else in some apps' tables).
pub const CATEGORIES: [&str; 9] = ["All", "List", "Process", "Cycle", "Hierarchy", "Relationship", "Matrix", "Pyramid", "Picture"];

/// The English text the dialog shows besides the categories and the layouts (for the apps'
/// translation tables and tests).
pub const NAMES: [&str; 7] = [
    "All layouts",
    "Insert Diagram",
    "Diagram text",
    "One item per line. Start a line with Tab for a sub-point.",
    "Leave empty for sample text",
    "OK",
    "Cancel",
];

/// Deepest sub-point level the text box gives (deeper tabs count as this level).
pub const MAX_LEVEL: u8 = 7;

/// Size of a layout's tile, points.
const TILE: f32 = 96.0;
/// Width of the categories column and of the preview column, points.
const CATEGORY_WIDTH: f32 = 112.0;
const PREVIEW_WIDTH: f32 = 210.0;

/// One layout an app offers. `name`, `category` and `description` are English: the dialog
/// translates them with the app's `tr`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    /// The id the app's insert command takes (`process`, `basicList`…).
    pub id: &'static str,
    pub name: &'static str,
    /// One of [`CATEGORIES`] (not "All").
    pub category: &'static str,
    pub description: &'static str,
}

/// What the dialog shows: the category, the picked layout's id and the items typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub category: String,
    pub layout: String,
    pub text: String,
}

impl State {
    /// "All" with `layout` picked and no items typed.
    pub fn new(layout: &str) -> State {
        State { category: "All".into(), layout: layout.into(), text: String::new() }
    }

    /// The layouts the current category shows.
    pub fn shown<'a>(&self, layouts: &'a [Layout]) -> Vec<&'a Layout> {
        layouts.iter().filter(|l| self.category == "All" || l.category == self.category).collect()
    }

    /// The typed items as (text, level): one per non-empty line, its level the number of tabs it
    /// starts with (at most [`MAX_LEVEL`]). Empty when nothing was typed (the app then uses its
    /// sample text).
    pub fn items(&self) -> Vec<(String, u8)> {
        items(&self.text)
    }
}

/// The items of `text`, as [`State::items`].
pub fn items(text: &str) -> Vec<(String, u8)> {
    text.lines()
        .filter_map(|line| {
            let tabs = line.chars().take_while(|c| *c == '\t').count();
            let t = line.trim();
            (!t.is_empty()).then(|| (t.to_string(), u8::try_from(tabs).unwrap_or(MAX_LEVEL).min(MAX_LEVEL)))
        })
        .collect()
}

/// What the person did this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Still choosing.
    Open,
    /// Insert the picked layout with [`State::items`].
    Insert,
    Cancel,
}

/// The categories `layouts` uses, "All" first.
pub fn categories(layouts: &[Layout]) -> Vec<&'static str> {
    CATEGORIES.into_iter().filter(|c| *c == "All" || layouts.iter().any(|l| l.category == *c)).collect()
}

/// The dialog's body and its OK and Cancel buttons. `tr` translates; `paint(painter, rect, id)`
/// draws layout `id`'s picture in `rect`; the grid scrolls past `max_height` points so the
/// buttons stay in the window.
pub fn show(
    ui: &mut Ui,
    tr: &dyn Fn(&str) -> String,
    layouts: &[Layout],
    state: &mut State,
    max_height: f32,
    paint: &mut dyn FnMut(&Painter, Rect, &str),
) -> Outcome {
    if !categories(layouts).contains(&state.category.as_str()) {
        state.category = "All".into();
    }
    if !state.shown(layouts).iter().any(|l| l.id == state.layout)
        && let Some(first) = state.shown(layouts).first()
    {
        state.layout = first.id.to_string();
    }
    let border = ui.visuals().widgets.noninteractive.bg_stroke.color;
    let text = ui.visuals().text_color();
    let dim = ui.visuals().weak_text_color();
    // Three rows of pictures at most; fewer in a short window (the grid then scrolls).
    let height = max_height.clamp(TILE + 8.0, 3.0 * (TILE + 8.0) + 8.0);
    let mut insert = false;
    ui.horizontal_top(|ui| {
        // Categories, their names at the left edge (owner, 2026-11-20).
        ui.vertical(|ui| {
            ui.set_min_size(vec2(CATEGORY_WIDTH, height));
            ui.set_max_width(CATEGORY_WIDTH);
            for cat in categories(layouts) {
                let on = state.category == cat;
                let label = tr(if cat == "All" { "All layouts" } else { cat });
                if ui
                    .with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                        ui.add(egui::Button::selectable(on, label).min_size(vec2(CATEGORY_WIDTH, 24.0)))
                    })
                    .inner
                    .clicked()
                    && !on
                {
                    state.category = cat.to_string();
                    if !state.shown(layouts).iter().any(|l| l.id == state.layout)
                        && let Some(first) = state.shown(layouts).first()
                    {
                        state.layout = first.id.to_string();
                    }
                }
            }
        });
        separator(ui, height, border);
        // The layouts as pictures.
        ui.vertical(|ui| {
            // A fixed size, whatever the category: the columns beside it never move.
            let width = 3.0 * TILE + 2.0 * 8.0 + 12.0;
            ui.set_min_size(vec2(width, height));
            ui.set_max_width(width);
            egui::ScrollArea::vertical().id_salt("krub_diagram_picker").max_height(height).auto_shrink([false, true]).show(ui, |ui| {
                ui.set_width(width - 12.0);
                let shown = state.shown(layouts);
                for row in shown.chunks(3) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                        for l in row {
                            let name = tr(l.name);
                            let on = state.layout == l.id;
                            // Not the app's strong selection fill: the picture and its name stay readable on a pale one.
                            // The hidden label (the accessible name) is cut short so a long name never widens the tile.
                            let resp = ui.add_sized(
                                vec2(TILE, TILE),
                                egui::Button::selectable(false, RichText::new(name.as_str()).color(Color32::TRANSPARENT)).truncate(),
                            );
                            let r = resp.rect;
                            let accent = ui.visuals().selection.bg_fill;
                            if on {
                                ui.painter().rect_filled(r, 4.0, accent.gamma_multiply(0.18));
                            }
                            let picture = Rect::from_min_size(r.min + vec2(6.0, 6.0), vec2(TILE - 12.0, 54.0));
                            paint(ui.painter(), picture, l.id);
                            tile_label(ui.painter(), r, &name, text);
                            if on {
                                ui.painter().rect_stroke(r, 4.0, Stroke::new(2.0, accent), StrokeKind::Inside);
                            }
                            let resp = resp.on_hover_text(name.as_str());
                            if resp.clicked() {
                                state.layout = l.id.to_string();
                            }
                            if resp.double_clicked() {
                                state.layout = l.id.to_string();
                                insert = true;
                            }
                        }
                    });
                    ui.add_space(8.0);
                }
            });
        });
        separator(ui, height, border);
        // The picked layout, large, and the items.
        ui.vertical(|ui| {
            ui.set_min_size(vec2(PREVIEW_WIDTH, height));
            ui.set_max_width(PREVIEW_WIDTH);
            let picked = layouts.iter().find(|l| l.id == state.layout);
            let (r, _) = ui.allocate_exact_size(vec2(PREVIEW_WIDTH, 120.0), egui::Sense::hover());
            ui.painter().rect(r, 4.0, ui.visuals().extreme_bg_color, Stroke::new(1.0, border), StrokeKind::Inside);
            if let Some(l) = picked {
                paint(ui.painter(), r.shrink(10.0), l.id);
                ui.add_space(4.0);
                ui.label(RichText::new(tr(l.name)).strong());
                ui.add(egui::Label::new(RichText::new(tr(l.description)).color(dim)).wrap());
            }
            ui.add_space(8.0);
            ui.label(tr("Diagram text"));
            ui.add(
                egui::TextEdit::multiline(&mut state.text)
                    .id_salt("krub_diagram_text")
                    .hint_text(tr("Leave empty for sample text"))
                    .lock_focus(true)
                    .desired_rows(5)
                    .desired_width(PREVIEW_WIDTH),
            );
            ui.add(egui::Label::new(RichText::new(tr("One item per line. Start a line with Tab for a sub-point.")).small().color(dim)).wrap());
        });
    });
    ui.separator();
    let mut outcome = Outcome::Open;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if ui.button(tr("Cancel")).clicked() {
            outcome = Outcome::Cancel;
        }
        if ui.button(tr("OK")).clicked() {
            outcome = Outcome::Insert;
        }
    });
    if insert { Outcome::Insert } else { outcome }
}

/// A thin vertical line `height` points tall.
fn separator(ui: &mut Ui, height: f32, color: Color32) {
    let (r, _) = ui.allocate_exact_size(vec2(9.0, height), egui::Sense::hover());
    ui.painter().line_segment([r.center_top(), r.center_bottom()], Stroke::new(1.0, color));
}

/// A tile's name under its picture; a name wider than the tile breaks at the space nearest its
/// middle (Thai names without spaces are cut short by the tile; the tooltip shows them whole).
fn tile_label(p: &Painter, tile: Rect, label: &str, color: Color32) {
    let font = FontId::proportional(11.0);
    let width = |s: &str| p.layout_no_wrap(s.to_string(), font.clone(), color).size().x;
    let room = tile.width() - 6.0;
    let lines: Vec<String> = if width(label) <= room {
        vec![label.to_string()]
    } else {
        label
            .match_indices(' ')
            .filter_map(|(i, _)| Some((label.get(..i)?.to_string(), label.get(i + 1..)?.to_string())))
            .min_by(|a, b| width(&a.0).max(width(&a.1)).total_cmp(&width(&b.0).max(width(&b.1))))
            .map_or_else(|| vec![label.to_string()], |(a, b)| vec![a, b])
    };
    let clip = p.with_clip_rect(tile.shrink(2.0));
    for (n, line) in lines.iter().rev().enumerate() {
        clip.text(pos2(tile.center().x, tile.bottom() - 11.0 - 13.0 * n as f32), Align2::CENTER_CENTER, line, font.clone(), color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAYOUTS: [Layout; 3] = [
        Layout { id: "a", name: "A", category: "List", description: "" },
        Layout { id: "b", name: "B", category: "Process", description: "" },
        Layout { id: "c", name: "C", category: "Process", description: "" },
    ];

    #[test]
    fn items_take_their_level_from_leading_tabs() {
        assert_eq!(
            items("Plan\n\tWrite\n\n  Build  \n\t\t\t\t\t\t\t\t\tDeep"),
            [("Plan".to_string(), 0), ("Write".to_string(), 1), ("Build".to_string(), 0), ("Deep".to_string(), MAX_LEVEL)]
        );
        assert!(items(" \n\t\n").is_empty());
    }

    #[test]
    fn only_used_categories_are_shown() {
        assert_eq!(categories(&LAYOUTS), ["All", "List", "Process"]);
        let mut s = State::new("a");
        s.category = "Process".into();
        assert_eq!(s.shown(&LAYOUTS).iter().map(|l| l.id).collect::<Vec<_>>(), ["b", "c"]);
    }

    #[test]
    fn the_dialog_draws_and_keeps_a_shown_layout_picked() {
        let ctx = egui::Context::default();
        let mut s = State::new("missing");
        s.category = "Matrix".into();
        let mut painted = Vec::new();
        let mut out = Outcome::Cancel;
        let mut frame = ctx.run_ui(egui::RawInput::default(), |ui| {
            out = show(ui, &|t| t.to_string(), &LAYOUTS, &mut s, 300.0, &mut |_, _, id| painted.push(id.to_string()));
        });
        frame.textures_delta.clear();
        assert_eq!(out, Outcome::Open);
        assert_eq!(s.category, "All");
        assert_eq!(s.layout, "a");
        assert!(painted.iter().any(|id| id == "c"), "every tile is painted: {painted:?}");
    }
}
