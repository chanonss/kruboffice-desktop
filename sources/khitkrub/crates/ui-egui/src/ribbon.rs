//! Title bar (Quick Access Toolbar), ribbon tabs and ribbon groups, and keyboard shortcuts.

use egui::{Align2, Color32, CornerRadius, Key, Modifiers, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use gridcraft_engine::model::{HAlign, Style, VAlign};
use serde_json::json;

use crate::SheetApp;
use crate::icons::{self, Icon};
use crate::keytips::keytip_table;
use crate::theme::{self, Cat, Tokens};
use crate::widgets::{
    big_button, big_split_button, color_palette, icon_button, small_button, small_split_button, small_toggle_button, split_button, toggle_button,
};

mod automate;
mod contextual;
mod controls;
mod data;
mod formulas;
mod galleries;
mod home;
mod insert;
mod page_layout;
mod review;
mod shortcuts;
mod title_bar;
mod view;

use automate::automate;
use contextual::{chart_design, table_design};
pub(crate) use controls::{ComboResult, editable_combo};
use controls::{act, keytip};
use data::data;
use formulas::formulas;
pub use home::FONTS;
use home::home;
use insert::{draw, insert};
use page_layout::page_layout;
use review::review;
pub use shortcuts::shortcut;
pub use title_bar::{MAC_CONTENT_LEFT, MAC_TITLE_BAR, TITLE_LEFT_PAD, title_bar};
use title_bar::{TITLE_BAR_H, TITLE_CONTROL_H};
use view::view;

pub const TABS: &[&str] = &["Home", "Insert", "Draw", "Page Layout", "Formulas", "Data", "Review", "View", "Automate"];
const _: () = assert!(TITLE_CONTROL_H < TITLE_BAR_H, "title-bar controls keep a margin above and below");

/// Height of the teal tab strip, which runs on from the title bar. The active tab reaches the
/// ribbon at its bottom edge.
const TAB_STRIP_H: f32 = 32.0;

/// The active tab: a tab with rounded top corners whose bottom corners curve outward into the ribbon
/// below, so the tab and the ribbon read as one shape. `bottom` is the strip's lower edge.
fn active_tab_shape(painter: &egui::Painter, tab: Rect, bottom: f32, fill: Color32, band: Color32) {
    const R: f32 = 8.0;
    let body = Rect::from_min_max(tab.min, pos2(tab.max.x, bottom));
    painter.rect_filled(body, CornerRadius { nw: R as u8, ne: R as u8, sw: 0, se: 0 }, fill);
    for left in [true, false] {
        let x = if left { tab.min.x - R } else { tab.max.x };
        let flare = Rect::from_min_size(pos2(x, bottom - R), vec2(R, R));
        let clipped = painter.with_clip_rect(flare);
        clipped.rect_filled(flare, CornerRadius::ZERO, fill);
        let centre = pos2(if left { flare.min.x } else { flare.max.x }, flare.min.y);
        clipped.circle_filled(centre, R, band);
    }
}

/// Draws the ribbon tabs and the active tab's horizontally scrollable card, resetting an unavailable contextual tab to Home.
pub fn show(app: &mut SheetApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let b = t.on_band();
    let collapsed = app.ui.ribbon_collapsed;
    let h = if collapsed { 34.0 } else { 34.0 + 80.0 };
    // A contextual tab exists only while its object is selected (Excel hides Table Design once the
    // cell leaves the table); a remembered one that is gone would draw an empty ribbon.
    if CONTEXTUAL_TABS.contains(&app.ui.ribbon_tab.as_str()) && !contextual_tabs(app).contains(&app.ui.ribbon_tab.as_str()) {
        app.ui.ribbon_tab = "Home".to_string();
    }
    // The Draw tab is off by default; a keytip or a remembered tab must not show it without its tab.
    if app.ui.ribbon_tab == "Draw" && !app.ui.show_draw_tab {
        app.ui.ribbon_tab = "Home".to_string();
    }
    egui::Panel::top("ribbon")
        .exact_size(h)
        .frame(egui::Frame::NONE.fill(if collapsed { t.band } else { t.ribbon }).inner_margin(egui::Margin {
            left: 10,
            right: 10,
            top: 0,
            bottom: 6,
        }))
        .show_separator_line(false)
        .show(ui, |ui| {
            // The teal band (the tab strip) runs on from the title bar across the whole window; the
            // ribbon below it is flat, so the active tab runs on into it.
            let screen = ui.ctx().content_rect();
            let top = ui.cursor().min.y;
            let strip = Rect::from_min_max(pos2(screen.left(), top), pos2(screen.right(), top + TAB_STRIP_H));
            ui.painter().rect_filled(strip, CornerRadius::ZERO, t.band);
            if !collapsed {
                let bottom = top + h;
                ui.painter().line_segment([pos2(screen.left(), bottom - 0.5), pos2(screen.right(), bottom - 0.5)], Stroke::new(1.0, t.ribbon_border));
            }

            let tabs_rect = Rect::from_min_max(pos2(screen.left() + 8.0, strip.top()), pos2(screen.right() - 8.0, strip.bottom()));
            let action_left = ribbon_actions(app, ui, tabs_rect, &b);
            let file_end = if app.chrome.file_tab {
                let file_rect = Rect::from_min_size(tabs_rect.min, vec2(72.0, 30.0));
                ui.scope_builder(egui::UiBuilder::new().max_rect(file_rect).layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| {
                    file_menu(app, ui)
                })
                .inner
                    + 4.0
            } else {
                tabs_rect.left()
            };
            let ctx_tabs = contextual_tabs(app);
            let tabs: Vec<_> = TABS.iter().copied().filter(|t| *t != "Draw" || app.ui.show_draw_tab).chain(ctx_tabs.iter().copied()).collect();
            let lang = app.ui.language;
            let show_numbers = app.ui.show_ribbon_tab_numbers;
            let font = theme::ui_font(13.5);
            let metrics: Vec<_> = tabs
                .iter()
                .map(|tab| {
                    let width = ui.painter().layout_no_wrap(lang.tr(tab).to_string(), theme::ui_bold(13.5), b.text).size().x + 20.0;
                    let priority = if app.ui.ribbon_tab == *tab { 2 } else { u8::from(ctx_tabs.contains(tab)) };
                    (width + if show_numbers { crate::ribbon_layout::TAB_BADGE_SPACE } else { 0.0 }, priority)
                })
                .collect();
            let tab_row = Rect::from_min_size(pos2(tabs_rect.left(), strip.top() + 1.0), vec2(tabs_rect.width(), 30.0));
            let (slots, overflow) = crate::ribbon_layout::tab_slots(tab_row, file_end, action_left - 4.0, &metrics, app.ui.ribbon_tabs_centered);
            for (index, (&tab, slot)) in tabs.iter().zip(&slots).enumerate() {
                let Some(r) = *slot else { continue };
                let active = app.ui.ribbon_tab == tab;
                let joined = active && !collapsed;
                let label = lang.tr(tab);
                let resp = ui.interact(r, ui.scope_id().with(("pilot_tab", tab)), Sense::click());
                resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, label));
                if joined {
                    active_tab_shape(ui.painter(), r, strip.bottom(), t.ribbon, t.band);
                } else if resp.hovered() {
                    ui.painter().rect_filled(r.shrink2(vec2(2.0, 4.0)), 5.0, b.hover);
                }
                let color = if joined {
                    if t.dark { t.text } else { t.band_ink }
                } else if ctx_tabs.contains(&tab) {
                    b.accent
                } else {
                    b.text
                };
                crate::ribbon_layout::tab_label(
                    ui.painter(),
                    r,
                    label,
                    if active { theme::ui_bold(13.5) } else { font.clone() },
                    color,
                    show_numbers.then_some((index + 1, t.band_ink)),
                );
                if active && !joined {
                    ui.painter().rect_filled(Rect::from_center_size(pos2(r.center().x, r.bottom() - 3.0), vec2(24.0, 3.0)), 1.5, b.accent);
                }
                if resp.clicked() {
                    app.ui.ribbon_tab = tab.to_string();
                    app.ui.ribbon_collapsed = active && !collapsed;
                }
                if resp.double_clicked() {
                    app.ui.ribbon_collapsed = !collapsed;
                }
                if app.keytips.prefix() == Some("")
                    && let Some(sequence) = keytip_table::tab_sequence(tab)
                {
                    keytip(app, ui, sequence, pos2(r.center().x, r.bottom() - 1.0));
                }
            }
            if let Some(rect) = overflow {
                let response = ui.interact(rect, ui.scope_id().with("more_tabs"), Sense::click());
                response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, lang.tr("More tabs")));
                ui.painter().text(rect.center(), Align2::CENTER_CENTER, "»", font, b.text);
                egui::Popup::menu(&response.on_hover_text(lang.tr("More tabs"))).show(|ui| {
                    for (&tab, slot) in tabs.iter().zip(slots) {
                        if slot.is_none() && ui.button(lang.tr(tab)).clicked() {
                            app.ui.ribbon_tab = tab.to_string();
                            app.ui.ribbon_collapsed = false;
                            ui.close();
                        }
                    }
                });
            }
            if collapsed {
                return;
            }
            // Anchor the content to KhianKrub's 8 px window margin, independently of the tabs'
            // frame padding. Keep it within the window even when the tab row is wider.
            let screen = ui.ctx().content_rect();
            let margin = crate::ribbon_layout::CONTENT_MARGIN;
            let card = Rect::from_min_size(
                pos2(screen.left() + margin, strip.bottom()),
                vec2((screen.width() - 2.0 * margin).max(0.0), crate::widgets::CONTENT_H + 16.0),
            );
            let inner = card.shrink2(vec2(0.0, 8.0));
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(inner).ui_stack_info(egui::UiStackInfo::default().with_tag(crate::ribbon_layout::RIBBON_COMMANDS)),
                |ui| {
                    // One scroll state per tab: a tab opened after scrolling another starts at its
                    // left edge, and a tab keeps its own offset while others are used.
                    let tab = app.ui.ribbon_tab.clone();
                    let mut area = egui::ScrollArea::horizontal()
                        .id_salt(("ribbon_scroll", tab.as_str()))
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
                    // egui 0.37 keeps `ScrollArea`'s offset private; `scroll_offset` is the supported
                    // way to move it. An arrow click records the requested offset for the next frame.
                    // Expire it if this tab stops rendering, rather than replaying it much later.
                    if let Some((frame, x)) = ui.ctx().data_mut(|d| d.remove_temp::<(u64, f32)>(RibbonOverflow::scroll_target_id(tab.as_str())))
                        && ui.ctx().cumulative_frame_nr().saturating_sub(frame) <= 1
                    {
                        area = area.horizontal_scroll_offset(x);
                    }
                    let scroll = area.show(ui, |ui| {
                        let _row = ui.horizontal_top(|ui| {
                            ui.set_min_height(crate::widgets::CONTENT_H);
                            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                            match app.ui.ribbon_tab.as_str() {
                                "Home" => home(app, ui),
                                "Insert" => insert(app, ui),
                                "Draw" => draw(app, ui),
                                "Page Layout" => page_layout(app, ui),
                                "Formulas" => formulas(app, ui),
                                "Data" => data(app, ui),
                                "Review" => review(app, ui),
                                "View" => view(app, ui),
                                "Automate" => automate(app, ui),
                                "Table Design" => table_design(app, ui),
                                "Chart Design" => chart_design(app, ui),
                                "Picture Format" | "Shape Format" => contextual::object_format(app, ui),
                                crate::paper_view::TAB => crate::paper_view::ribbon_tab(app, ui),
                                _ => home(app, ui),
                            }
                        });
                        // A group taller than the two rows makes the whole row taller.
                        #[cfg(test)]
                        if _row.response.rect.height() > crate::widgets::CONTENT_H + 0.5 {
                            crate::ribbon_layout::TALL_GROUPS.with(|g| g.borrow_mut().push(format!("whole tab: {:.1}", _row.response.rect.height())));
                        }
                    });
                    scroll_arrows(ui, &t, tab.as_str(), scroll);
                },
            );
        });
}

/// How much of the active tab's groups the ribbon card shows, recorded each frame for the scroll
/// arrows and for tests.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RibbonOverflow {
    /// Width of all the tab's groups.
    pub content: f32,
    /// Width of the card's content area.
    pub visible: f32,
    /// How far the groups are scrolled.
    pub offset: f32,
    /// Height of the tallest group (a column of small buttons) and of the content area.
    pub content_height: f32,
    pub visible_height: f32,
}

impl RibbonOverflow {
    /// Returns the stable id for the ribbon overflow state.
    fn id() -> egui::Id {
        egui::Id::unique("ribbon_overflow")
    }

    /// Where an arrow click asks the next frame's `ScrollArea` to scroll to.
    fn scroll_target_id(tab: &str) -> egui::Id {
        egui::Id::unique(("ribbon_scroll_target", tab))
    }

    /// Last frame's overflow.
    pub fn get(ctx: &egui::Context) -> RibbonOverflow {
        ctx.data(|d| d.get_temp(Self::id())).unwrap_or_default()
    }

    /// Whether the groups are wider than the card.
    pub fn overflows(&self) -> bool {
        self.content > self.visible + 0.5
    }
}

/// When the tab's groups are wider than the card, arrows at its edges scroll them (horizontal wheel or
/// trackpad input over the card still does too).
fn scroll_arrows(ui: &mut Ui, t: &Tokens, tab: &str, scroll: egui::scroll_area::ScrollAreaOutput<()>) {
    let inner = scroll.inner_rect;
    let max = (scroll.content_size.x - inner.width()).max(0.0);
    let overflow = RibbonOverflow {
        content: scroll.content_size.x,
        visible: inner.width(),
        offset: scroll.state.clamped_offset().x,
        content_height: scroll.content_size.y,
        visible_height: inner.height(),
    };
    ui.ctx().data_mut(|d| d.insert_temp(RibbonOverflow::id(), overflow));
    if !overflow.overflows() {
        return;
    }
    let step = (inner.width() * 0.6).max(60.0);
    let mut target = None;
    for (left, show) in [(true, overflow.offset > 0.5), (false, overflow.offset < max - 0.5)] {
        if !show {
            continue;
        }
        let r = if left {
            Rect::from_min_size(inner.left_top(), vec2(18.0, inner.height()))
        } else {
            Rect::from_min_size(pos2(inner.right() - 18.0, inner.top()), vec2(18.0, inner.height()))
        };
        let resp = ui.interact(r, ui.scope_id().with(("ribbon_scroll_arrow", left)), Sense::click());
        ui.painter().rect_filled(r, 4.0, if resp.hovered() { t.hover } else { t.ribbon });
        ui.painter().rect_stroke(r, 4.0, Stroke::new(1.0, t.ribbon_border), StrokeKind::Inside);
        icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(14.0, 14.0)), if left { Icon::Left } else { Icon::Right }, t.text);
        let name = if left { "Scroll ribbon left" } else { "Scroll ribbon right" };
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, name));
        if resp.on_hover_text(crate::i18n::current(ui.ctx()).tr(name)).clicked() {
            target = Some(if left { overflow.offset - step } else { overflow.offset + step });
        }
    }
    if let Some(x) = target {
        let frame = ui.ctx().cumulative_frame_nr();
        ui.ctx().data_mut(|d| d.insert_temp(RibbonOverflow::scroll_target_id(tab), (frame, x.clamp(0.0, max))));
        ui.ctx().request_repaint();
    }
}

/// The File tab opens the Backstage view (New, Open, Info, Save As, Print, Export, Options…), like
/// Excel's; the ribbon's tab and collapsed state are left as they were for when it closes.
fn file_menu(app: &mut SheetApp, ui: &mut Ui) -> f32 {
    let b = Tokens::get(ui.ctx()).on_band();
    let name = app.ui.language.tr("File");
    let font = theme::ui_font(13.5);
    let tw = ui.painter().layout_no_wrap(name.to_string(), font.clone(), b.text).size().x;
    let (r, file) = ui.allocate_exact_size(vec2((tw + 22.0).max(46.0), 30.0), Sense::click());
    file.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, name));
    if file.hovered() {
        ui.painter().rect_filled(r.shrink2(vec2(2.0, 4.0)), 5.0, b.hover);
    }
    ui.painter().text(r.center(), Align2::CENTER_CENTER, name, font, b.text);
    if file.clicked() {
        app.open_backstage(crate::backstage::Page::Home);
    }
    if app.keytips.prefix() == Some("")
        && let Some(sequence) = keytip_table::tab_sequence("File")
    {
        keytip(app, ui, sequence, pos2(file.rect.center().x, file.rect.bottom() - 1.0));
    }
    r.right()
}

fn ribbon_actions(app: &mut SheetApp, ui: &mut Ui, strip: Rect, b: &Tokens) -> f32 {
    let lang = app.ui.language;
    let right = strip.right();
    let action = |offset: f32, icon, label: &str, primary| {
        let rect = Rect::from_center_size(pos2(right - offset, strip.center().y), vec2(28.0, 24.0));
        let response = ui.interact(rect, ui.scope_id().with(("ribbon_action", label)), Sense::click());
        let fill = if primary {
            if response.hovered() { b.accent.gamma_multiply(0.85) } else { b.accent }
        } else if response.hovered() {
            b.hover
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 4.0, fill);
        icons::paint_flat(ui.painter(), rect.shrink2(vec2(6.0, 4.0)), icon, if primary { b.on_accent } else { b.text }, b.band);
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, lang.tr(label)));
        response.on_hover_text(lang.tr(label))
    };
    let share = action(14.0, Icon::Share, "Share", true);
    let comments = action(46.0, Icon::Comment, "Comments", false);
    controls::control_keytip(app, ui, &comments, "comments");
    if comments.clicked() {
        app.open_dialog("comments", json!({}));
    }
    egui::Popup::menu(&share).show(|ui| {
        if ui.button(lang.tr("Export as PDF…")).clicked() {
            app.run_or_alert("file.exportPdf", json!({}));
        }
        for (label, format) in
            [("Save a Copy as Excel Workbook (.xlsx)…", "xlsx"), ("Export as CSV…", "csv"), ("Export as Web Page (.html)…", "html")]
        {
            if ui.button(lang.tr(label)).clicked() {
                app.open_dialog("saveCopy", json!({"format": format}));
            }
        }
    });
    right - 60.0
}

/// System / Light / Dark, as in the View tab's Display theme box (`view.theme`).
fn theme_choices(app: &mut SheetApp, ui: &mut Ui) {
    let lang = app.ui.language;
    let mode = app.ui.theme_mode();
    for (value, label) in [("system", "System"), ("light", "Light"), ("dark", "Dark")] {
        if ui.radio(mode == value, lang.tr(label)).clicked() {
            act(app, "view.theme", json!({"mode": value}));
            ui.close();
        }
    }
}

const CONTEXTUAL_TABS: [&str; 5] = ["Table Design", "Chart Design", "Picture Format", "Shape Format", crate::paper_view::TAB];

fn contextual_tabs(app: &SheetApp) -> Vec<&'static str> {
    let mut v = Vec::new();
    if let Some(d) = app.session.active()
        && let Some(sh) = d.wb.active()
    {
        if sh.table_at(d.selection.active).is_some() {
            v.push("Table Design");
        }
        if app.selected_chart().is_some_and(|id| sh.charts.iter().any(|c| c.id == id)) {
            v.push("Chart Design");
        }
        match app.selected_object.filter(|o| sh.object_anchor(*o).is_some()).map(|o| o.kind) {
            Some(gridcraft_engine::model::ObjKind::Image) => v.push("Picture Format"),
            Some(gridcraft_engine::model::ObjKind::Shape) => v.push("Shape Format"),
            _ => {}
        }
        if app.grid.hf_edit.is_some() {
            v.push(crate::paper_view::TAB);
        }
    }
    v
}

#[cfg(test)]
mod fit_tests;

#[cfg(test)]
mod tooltip_tests {
    /// A ribbon tooltip names its control only; `widgets::tooltip_shortcut` adds the shortcut of
    /// the platform it runs on. A Command-key glyph written into the text would show on Windows
    /// and Linux, where the shortcut is a Ctrl or Alt one (AutoSum is Alt+=).
    #[test]
    fn tooltips_leave_the_shortcut_to_the_platform_lookup() {
        let sources = [
            include_str!("ribbon.rs"),
            include_str!("ribbon/automate.rs"),
            include_str!("ribbon/contextual.rs"),
            include_str!("ribbon/controls.rs"),
            include_str!("ribbon/data.rs"),
            include_str!("ribbon/formulas.rs"),
            include_str!("ribbon/galleries.rs"),
            include_str!("ribbon/home.rs"),
            include_str!("ribbon/insert.rs"),
            include_str!("ribbon/page_layout.rs"),
            include_str!("ribbon/review.rs"),
            include_str!("ribbon/shortcuts.rs"),
            include_str!("ribbon/title_bar.rs"),
            include_str!("ribbon/view.rs"),
        ];
        let mac_only = sources.iter().flat_map(|s| s.lines()).filter(|l| l.contains('\u{2318}')).collect::<Vec<_>>();
        assert!(mac_only.is_empty(), "Command-key glyph in ribbon text: {mac_only:?}");
    }
}
