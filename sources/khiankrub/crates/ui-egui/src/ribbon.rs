//! The ribbon: tab strip and every tab's groups. Each control runs a command by id.

use crate::widgets::SteadyUi;
use egui::{Align2, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::colors::{Remember, Target};
use crate::options::MeasureExt;
use crate::theme::{Tokens, medium, semibold};
use crate::widgets::{CONTENT_H, big, big_toggle, combo, group, menu_button, small, split};
use crate::{WordApp, icons};

pub use wordcraft_engine::ribbon::TABS;

/// True when the caret/selection touches a picture (#147).
pub fn has_picture_selected(s: &wordcraft_engine::Session) -> bool {
    matches!(wordcraft_engine::cmd::objects::selected(s), Some((_, wordcraft_doc::para::InlineObject::Image { .. })))
}

/// Whether the selection is a shape, text box or Drawing Canvas (so Shape Format is shown).
pub fn has_shape_selected(s: &wordcraft_engine::Session) -> bool {
    match wordcraft_engine::cmd::objects::selected(s) {
        Some((_, o)) => matches!(o, wordcraft_doc::para::InlineObject::Shape { .. }) || o.is_canvas(),
        None => false,
    }
}

/// Reading controls replace editing tabs entirely. All settings are transient view controls.
fn reading_toolbar(app: &mut WordApp, ui: &mut Ui, t: Tokens) {
    egui::Panel::top("reading_toolbar")
        .exact_size(46.0)
        .frame(egui::Frame::NONE.fill(t.ribbon).inner_margin(egui::Margin::symmetric(12, 6)).stroke(Stroke::new(1.0, t.border)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(egui::RichText::new(tl!("Reading Mode")).font(semibold(14.0)));
                ui.separator();
                let w = app.session.view.read_column_width;
                let active = match w {
                    None => "Auto",
                    Some(v) if v <= 180.0 => "Narrow",
                    Some(v) if v <= 240.0 => "Medium",
                    Some(v) if v <= 350.0 => "Wide",
                    _ => "Page",
                };
                ui.menu_button(format!("{}: {}", tl!("Column width"), tl!(active)), |ui| {
                    for (name, value) in [("Auto", "auto"), ("Narrow", "narrow"), ("Medium", "medium"), ("Wide", "wide"), ("Page", "page")] {
                        if ui.steady_label(active == name, tl!(name)).clicked() {
                            let _ = app.run("view.readColumnWidth", json!({"value": value}));
                            ui.close();
                        }
                    }
                });
                ui.separator();
                let pct = (app.session.view.read_zoom * 100.0).round() as u32;
                ui.menu_button(format!("{}: {pct}%", tl!("Text size")), |ui| {
                    for percent in [75, 90, 100, 110, 125, 150] {
                        if ui.steady_label(pct == percent, format!("{percent}%")).clicked() {
                            let _ = app.run("view.readTextSize", json!({"value": percent}));
                            ui.close();
                        }
                    }
                });
                ui.separator();
                let spacing = app.session.view.read_line_spacing;
                let label = spacing.map_or_else(|| tl!("Document").to_owned(), |v| format!("{v}×"));
                ui.menu_button(format!("{}: {}", tl!("Line spacing"), label), |ui| {
                    if ui.steady_label(spacing.is_none(), tl!("Document")).clicked() {
                        let _ = app.run("view.readLineSpacing", json!({"value": "document"}));
                        ui.close();
                    }
                    for value in [1.0, 1.25, 1.5, 2.0] {
                        if ui.steady_label(spacing == Some(value), format!("{value}×")).clicked() {
                            let _ = app.run("view.readLineSpacing", json!({"value": value}));
                            ui.close();
                        }
                    }
                });
                ui.separator();
                // Headings to jump to, in the Navigation pane beside the columns (Outline view
                // is an editing layout, so it isn't offered here).
                if ui.button(tl!("Navigation Pane")).clicked() {
                    let _ = app.run("view.navigationPane", json!({"value": true}));
                    app.ui.nav_tab = "headings".into();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tl!("Edit Document")).clicked() {
                        let _ = app.run("view.printLayout", json!({}));
                    }
                });
            });
        });
}

/// Whether the selection is a SmartArt graphic WordCraft can edit (so SmartArt Design is shown).
pub fn has_smart_art_selected(s: &wordcraft_engine::Session) -> bool {
    wordcraft_engine::cmd::smart_art::selected_smart_art(s).is_some()
}

/// Whether the selection is a chart WordCraft can edit (so Chart Design is shown).
pub fn has_chart_selected(s: &wordcraft_engine::Session) -> bool {
    wordcraft_engine::cmd::chart::selected_chart(s).is_some()
}

/// Contextual tabs for the current selection (pure, tested).
pub fn contextual_tabs(s: &wordcraft_engine::Session) -> Vec<&'static str> {
    let mut tabs = Vec::new();
    // A table's commands are in the Table pane (`table_pane`), not in contextual tabs.
    // A picture's, shape's, chart's and SmartArt graphic's commands are in their panes
    // (`object_pane`).
    // A header's or footer's commands are in the Header & Footer pane (`hf_pane`, 2026-11-19).
    // Editing an equation.
    if s.math.is_some() {
        tabs.push("Equation");
    }
    // Outline view (drawn first, next to File, as Word does).
    if s.view.mode == wordcraft_layout::ViewMode::Outline {
        tabs.push("Outlining");
    }
    tabs
}

/// Tab to show when the stored tab is no longer applicable (pure, tested).
pub fn resolve_tab<'a>(current: &'a str, available: &[&str]) -> &'a str {
    if available.contains(&current) { current } else { crate::DEFAULT_TAB }
}

/// Width of the » button that holds the tabs the strip has no room for.
const MORE_TABS_W: f32 = 24.0;

/// Which tabs fit on a strip `avail` points wide (#897), from each tab's width and whether it must
/// stay. The others give way from the end, leaving `more` points for the » menu that holds them
/// (pure, tested).
pub fn fit_tabs(tabs: &[(f32, bool)], avail: f32, more: f32) -> Vec<bool> {
    let mut fits = vec![true; tabs.len()];
    let mut total: f32 = tabs.iter().map(|(w, _)| w).sum();
    if total <= avail {
        return fits;
    }
    for (fit, (w, keep)) in fits.iter_mut().zip(tabs).rev() {
        if total + more <= avail {
            break;
        }
        if !keep {
            *fit = false;
            total -= w;
        }
    }
    fits
}

const TAB_NUMBER_SPACE: f32 = 21.0;

/// Whether the caret is inside a table (so the contextual tabs and their badges are shown).
pub fn in_table_public(app: &WordApp) -> bool {
    app.session.sel.focus.path.cell().is_some()
}

/// The tab strip (the one bar) and the ribbon under it.
pub fn show(app: &mut WordApp, ui: &mut Ui) {
    show_tabs(app, ui);
    show_body(app, ui);
}

/// The one bar: the title bar and the tab strip in one row, across the whole window.
pub fn show_tabs(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    if app.session.view.read_mode {
        reading_toolbar(app, ui, t);
        return;
    }
    // Editing an equation brings up the Equation tab; leaving it goes back.
    if app.session.math.is_some() {
        if app.equation_prev_tab.is_none() {
            app.equation_prev_tab = Some(app.ui.tab.clone());
            app.ui.tab = "Equation".into();
        }
    } else if let Some(prev) = app.equation_prev_tab.take()
        && app.ui.tab == "Equation"
    {
        app.ui.tab = prev;
    }
    // A Header & Footer tab remembered from before the pane (2026-11-19) goes back.
    if let Some(prev) = app.hf_prev_tab.take()
        && app.ui.tab == "Header & Footer"
    {
        app.ui.tab = prev;
    }
    // Outline view brings up the Outlining tab; closing it goes back.
    if app.session.view.mode == wordcraft_layout::ViewMode::Outline {
        if app.outline_prev_tab.is_none() {
            app.outline_prev_tab = Some(app.ui.tab.clone());
            app.ui.tab = "Outlining".into();
        }
    } else if let Some(prev) = app.outline_prev_tab.take()
        && app.ui.tab == "Outlining"
    {
        app.ui.tab = prev;
    }
    // The tabs shown: File, the user's tabs in their order (Customize Ribbon, #379), then the
    // contextual ones (the Outlining tab right after File). A contextual tab (Table, Picture
    // Format) may be stored while the selection moved away, and a tab may have been hidden.
    let mut user_tabs = app.session.ribbon.tabs().into_owned();
    user_tabs.retain(|t| !cfg!(target_arch = "wasm32") || t.name != "Zotero");
    let mut tabs: Vec<(String, bool)> = vec![("File".to_string(), false)];
    tabs.extend(user_tabs.iter().filter(|e| !e.hidden).map(|e| (e.name.clone(), e.custom)));
    for ct in contextual_tabs(&app.session) {
        if ct == "Outlining" {
            tabs.insert(1.min(tabs.len()), (ct.to_string(), false));
        } else if !tabs.iter().any(|(n, _)| n == ct) {
            tabs.push((ct.to_string(), false));
        }
    }
    {
        let names: Vec<&str> = tabs.iter().map(|(n, _)| n.as_str()).collect();
        let next = resolve_tab(&app.ui.tab, &names);
        let next = if names.contains(&next) { next } else { names.iter().copied().find(|n| *n != "File").unwrap_or(next) };
        if next != app.ui.tab {
            app.ui.tab = next.to_string();
        }
    }
    // The one bar (owner, 2026-11-18): the title bar and the tab strip in one row, a little room
    // above the tabs; on macOS the tabs start after the traffic lights.
    let mac = app.integrated_titlebar;
    let left = if mac { app.mac_titlebar.content_left as i8 } else { 8 };
    egui::Panel::top("tabs")
        .exact_size(krub_icons::chrome::ONE_BAR_H)
        // No line under the strip: the active tab runs on into the ribbon.
        .show_separator_line(false)
        .frame(egui::Frame::NONE.fill(t.tab_strip).inner_margin(egui::Margin {
            left,
            right: 10,
            top: krub_icons::chrome::ONE_BAR_GAP as i8,
            bottom: 0,
        }))
        .show(ui, |ui| {
            // The navy band: white tab names; the active tab is a white tab joined to the ribbon
            // below it, its name in navy.
            crate::theme::band_ui(ui);
            let band = t.band();
            let (_, motif) = crate::theme::band(ui.ctx());
            let strip = ui.clip_rect();
            crate::band_motifs::set_band_rect(ui.ctx(), strip);
            crate::band_motifs::paint(ui.painter(), strip, motif, t.dark);
            crate::chrome::window_drag(ui, strip);
            let row = ui.max_rect();
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // The user icon (its menu has Search, Comments and the editing mode) and the
                    // Quick Access Toolbar, drawn first so the tabs get only the room left of them
                    // (#897); the file pill sits between the last tab and them.
                    let right = crate::chrome::bar_right(app, ui);
                    ui.add_space(8.0);
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        // The tabs that don't fit go into a » menu at the end of the strip. File, the
                        // active tab and the contextual tabs (appended last) stay on the strip (#897).
                        // UI experiment: the left panel's button opens the bar.
                        if crate::panes::panel_toggle(ui, app, false) {
                            app.session.view.nav_pane = !app.session.view.nav_pane;
                        }
                        ui.add_space(4.0);
                        let labels: Vec<&str> = tabs.iter().map(|(tab, custom)| if *custom { tab.as_str() } else { tl!(tab) }).collect();
                        let gap = ui.spacing().item_spacing.x;
                        let show_numbers = app.ui.show_ribbon_tab_numbers;
                        let widths: Vec<(f32, bool)> = tabs
                            .iter()
                            .zip(&labels)
                            .map(|((tab, custom), shown)| {
                                let font = if *tab == app.ui.tab { semibold(12.5) } else { medium(12.5) };
                                let label_width = ui.ctx().fonts_mut(|f| f.layout_no_wrap(shown.to_string(), font, t.text).size().x);
                                let number_width = if show_numbers && tab != "File" { TAB_NUMBER_SPACE } else { 0.0 };
                                let contextual = !custom && wordcraft_engine::ribbon::CONTEXTUAL_TABS.contains(&tab.as_str());
                                (label_width + 18.0 + number_width + gap, tab == "File" || *tab == app.ui.tab || contextual)
                            })
                            .collect();
                        // Save, Undo and Redo follow File (owner, 2026-11-18); the file pill keeps at
                        // least its narrowest width after the tabs.
                        let qat_w = crate::chrome::quick_access_width(app) + 8.0;
                        let pill_room = krub_icons::chrome::PILL_MIN_W + 2.0 * krub_icons::chrome::PILL_MARGIN;
                        let available = (ui.available_width() - pill_room - qat_w).max(0.0);
                        let mut fits = fit_tabs(&widths, available, MORE_TABS_W + gap);
                        let occupied = |fits: &[bool]| {
                            let visible: f32 = widths.iter().zip(fits).filter(|(_, fit)| **fit).map(|((w, _), _)| w).sum();
                            visible + if fits.contains(&false) { MORE_TABS_W + gap } else { 0.0 }
                        };
                        if occupied(&fits) > available {
                            // At narrow widths keep File and the selected tab. Contextual tabs can
                            // still be reached through the overflow menu.
                            let essential: Vec<_> =
                                widths.iter().zip(&tabs).map(|((w, _), (tab, _))| (*w, tab == "File" || *tab == app.ui.tab)).collect();
                            fits = fit_tabs(&essential, available, MORE_TABS_W + gap);
                        }
                        let file_width = widths.first().map_or(0.0, |(width, _)| *width);
                        let tab_width = (occupied(&fits) - file_width).max(0.0);
                        let first_tab_x = ui.cursor().min.x + file_width + qat_w;
                        // UI experiment: centred on the document column between the side panels.
                        let (left_w, right_w) = crate::panes::panel_widths(app, ui.ctx());
                        let doc_centre = strip.min.x + (left_w + strip.width() - right_w) / 2.0;
                        let desired_start = doc_centre - tab_width / 2.0;
                        // Centred, KrubOffice's signature (kept on the owner's second look); off
                        // (File › Options › General) the tabs start right after the toolbar.
                        let offset = if app.ui.ribbon_tabs_centered {
                            (desired_start - first_tab_x).clamp(0.0, (available - file_width - tab_width).max(0.0))
                        } else {
                            0.0
                        };

                        app.keytip_tabs = tabs.iter().map(|(tab, _)| tab.clone()).collect();
                        for (index, ((((tab, custom), shown), (width, _)), fits)) in tabs.iter().zip(&labels).zip(&widths).zip(&fits).enumerate() {
                            if !fits {
                                continue;
                            }
                            let (tab, shown) = (tab.as_str(), *shown);
                            let contextual = !custom && wordcraft_engine::ribbon::CONTEXTUAL_TABS.contains(&tab);
                            let (r, resp) = ui.allocate_exact_size(vec2(width - gap, 30.0), Sense::click());
                            if app.ui.keytips == crate::keytips::Phase::Tabs {
                                crate::keytips::record_tab(r, tab, index, show_numbers, &app.ui.keytip_prefix, &mut app.keytip_rects);
                            }
                            let active = app.ui.tab == tab && !app.ui.backstage;
                            resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, active, shown));
                            if active {
                                // A tab of the ribbon's colour, from just below the strip's top
                                // down into the ribbon, so the two read as one.
                                let tab_r = Rect::from_min_max(pos2(r.min.x, r.min.y + 3.0), pos2(r.max.x, r.max.y + 1.0));
                                // Rounded top corners, and bottom corners that curve out into the
                                // ribbon with the same radius: a ribbon-coloured square beside each,
                                // with a band-coloured circle cut out of its upper outer corner.
                                const R: f32 = 8.0;
                                ui.painter().rect_filled(tab_r, egui::CornerRadius { nw: R as u8, ne: R as u8, sw: 0, se: 0 }, t.ribbon);
                                // The strip is cut off where the ribbon starts: curve up from there,
                                // or the ribbon hides the curve's lower part and it looks tighter.
                                let bottom = tab_r.max.y.min(ui.clip_rect().max.y);
                                // Each is a fan of triangles from the tab's foot to the quarter
                                // circle, so the band (and its motif) shows above the curve.
                                for (x, outer) in [(tab_r.min.x, tab_r.min.x - R), (tab_r.max.x, tab_r.max.x + R)] {
                                    let centre = pos2(outer, bottom - R);
                                    let mut mesh = egui::epaint::Mesh::default();
                                    mesh.colored_vertex(pos2(x, bottom), t.ribbon);
                                    const STEPS: u32 = 8;
                                    for i in 0..=STEPS {
                                        let a = std::f32::consts::FRAC_PI_2 * i as f32 / STEPS as f32;
                                        // From straight below the centre round to level with it, on the tab's side.
                                        let p = centre + vec2((x - outer).signum() * R * a.sin(), R * a.cos());
                                        mesh.colored_vertex(p, t.ribbon);
                                        if i > 0 {
                                            mesh.add_triangle(0, i, i + 1);
                                        }
                                    }
                                    ui.painter().add(mesh);
                                }
                            } else if resp.hovered() {
                                ui.painter().rect_filled(r.shrink2(vec2(0.0, 4.0)), 4.0, band.hover);
                            }
                            // The active tab's name in the app's navy (light blue in the dark theme); the
                            // others white; contextual tabs (Table Design…) in the icon's gold.
                            let color = if active {
                                if t.dark { t.accent_text } else { t.band_ink }
                            } else if contextual {
                                band.accent_text
                            } else {
                                band.text
                            };
                            let mut label_center = r.center();
                            if show_numbers && index > 0 {
                                let badge = pos2(r.left() + 17.0, r.center().y);
                                // Gold while choosing a tab: its number is the key (owner, 2026-11-19).
                                let lit = app.ui.keytips == crate::keytips::Phase::Tabs && app.ui.keytip_prefix.is_empty();
                                ui.painter().circle_filled(badge, 8.0, if lit { band.accent } else { egui::Color32::WHITE });
                                if active {
                                    ui.painter().circle_stroke(badge, 8.0, Stroke::new(0.75, t.border_strong));
                                }
                                ui.painter().text(badge, Align2::CENTER_CENTER, index.to_string(), semibold(9.0), t.band_ink);
                                label_center.x += TAB_NUMBER_SPACE / 2.0;
                            }
                            ui.painter().text(label_center, Align2::CENTER_CENTER, shown, if active { semibold(12.5) } else { medium(12.5) }, color);
                            if resp.clicked() {
                                let _ = app.run("ui.tab", json!({"tab": tab}));
                            }
                            if resp.double_clicked() && tab != "File" {
                                let _ = app.run("ui.collapseRibbon", json!({}));
                            }
                            resp.context_menu(|ui| crate::widgets::customize_menu_items(ui, app));
                            if index == 0 {
                                ui.add_space(4.0);
                                crate::chrome::bar_quick_access(app, ui);
                                ui.add_space(4.0 + offset);
                            }
                        }
                        let left_out: Vec<(&str, &str)> = tabs
                            .iter()
                            .zip(&labels)
                            .zip(&fits)
                            .filter(|(_, fits)| !**fits)
                            .map(|(((tab, _), shown), _)| (tab.as_str(), *shown))
                            .collect();
                        if !left_out.is_empty() {
                            let (r, resp) = ui.allocate_exact_size(vec2(MORE_TABS_W, 30.0), Sense::click());
                            if resp.hovered() {
                                ui.painter().rect_filled(r.shrink2(vec2(0.0, 4.0)), 4.0, t.hover);
                            }
                            ui.painter().text(r.center(), Align2::CENTER_CENTER, "»", medium(12.5), band.text);
                            resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tl!("More tabs")));
                            egui::Popup::menu(&resp).show(|ui| {
                                for (tab, shown) in left_out {
                                    if ui.steady_label(false, shown).clicked() {
                                        let _ = app.run("ui.tab", json!({"tab": tab}));
                                        ui.close();
                                    }
                                }
                            });
                        }
                        let tabs_end = ui.min_rect().max.x;
                        crate::chrome::file_pill(app, ui, krub_icons::chrome::pill_area(row, tabs_end, right - 8.0));
                    });
                });
            });
        });
}

/// What the one-line ribbon keeps of each tab (UI experiment, branch `ui-pages-layout`,
/// 2026-11-23): command ids or icon names; the rest of the tab is in the line's More menu and,
/// for settings, in the side panel. `*` keeps a tab whole (drawn small, on one line).
pub(crate) fn line_keys(tab: &str) -> &'static [&'static str] {
    match tab {
        "Home" => &[
            "edit.paste",
            "edit.formatPainter",
            "font",
            "size",
            "format.bold",
            "format.italic",
            "format.underline",
            "format.highlight",
            "format.color",
            "para.bullets",
            "para.numbering",
            "para.alignLeft",
            "para.alignCenter",
            "para.alignRight",
            "para.thaiDistribute",
            "edit.copy",
            "view.marks",
            "format.growFont",
            "format.superscript",
            "format.border",
            // With the ribbon full width (owner, 2026-11-24) these come back when there is room.
            "para.justify",
            "format.strikethrough",
            "para.lineSpacing",
            "format.clear",
            "para.outdent",
            "para.indent",
            "para_style_box",
        ],
        "Insert" => &[
            "insert.coverPage",
            "insert.pageBreak",
            "insert.table",
            "insert.picture",
            "insert.shape",
            "insert.icon",
            "insert.chart",
            "insert.smartArt",
            "link",
            "bookmark",
            "review.newComment",
            "insert.header",
            "insert.footer",
            "insert.pageNumber",
            "insert.textBox",
            "wordArt",
            "insert.dropCap",
            "insert.dateTime",
            "insert.equation",
            "insert.symbol",
        ],
        "Draw" => {
            &["draw.select", "draw.lasso", "draw.pen", "draw.pencil", "draw.highlighter", "draw.eraser", "addPen", "draw.inkToShape", "insert.canvas"]
        }
        "Design" => &[
            "design.themes",
            "design.themeColors",
            "design.themeFonts",
            "design.paragraphSpacing",
            "design.effects",
            "design.setDefault",
            "design.watermark",
            "design.pageColor",
            "design.pageBorders",
        ],
        "Layout" => &[
            "layout.textDirection",
            "layout.margins",
            "layout.orientation",
            "layout.size",
            "layout.columns",
            "layout.break",
            "layout.lineNumbers",
            "arrange.position",
            "arrange.bringForward",
            "arrange.sendBackward",
            "arrange.selectionPane",
            "arrange.align",
        ],
        "References" => &[
            "toc",
            "references.updateLists",
            "references.addText",
            "references.footnote",
            "references.endnote",
            "references.nextFootnote",
            "references.notes",
            "citation",
            "references.sources",
            "references.citationStyle",
            "references.bibliography",
            "references.caption",
            "insert.crossReference",
            "references.markEntry",
            "references.index",
        ],
        // The Equation tab on one line (owner, 2026-11-24, draft A): the tools, Symbols (a
        // panel), the structures as small picture buttons, then the equation's options.
        "Equation" => &[
            "eq.builtin",
            "equation.inputFormat",
            "equation.convert",
            "equation.normalText",
            "eq.symbols",
            "eq.fraction",
            "eq.script",
            "eq.radical",
            "eq.integral",
            "eq.largeOperator",
            "eq.bracket",
            "eq.function",
            "eq.accent",
            "eq.limitLog",
            "eq.operator",
            "eq.matrix",
            "equation.display",
            "equation.number",
            "equation.justify",
            "equation.exit",
        ],
        // The Outlining tab (Outline view) on one line: each control once, in the tab's order.
        "Outlining" => &[
            "outline.promoteToHeading1",
            "outline.promote",
            "outlineLevel",
            "outline.demote",
            "outline.demoteToBody",
            "outline.moveUp",
            "outline.moveDown",
            "outline.expand",
            "outline.collapse",
            "outlineShowLevel",
            "outline.showFormatting",
            "outline.firstLineOnly",
            "outline.close",
        ],
        "Mailings" => &["mailings.start", "mailings.recipients", "mailings.insertField", "mailings.preview", "mailings.finish"],
        "Review" => &[
            "review.spelling",
            "review.thesaurus",
            "wordCount",
            "review.readAloud",
            "file.accessibility",
            "review.newComment",
            "review.deleteComment",
            "review.previousComment",
            "review.nextComment",
            "view.commentsPane",
            "review.trackChanges",
            "review.accept",
            "review.reject",
            "review.previousChange",
            "review.nextChange",
            "review.compare",
            "review.restrict",
        ],
        "View" => &[
            "view.readMode",
            "view.printLayout",
            "view.webLayout",
            "view.outline",
            "view.focus",
            "view.immersive",
            "view.navigationPane",
            "view.ruler",
            "view.gridlines",
            "view.textBoundaries",
            "zoom",
            "view.zoomOut",
            "view.zoomIn",
            "view.onePage",
            "view.multiplePages",
            "view.pageWidth",
            "view.darkMode",
        ],
        _ => &["*"],
    }
}

/// Draw the current tab's groups (and the custom groups the user added to it).
fn draw_tab(app: &mut WordApp, ui: &mut Ui, entry: Option<&wordcraft_engine::ribbon::TabEntry>) {
    match entry {
        // Every tab hidden: nothing to show.
        Some(e) if e.hidden => return,
        Some(e) if e.custom => return custom_groups(app, ui, &e.groups),
        _ => {}
    }
    // Accent colours (owner, 2026-11-24: style A), tab by tab as each is approved.
    let ctx = ui.ctx().clone();
    // Every built-in tab is coloured now (2026-11-25: Draw, Mailings, Developer, Zotero, Help,
    // Outlining and Equation were the last).
    krub_icons::tabler_colour::with_colour(&ctx, true, || draw_tab_body(app, ui));
    if let Some(e) = entry {
        custom_groups(app, ui, &e.groups);
    }
}

fn draw_tab_body(app: &mut WordApp, ui: &mut Ui) {
    match app.ui.tab.as_str() {
        "Home" => home(app, ui),
        "Insert" => insert(app, ui),
        "Draw" => draw(app, ui),
        "Design" => design(app, ui),
        "Layout" => layout(app, ui),
        "References" => references(app, ui),
        "Mailings" => mailings(app, ui),
        "Review" => review(app, ui),
        "View" => view(app, ui),
        "Developer" => developer(app, ui),
        "Zotero" => zotero(app, ui),
        "Help" => help(app, ui),
        "Equation" if app.session.math.is_some() => crate::equation_tab::show(app, ui),
        "Outlining" => outlining(app, ui),
        _ => home(app, ui),
    }
}

/// The ribbon (UI experiment, branch `ui-pages-layout`): one line of the tab's essentials
/// ([`line_keys`]), centred over the document as the tabs are (KrubOffice's signature), then
/// More, which opens the whole tab as the two-row ribbon draws it.
pub fn show_body(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    if app.session.view.read_mode || app.ui.ribbon_collapsed {
        return;
    }
    let mut user_tabs = app.session.ribbon.tabs().into_owned();
    user_tabs.retain(|t| !cfg!(target_arch = "wasm32") || t.name != "Zotero");
    egui::Panel::top("ribbon")
        // One line of small buttons with the ribbon's padding above and below.
        .exact_size(crate::widgets::line_button().0 + 10.0)
        .frame(
            // No outline: its top edge drew a grey line between the active tab and the ribbon. The
            // panel's own divider still separates the ribbon from the ruler below.
            egui::Frame::NONE.fill(t.ribbon).inner_margin(egui::Margin { left: 8, right: 8, top: 5, bottom: 5 }),
        )
        .show(ui, |ui| {
            // Right-click on the ribbon (outside a button): Customize Ribbon. Registered first, so
            // the buttons drawn over it keep their own clicks.
            // No document open (#1129): every command is greyed out but Help's.
            if !app.has_document() && app.ui.tab != "Help" {
                ui.disable();
            }
            let bg = ui.interact(ui.max_rect(), ui.id().with("ribbon_background"), Sense::click());
            bg.context_menu(|ui| crate::widgets::customize_menu_items(ui, app));
            let entry = user_tabs.iter().find(|e| e.name == app.ui.tab);
            let view_w = ui.available_width();
            let scroll = egui::ScrollArea::horizontal().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                let ribbon_row = egui::UiBuilder::new()
                    .layout(egui::Layout::left_to_right(egui::Align::Center).with_cross_align(egui::Align::Min))
                    .ui_stack_info(egui::UiStackInfo::default().with_tag(crate::widgets::RIBBON_CONTROL_TAG));
                ui.scope_builder(ribbon_row, |ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                    // Centred: the line's width from the last frame sets the space before it.
                    let wid = egui::Id::new("ribbon_line_width");
                    let last: Option<f32> = ui.data(|d| d.get_temp(wid));
                    if app.ui.ribbon_tabs_centered {
                        ui.add_space(((view_w - last.unwrap_or(view_w)) / 2.0).max(0.0).floor());
                    }
                    let start = ui.cursor().min.x;
                    let ctx = ui.ctx().clone();
                    crate::widgets::set_compact(&ctx, true);
                    // Every control the tab has on the line, then the order to draw them in:
                    // the user's (owner, 2026-11-24: drag them in More), else the tab's own.
                    let items = line_census(app, ui, entry);
                    let (order, priority) = line_order(app, &app.ui.tab, &items);
                    let customizable = !items.is_empty();
                    // More's footprint (with the space before it) as drawn on the last frame.
                    let more_id = egui::Id::new("ribbon_line_more_w");
                    let more_room = if customizable { ui.data(|d| d.get_temp::<f32>(more_id)).unwrap_or(110.0) + 10.0 } else { 0.0 };
                    // Measured from where the line starts, so the centring gap never changes what fits.
                    crate::widgets::set_line_limit(&ctx, Some(start + view_w - more_room));
                    // What fits, most important first: the tab's own order of importance, or the
                    // user's order (the last ones go to More first), from each button's width as
                    // drawn on earlier frames.
                    let widths_id = egui::Id::new(("ribbon_line_widths", app.ui.tab.clone()));
                    let mut widths: std::collections::HashMap<String, f32> = ui.data(|d| d.get_temp(widths_id)).unwrap_or_default();
                    let budget = view_w - more_room + 4.0;
                    let mut used = 0.0;
                    let mut included: Vec<&str> = Vec::new();
                    for key in &priority {
                        let est = items.iter().find(|i| &i.key == key).map_or(48.0, |i| i.est);
                        let w = widths.get(key).copied().unwrap_or(est);
                        if used + w <= budget {
                            used += w;
                            included.push(key);
                        }
                    }
                    // One button at a time, in order, with a divider where the group changes.
                    crate::widgets::set_no_group_dividers(&ctx, true);
                    let top = ui.cursor().min.y;
                    let h = crate::widgets::line_button().0;
                    let mut drawn: Vec<(String, Rect)> = Vec::new();
                    let mut last_group: Option<&str> = None;
                    for key in order.iter().filter(|k| included.contains(&k.as_str())) {
                        let group = items.iter().find(|i| &i.key == key).map_or("", |i| i.group.as_str());
                        let x0 = ui.cursor().min.x;
                        if last_group.is_some_and(|g| g != group) {
                            ui.add_space(2.0);
                            let x = ui.cursor().min.x;
                            ui.painter().line_segment([pos2(x, top + 5.0), pos2(x, top + h - 5.0)], egui::Stroke::new(1.0, t.border));
                            ui.add_space(3.0);
                        }
                        if let Some(r) = draw_one(app, ui, entry, key, top, h) {
                            widths.insert(key.clone(), r.max.x - x0 + 2.0);
                            drawn.push((key.clone(), r));
                            last_group = Some(group);
                        }
                    }
                    ui.data_mut(|d| d.insert_temp(widths_id, widths));
                    crate::widgets::set_no_group_dividers(&ctx, false);
                    crate::widgets::set_line_limit(&ctx, None);
                    crate::widgets::set_line(&ctx, None);
                    crate::widgets::set_compact(&ctx, false);
                    let mut tray = None;
                    if customizable {
                        let before = ui.cursor().min.x;
                        tray = Some(more_button(app, ui, entry, &items, &order));
                        let more_w = ui.cursor().min.x - before;
                        ui.data_mut(|d| d.insert_temp(more_id, more_w));
                    }
                    let line_rect = Rect::from_min_max(pos2(ui.max_rect().min.x, top - 5.0), pos2(ui.max_rect().max.x, top + h + 5.0));
                    line_drag(app, ui, line_rect, &drawn, &items, &order, tray.unwrap_or_default());
                    let w = ui.cursor().min.x - start;
                    if last.is_none_or(|l| (l - w).abs() > 0.5) {
                        ui.data_mut(|d| d.insert_temp(wid, w));
                        ui.ctx().request_discard("ribbon line width");
                    }
                });
            });
            // The bar is hidden, so a row wider than the window would be clipped with no sign. Edge chevrons show
            // where more is and scroll it on click. Every command also stays reachable through Search Commands.
            let r = scroll.inner_rect;
            let overflow = scroll.content_size.x - r.width();
            let offset = scroll.state.offset.x;
            let step = 160.0;
            let mut next = None;
            if offset > 1.0 {
                let left = Rect::from_min_max(r.left_top(), pos2(r.left() + 18.0, r.bottom()));
                if edge_cue(ui, left, "‹", tl!("Scroll ribbon left"), &t) {
                    next = Some(offset - step);
                }
            }
            if overflow - offset > 1.0 {
                let right = Rect::from_min_max(pos2(r.right() - 18.0, r.top()), r.right_bottom());
                if edge_cue(ui, right, "›", tl!("Scroll ribbon right"), &t) {
                    next = Some(offset + step);
                }
            }
            if let Some(x) = next {
                let mut state = scroll.state;
                state.offset.x = x.clamp(0.0, overflow.max(0.0));
                state.store(ui.ctx(), scroll.id);
            }
        });
}

/// The selection's formatting (`format.state`), worked out once a frame: the one-line ribbon draws
/// Home once per button.
fn format_state(app: &mut WordApp, ctx: &egui::Context) -> Value {
    let id = egui::Id::new("krub_format_state_frame");
    let key = (ctx.cumulative_pass_nr(), app.session.rev());
    if let Some((k, v)) = ctx.data(|d| d.get_temp::<((u64, u64), Value)>(id))
        && k == key
    {
        return v;
    }
    let v = app.session.run("format.state", &json!({})).unwrap_or_default();
    ctx.data_mut(|d| d.insert_temp(id, (key, v.clone())));
    v
}

/// Every control the one-line ribbon can show for the tab, in its drawing order: the tab is drawn
/// once out of sight with the census on ([`crate::widgets::set_census`]).
fn line_census(app: &mut WordApp, ui: &mut Ui, entry: Option<&wordcraft_engine::ribbon::TabEntry>) -> Vec<crate::widgets::LineItem> {
    let ctx = ui.ctx().clone();
    crate::widgets::set_line(&ctx, Some(vec!["*".into()]));
    crate::widgets::set_census(&ctx, true);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(ui.max_rect()).invisible().layout(egui::Layout::left_to_right(egui::Align::Center)));
    draw_tab(app, &mut child, entry);
    crate::widgets::set_census(&ctx, false);
    crate::widgets::set_line(&ctx, None);
    crate::widgets::census(&ctx)
}

/// The order the line draws `items` in and the order they get room in: the user's arrangement of
/// `tab` (both the same: the last go to More first), else the tab's own list (drawn in the tab's
/// order, room by its importance), else, for a tab without a list, everything.
pub(crate) fn line_order(app: &WordApp, tab: &str, items: &[crate::widgets::LineItem]) -> (Vec<String>, Vec<String>) {
    let key_of = |k: &str| items.iter().find(|i| i.is(k)).map(|i| i.key.clone());
    let dedup = |v: Vec<String>| {
        let mut out: Vec<String> = Vec::new();
        for k in v {
            if !out.contains(&k) {
                out.push(k);
            }
        }
        out
    };
    if let Some(custom) = app.ui.ribbon_line.get(tab) {
        let order = dedup(custom.iter().filter_map(|k| key_of(k)).collect());
        return (order.clone(), order);
    }
    let defaults = line_keys(tab);
    if defaults == ["*"] {
        let all: Vec<String> = items.iter().map(|i| i.key.clone()).collect();
        return (all.clone(), all);
    }
    let order = items.iter().filter(|i| defaults.iter().any(|k| i.is(k))).map(|i| i.key.clone()).collect();
    let priority = dedup(defaults.iter().filter_map(|k| key_of(k)).collect());
    (order, priority)
}

/// Draw only the control keyed `key` of the tab at the cursor, `h` high from `top`, and take its
/// width: the tab draws in a child shifted so the control lands at the cursor (the tab's own
/// spacing before it, measured on the last frame, falls left of it, out of sight). Its rect,
/// or `None` when the tab doesn't draw it.
fn draw_one(app: &mut WordApp, ui: &mut Ui, entry: Option<&wordcraft_engine::ribbon::TabEntry>, key: &str, top: f32, h: f32) -> Option<Rect> {
    let ctx = ui.ctx().clone();
    // Per place (the line, a tray tile): their spacing differs.
    let off_id = ui.id().with(("ribbon_line_offset", key.to_string()));
    let off: f32 = ui.data(|d| d.get_temp(off_id)).unwrap_or(0.0);
    let x = ui.cursor().min.x;
    let max = Rect::from_min_size(pos2(x - off, top), vec2(ui.available_width() + off, h));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(max).layout(egui::Layout::left_to_right(egui::Align::Center)));
    child.spacing_mut().item_spacing = ui.spacing().item_spacing;
    crate::widgets::take_kept_at(&ctx);
    crate::widgets::take_kept_end(&ctx);
    crate::widgets::set_line(&ctx, Some(vec![key.to_string()]));
    draw_tab(app, &mut child, entry);
    let start = crate::widgets::take_kept_at(&ctx)?;
    let new_off = start - max.min.x;
    if (new_off - off).abs() > 0.5 {
        ui.data_mut(|d| d.insert_temp(off_id, new_off));
        ctx.request_discard("ribbon line offset");
    }
    // The control's own right edge (not the tab's spacing after it).
    let end = crate::widgets::take_kept_end(&ctx).unwrap_or_else(|| child.min_rect().max.x);
    let w = (end - start).max(0.0);
    let r = Rect::from_min_max(pos2(x, top), pos2(x + w, top + h));
    ui.allocate_rect(r, Sense::hover());
    Some(r)
}

/// What More's tray showed this frame: whether it is open, and each tile with its key.
#[derive(Clone, Default)]
struct Tray {
    open: bool,
    tiles: Vec<(String, Rect)>,
}

/// The one-line ribbon's More (owner, 2026-11-24): a tray of every button the tab has, in its
/// groups. A tile works as the button does; dragging tiles onto the line, and the line's buttons
/// along it or off it, arranges the line ([`line_drag`]).
fn more_button(
    app: &mut WordApp,
    ui: &mut Ui,
    entry: Option<&wordcraft_engine::ribbon::TabEntry>,
    items: &[crate::widgets::LineItem],
    order: &[String],
) -> Tray {
    let t = Tokens::get(ui.ctx());
    ui.add_space(6.0);
    let label = tl!("More");
    let galley = ui.painter().layout_no_wrap(label.to_string(), crate::theme::regular(12.0), t.text);
    let w = galley.size().x + crate::widgets::DROPDOWN + 18.0;
    // The line's buttons' height and face, so More reads as one of them (owner, 2026-11-23). A
    // fixed id: its tray stays open while the buttons before it change.
    let (r, _) = ui.allocate_exact_size(vec2(w, crate::widgets::line_button().0), Sense::hover());
    let resp = ui.interact(r, egui::Id::new("krub_ribbon_line_more"), Sense::click());
    crate::widgets::ribbon_control_bg(ui, r, &resp, false, &t);
    let gs = galley.size();
    ui.painter().galley(pos2(r.min.x + 8.0, r.center().y - gs.y / 2.0), galley, t.text);
    crate::icons::paint(
        ui.painter(),
        Rect::from_center_size(
            pos2(r.max.x - 6.0 - crate::widgets::DROPDOWN / 2.0, r.center().y),
            vec2(crate::widgets::DROPDOWN, crate::widgets::DROPDOWN),
        ),
        "dropdown",
        t.icon,
        t.icon,
    );
    let resp = resp.on_hover_text(tl!("Every button on this tab; drag them to arrange the line"));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tl!("More")));
    // A drag from the tray keeps it open until the drop (the pointer leaves it on the way).
    let dragging = ui.ctx().data(|d| d.get_temp::<LineDrag>(egui::Id::new(DRAG_ID))).is_some();
    // So does a press on the line itself (it may start dragging one of its buttons).
    let band = r.y_range().expand(8.0);
    let on_line = ui.ctx().input(|i| i.pointer.any_down() && i.pointer.press_origin().is_some_and(|p| band.contains(p.y) && p.x < r.min.x));
    let close = if dragging || on_line { egui::PopupCloseBehavior::IgnoreClicks } else { egui::PopupCloseBehavior::CloseOnClickOutside };
    let max_w = (r.max.x - ui.ctx().content_rect().min.x - 16.0).clamp(240.0, 760.0);
    let mut tray = Tray::default();
    let tab = app.ui.tab.clone();
    let shown = egui::Popup::menu(&resp).align(egui::RectAlign::BOTTOM_END).close_behavior(close).show(|ui| {
        // A menu's style draws widgets without outlines; the tab's font and size boxes need the
        // ribbon's, so the tray draws in the app's normal style.
        *ui.style_mut() = (*ui.ctx().global_style()).clone();
        ui.set_min_width(max_w);
        ui.set_max_width(max_w);
        // What the tray is for, first (as a toolbar's customize sheet says it).
        ui.label(egui::RichText::new(tl!("Drag the buttons you use onto the line…")).size(13.0).color(t.text));
        ui.add_space(4.0);
        // As tall as the window allows under the line (it was a fixed 560 pt, so Review's Protect
        // and Ink groups sat out of sight), with a scroll bar that shows whenever it scrolls.
        let max_h = (ui.ctx().content_rect().bottom() - r.max.y - 96.0).max(240.0);
        ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
        egui::ScrollArea::vertical().max_height(max_h).auto_shrink([false, true]).show(ui, |ui| {
            let mut groups: Vec<&str> = Vec::new();
            for i in items {
                if !groups.contains(&i.group.as_str()) {
                    groups.push(&i.group);
                }
            }
            // Tile widths from the last frame, so the rows wrap before the tray's edge.
            let widths_id = egui::Id::new(("ribbon_tray_widths", tab.clone()));
            let mut widths: std::collections::HashMap<String, f32> = ui.data(|d| d.get_temp(widths_id)).unwrap_or_default();
            let row_w = ui.available_width();
            for group in groups {
                let heading = if group.is_empty() { tl!("Other") } else { tl!(group) };
                ui.label(egui::RichText::new(heading).size(12.0).strong().color(t.text_dim));
                let members: Vec<&crate::widgets::LineItem> = items.iter().filter(|i| i.group == group).collect();
                let mut rows: Vec<Vec<&crate::widgets::LineItem>> = vec![Vec::new()];
                let mut used = 0.0;
                for item in members {
                    let w = widths.get(&item.key).copied().unwrap_or(150.0) + 6.0;
                    if used + w > row_w && rows.last().is_some_and(|r| !r.is_empty()) {
                        rows.push(Vec::new());
                        used = 0.0;
                    }
                    used += w;
                    if let Some(r) = rows.last_mut() {
                        r.push(item);
                    }
                }
                for row in rows {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                        for item in row {
                            let rect = tray_tile(app, ui, entry, item, order, &tab);
                            widths.insert(item.key.clone(), rect.width());
                            tray.tiles.push((item.key.clone(), rect));
                        }
                    });
                }
                ui.add_space(6.0);
            }
            ui.data_mut(|d| d.insert_temp(widths_id, widths));
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(tl!("Drag along the line to move a button, or off it to remove it")).small().color(t.text_dim));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let custom = app.ui.ribbon_line.contains_key(&tab);
                if ui.add_enabled(custom, egui::Button::new(tl!("Reset"))).on_hover_text(tl!("Put this tab's line back as it was")).clicked() {
                    app.ui.ribbon_line.remove(&tab);
                }
            });
        });
    });
    tray.open = shown.is_some();
    tray
}

/// One tile of More's tray: the button itself, as on the line, then its name (whose right-click
/// adds it to the line or takes it off); a dot when it is on the line. Its rect.
fn tray_tile(
    app: &mut WordApp,
    ui: &mut Ui,
    entry: Option<&wordcraft_engine::ribbon::TabEntry>,
    item: &crate::widgets::LineItem,
    order: &[String],
    tab: &str,
) -> Rect {
    let t = Tokens::get(ui.ctx());
    let ctx = ui.ctx().clone();
    let on_line = order.contains(&item.key);
    let fill = if on_line { t.accent.gamma_multiply(0.08) } else { t.menu };
    let frame = egui::Frame::new().fill(fill).stroke(egui::Stroke::new(1.0, t.border)).corner_radius(6).inner_margin(egui::Margin {
        left: 2,
        right: 10,
        top: 2,
        bottom: 2,
    });
    let tile = frame.show(ui, |ui| {
        let row = egui::UiBuilder::new()
            .layout(egui::Layout::left_to_right(egui::Align::Center))
            .ui_stack_info(egui::UiStackInfo::default().with_tag(crate::widgets::RIBBON_CONTROL_TAG));
        ui.scope_builder(row, |ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
            crate::widgets::set_compact(&ctx, true);
            crate::widgets::set_no_group_dividers(&ctx, true);
            let top = ui.cursor().min.y;
            draw_one(app, ui, entry, &item.key, top, crate::widgets::line_button().0);
            crate::widgets::set_line(&ctx, None);
            crate::widgets::set_no_group_dividers(&ctx, false);
            crate::widgets::set_compact(&ctx, false);
            if !item.named {
                let name = if item.name.is_empty() { item.key.clone() } else { tl!(&item.name).replace('\n', " ") };
                ui.add(egui::Label::new(egui::RichText::new(name).size(12.5)).selectable(false).extend());
            }
            // The keyboard's way to arrange the line (and the mouse's, without dragging): + adds
            // the button at the line's end, − takes it off.
            let (icon, tip) = if on_line { ("minus", tl!("Remove from the line")) } else { ("plus", tl!("Add to the line")) };
            // Accent colours (owner, 2026-11-25): the green plus and the orange minus.
            if krub_icons::tabler_colour::with_colour(&ctx, true, || crate::widgets::icon_button(ui, icon, tip)).clicked() {
                let mut o: Vec<String> = order.iter().filter(|k| **k != item.key).cloned().collect();
                if !on_line {
                    o.push(item.key.clone());
                }
                set_line_order(app, tab, o);
            }
        });
    });
    let rect = tile.response.rect;
    if on_line {
        ui.painter().circle_filled(pos2(rect.max.x - 5.0, rect.min.y + 5.0), 3.0, t.accent);
    }
    rect
}

/// Save `order` as the user's arrangement of `tab`'s line.
fn set_line_order(app: &mut WordApp, tab: &str, order: Vec<String>) {
    app.ui.ribbon_line.insert(tab.to_string(), order);
}

const DRAG_ID: &str = "krub_ribbon_line_drag";

/// A button being dragged to arrange the line: its key, and whether it came from the line.
#[derive(Clone)]
struct LineDrag {
    key: String,
    from_line: bool,
}

/// Dragging to arrange the line (owner, 2026-11-24): a tile from More's tray onto the line adds
/// it where it is dropped; a button on the line (while the tray is open, so a click on the line
/// never moves anything) moves along it, or comes off it when dropped elsewhere. A drag starts
/// once the pointer has moved 6 pt with the button down, so clicks stay clicks.
fn line_drag(
    app: &mut WordApp,
    ui: &mut Ui,
    line_rect: Rect,
    drawn: &[(String, Rect)],
    items: &[crate::widgets::LineItem],
    order: &[String],
    tray: Tray,
) {
    let ctx = ui.ctx().clone();
    let id = egui::Id::new(DRAG_ID);
    let (down, origin, pos) = ctx.input(|i| (i.pointer.primary_down(), i.pointer.press_origin(), i.pointer.latest_pos()));
    let mut drag: Option<LineDrag> = ctx.data(|d| d.get_temp(id));
    if drag.is_none()
        && down
        && let (Some(o), Some(p)) = (origin, pos)
        && o.distance(p) > 6.0
    {
        let started = ctx.data(|d| d.get_temp::<bool>(id.with("press_seen"))).unwrap_or(false);
        if !started {
            ctx.data_mut(|d| d.insert_temp(id.with("press_seen"), true));
            if let Some((k, _)) = tray.tiles.iter().find(|(_, r)| r.contains(o)) {
                drag = Some(LineDrag { key: k.clone(), from_line: false });
            } else if tray.open
                && let Some((k, _)) = drawn.iter().find(|(_, r)| r.contains(o))
            {
                drag = Some(LineDrag { key: k.clone(), from_line: true });
            }
        }
    }
    if !down {
        ctx.data_mut(|d| d.remove::<bool>(id.with("press_seen")));
    }
    let t = Tokens::get(&ctx);
    // While the tray is open the line is being arranged: its buttons are outlined, and an open
    // hand over anything that can be dragged says so.
    if tray.open && drag.is_none() {
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, id.with("outline")));
        for (_, r) in drawn {
            painter.rect_stroke(r.expand(1.0), 5.0, egui::Stroke::new(1.0, t.accent.gamma_multiply(0.55)), egui::StrokeKind::Outside);
        }
        if pos.is_some_and(|p| tray.tiles.iter().chain(drawn).any(|(_, r)| r.contains(p))) && !down {
            ctx.set_cursor_icon(egui::CursorIcon::Grab);
        }
    }
    let Some(d) = drag else { return };
    // Where it would go: before the first button whose middle is right of the pointer.
    let at = pos.unwrap_or(line_rect.center());
    let over_line = line_rect.expand2(vec2(0.0, 10.0)).contains(at);
    let index = drawn.iter().position(|(_, r)| r.center().x > at.x).unwrap_or(drawn.len());
    if down {
        ctx.data_mut(|m| m.insert_temp(id, d.clone()));
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, id.with("ghost")));
        if over_line {
            let x = match (drawn.get(index), index.checked_sub(1).and_then(|i| drawn.get(i))) {
                (Some((_, r)), _) => r.min.x - 2.0,
                (None, Some((_, r))) => r.max.x + 2.0,
                (None, None) => line_rect.min.x + 8.0,
            };
            let gold = egui::Color32::from_rgb(0xE9, 0xB3, 0x08);
            painter.rect_filled(Rect::from_min_max(pos2(x - 1.5, line_rect.min.y + 6.0), pos2(x + 1.5, line_rect.max.y - 6.0)), 1.5, gold);
        }
        // The button's icon and name under the pointer; a − when it would come off the line.
        let item = items.iter().find(|i| i.key == d.key);
        let name = item.map_or(d.key.clone(), |i| tl!(&i.name).replace('\n', " "));
        // The boxes (font, size, style) have no icon of their own.
        let icon = item.map(|i| i.icon.clone()).filter(|i| !i.is_empty() && !matches!(i.as_str(), "font" | "size" | "para_style_box"));
        let galley = painter.layout_no_wrap(name, crate::theme::regular(12.0), t.text);
        let icon_w = if icon.is_some() { 26.0 } else { 0.0 };
        let r = Rect::from_min_size(at + vec2(10.0, 10.0), galley.size() + vec2(16.0 + icon_w, 12.0));
        painter.rect(r, 6.0, t.menu, egui::Stroke::new(1.5, t.accent), egui::StrokeKind::Inside);
        if let Some(icon) = &icon {
            let ir = Rect::from_center_size(pos2(r.min.x + 8.0 + 10.0, r.center().y), vec2(20.0, 20.0));
            crate::icons::paint(&painter, ir, icon, t.icon, t.accent);
        }
        painter.galley(pos2(r.min.x + 8.0 + icon_w, r.center().y - galley.size().y / 2.0), galley, t.text);
        if d.from_line && !over_line {
            let c = pos2(r.max.x, r.min.y);
            painter.circle_filled(c, 8.0, egui::Color32::from_rgb(0xC2, 0x41, 0x0C));
            painter.line_segment([c - vec2(4.0, 0.0), c + vec2(4.0, 0.0)], egui::Stroke::new(2.0, egui::Color32::WHITE));
        }
        ctx.request_repaint();
        return;
    }
    // Dropped.
    ctx.data_mut(|m| m.remove::<LineDrag>(id));
    let tab = app.ui.tab.clone();
    let mut new: Vec<String> = order.iter().filter(|k| **k != d.key).cloned().collect();
    if over_line {
        let pos = match drawn.get(index) {
            Some((k, _)) if *k == d.key => drawn.get(index + 1).and_then(|(n, _)| new.iter().position(|x| x == n)).unwrap_or(new.len()),
            Some((k, _)) => new.iter().position(|x| x == k).unwrap_or(new.len()),
            None => drawn.last().and_then(|(k, _)| new.iter().position(|x| x == k)).map_or(new.len(), |p| p + 1),
        };
        new.insert(pos.min(new.len()), d.key);
    } else if !d.from_line {
        return;
    }
    if new != order {
        set_line_order(app, &tab, new);
    }
}

/// Header, Footer and Page Number: on Insert and on the Header & Footer tab.
fn hf_menus(ui: &mut Ui, app: &mut WordApp) {
    menu_button(ui, app, "header", Some("Header"), "Header", true, "insert.header", header_items);
    menu_button(ui, app, "footer", Some("Footer"), "Footer", true, "insert.footer", footer_items);
    menu_button(ui, app, "pageNumber", Some("Page\nNumber"), "Page Number", true, "insert.pageNumber", page_number_items);
}

/// The Header menu's items (the Insert tab and the Header & Footer pane).
pub(crate) fn header_items(ui: &mut Ui, app: &mut WordApp) {
    mi(ui, app, "Blank", "insert.header", json!({"preset": "blank"}));
    mi(ui, app, "Blank (Three Columns)", "insert.header", json!({"preset": "blankThree"}));
    mi(ui, app, "Document Title", "insert.header", json!({"preset": "title"}));
    ui.separator();
    mi(ui, app, "Edit Header", "insert.editHeader", json!({}));
    mi(ui, app, "Remove Header", "insert.removeHeader", json!({}));
}

/// The Footer menu's items.
pub(crate) fn footer_items(ui: &mut Ui, app: &mut WordApp) {
    mi(ui, app, "Blank", "insert.footer", json!({"preset": "blank"}));
    mi(ui, app, "Blank (Three Columns)", "insert.footer", json!({"preset": "blankThree"}));
    mi(ui, app, "Page Number", "insert.footer", json!({"preset": "pageNumber"}));
    ui.separator();
    mi(ui, app, "Edit Footer", "insert.editFooter", json!({}));
    mi(ui, app, "Remove Footer", "insert.removeFooter", json!({}));
}

/// The Page Number menu's items.
pub(crate) fn page_number_items(ui: &mut Ui, app: &mut WordApp) {
    pic_item(ui, app, "Top of Page", None, "pn_top", "insert.pageNumber", json!({"position": "top", "align": "right"}));
    pic_item(ui, app, "Bottom of Page", None, "pn_bottom", "insert.pageNumber", json!({"position": "bottom", "align": "center"}));
    pic_item(ui, app, "Page X of Y", None, "pn_xofy", "insert.pageNumber", json!({"position": "bottom", "format": "x of y"}));
    pic_item(ui, app, "Current Position", None, "pn_current", "insert.pageNumber", json!({"position": "current"}));
    ui.separator();
    pic_item(ui, app, "Format Page Numbers…", None, "pn_format", "ui.dialog", json!({"name": "pageNumberFormat"}));
    pic_item(ui, app, "Page Number Position…", None, "pn_position", "ui.dialog", json!({"name": "pageNumberPosition"}));
}

/// Custom groups (Customize Ribbon, #379): up to three commands as large buttons, more as small
/// labelled buttons in columns of three. Each shows the command's own icon and label.
fn custom_groups(app: &mut WordApp, ui: &mut Ui, groups: &[wordcraft_engine::ribbon::CustomGroup]) {
    for g in groups {
        let ids: Vec<(&str, String)> =
            g.commands.iter().filter_map(|id| app.session.registry.get(id).map(|s| (s.id, tl!(s.label).to_string()))).collect();
        crate::widgets::group_titled(ui, &g.name, &g.name, app, |ui, app| {
            if ids.len() <= 3 {
                for (id, label) in &ids {
                    big(ui, app, crate::widgets::command_icon(id), &two_lines(label), id, json!({}), false);
                }
                return;
            }
            for column in ids.chunks(3) {
                stack(ui, |ui| {
                    for (id, label) in column {
                        small(ui, app, crate::widgets::command_icon(id), Some(label), label, id, json!({}), false);
                    }
                });
            }
        });
    }
}

/// A large button's label on two lines when it is long (broken at the space nearest the middle).
fn two_lines(label: &str) -> String {
    let n = label.chars().count();
    if n <= 10 {
        return label.to_string();
    }
    let mid = n / 2;
    let best = label.char_indices().filter(|(_, c)| *c == ' ').min_by_key(|(i, _)| label[..*i].chars().count().abs_diff(mid)).map(|(i, _)| i);
    match best {
        Some(i) => format!("{}\n{}", &label[..i], label[i..].trim_start()),
        None => label.to_string(),
    }
}

/// A chevron over a ribbon edge that is also a button; `true` on the frame it is clicked.
fn edge_cue(ui: &mut Ui, rect: Rect, glyph: &str, hint: &str, t: &Tokens) -> bool {
    let resp = ui.interact(rect, ui.id().with(glyph), Sense::click()).on_hover_text(hint);
    let ink = if resp.hovered() { t.text } else { t.text_dim };
    ui.painter().rect_filled(rect, 0.0, t.ribbon);
    crate::icons::paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), vec2(18.0, 18.0)),
        if glyph.contains('‹') || glyph.contains('❮') { "chevron-left" } else { "chevron-right" },
        ink,
        ink,
    );
    resp.clicked()
}

/// A column of rows or small buttons, centred in the group's height as in Word (two rows sit in
/// the middle, three fill it). The centring uses the column's height from the last frame; a frame
/// whose height changed is redrawn before it is shown.
pub(crate) fn stack(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    if crate::widgets::compact(ui) {
        // One-line ribbon (UI experiment): the column's rows go straight into the line, so every
        // button sits on its centre.
        return add(ui);
    }
    let id = ui.next_auto_id().with("stack_height");
    // Only a column standing in a group's full height is centred, not one nested in a row.
    let room = ui.available_height();
    let centre = ui.layout().is_horizontal() && room >= CONTENT_H - 0.5;
    let last: Option<f32> = ui.data(|d| d.get_temp(id));
    let pad = if centre { last.map_or(0.0, |h| ((CONTENT_H - h) / 2.0).max(0.0).floor()) } else { 0.0 };
    let inner = ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(1.0, 0.0);
        ui.add_space(pad);
        add(ui);
    });
    let h = inner.response.rect.height() - pad;
    if centre && last.is_none_or(|l| (l - h).abs() > 0.5) {
        ui.data_mut(|d| d.insert_temp(id, h));
        ui.ctx().request_discard("ribbon column height");
    }
}

/// Layout › Size: the named paper sizes, then More Paper Sizes… for a custom size in Page
/// Setup (#1433).
pub(crate) fn size_menu(ui: &mut Ui, app: &mut WordApp) {
    // The current paper size is checked.
    let x = wordcraft_engine::cmd::page::sect(&app.session);
    let current = wordcraft_geom::paper_name(x.page_w, x.page_h);
    for (n, w, h) in wordcraft_geom::PAPER_SIZES {
        // Paper names are the industry's (Letter, A4, B5 (JIS)) and stay as they are; the
        // envelopes say what they are. Sizes in the measurement unit, written the region's
        // way (`21 cm × 29,7 cm`, #838).
        let name = if n.starts_with("Envelope ") { tl!(n) } else { n };
        let (width, height) = (crate::options::length(*w), crate::options::length(*h));
        let label = crate::i18n::fmt(tl!("{paper}   {width} × {height}"), &[("paper", name), ("width", &width), ("height", &height)]);
        mi_check(ui, app, &label, current == Some(*n), "layout.size", json!({"name": n}));
    }
    ui.separator();
    mi(ui, app, crate::dialogs::PAPER_LABELS.more, "ui.dialog", json!({"name": "pageSetup"}));
}

/// A row of the Underline menu: the style drawn as a line (a contributed patch). Clicking applies it.
pub(crate) fn underline_sample(ui: &mut Ui, app: &mut WordApp, label: &str, style: &str) {
    let t = crate::theme::Tokens::get(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(170.0, 24.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 3.0, t.hover);
    }
    if let Some(look) = crate::line_looks::word_look(style) {
        krub_icons::line_style::paint(ui.painter(), rect.left() + 12.0, rect.right() - 12.0, rect.center().y, look, t.text);
    }
    let resp = resp.on_hover_text(tl!(label));
    if resp.clicked() {
        crate::widgets::run_from_ribbon(app, "format.underline", json!({"style": style}));
        ui.close();
    }
}

pub(crate) fn mi(ui: &mut Ui, app: &mut WordApp, label: &str, id: &str, params: Value) {
    mi_text(ui, app, tl!(label), id, params);
}

/// Home › Multilevel List › My Lists: the lists saved by name (Define New Multilevel List), each
/// with how it numbers; one picked numbers the selected paragraphs with it. Remove from My Lists
/// takes one out of the settings (lists already in documents stay).
pub(crate) fn saved_list_items(ui: &mut Ui, app: &mut WordApp) {
    if app.ui.saved_lists.is_empty() {
        return;
    }
    ui.separator();
    ui.label(egui::RichText::new(tl!("My Lists")).small().weak());
    let lists = app.ui.saved_lists.clone();
    for l in &lists {
        let label = format!("{}   {}", l.name, l.sample());
        // The name is the user's: shown as it is, not looked up for a translation.
        let enabled = crate::widgets::enabled(app, "list.define");
        if ui.add_enabled(enabled, egui::Button::new(label).min_size(vec2(200.0, 0.0))).clicked() {
            crate::widgets::run_from_ribbon(app, "list.define", l.params());
            ui.close();
        }
    }
    ui.menu_button(tl!("Remove from My Lists"), |ui| {
        for l in &lists {
            if ui.button(&l.name).clicked() {
                app.ui.saved_lists.retain(|x| x.name != l.name);
                ui.close();
            }
        }
    });
}

/// Home › Borders and Table Design › Borders, as in Word: the sides, the presets, the inside
/// and diagonal rules, Horizontal Line, then Draw Table, View Gridlines and the dialog. In a table
/// they set the table's borders (diagonals: the selected cells'); elsewhere the paragraph's, and
/// the table-only choices are off.
pub(crate) fn border_items(ui: &mut Ui, app: &mut WordApp) {
    let in_table = crate::widgets::enabled(app, "table.borders");
    let side = |k: &str| -> (&'static str, Value) {
        if in_table {
            ("table.borders", json!({"kind": k}))
        } else {
            // A paragraph's inside border is the one between its lines (`insideH`).
            ("para.borders", json!({"kind": if k == "insideH" { "inside" } else { k }}))
        }
    };
    let groups: [&[(&str, &str)]; 3] = [
        &[("Bottom Border", "bottom"), ("Top Border", "top"), ("Left Border", "left"), ("Right Border", "right")],
        &[("No Border", "none"), ("All Borders", "all"), ("Outside Borders", "outside"), ("Inside Borders", "inside")],
        &[
            ("Inside Horizontal Border", "insideH"),
            ("Inside Vertical Border", "insideV"),
            ("Diagonal Down Border", "diagDown"),
            ("Diagonal Up Border", "diagUp"),
        ],
    ];
    for group in groups {
        for (l, k) in group {
            let (id, params) = if matches!(*k, "insideV" | "diagDown" | "diagUp") { ("table.borders", json!({"kind": k})) } else { side(k) };
            pic_item(ui, app, l, None, &format!("tb_{k}"), id, params);
        }
        ui.separator();
    }
    pic_item(ui, app, "Horizontal Line", None, "hline", "insert.horizontalLine", json!({}));
    ui.separator();
    let gridlines = app.session.view.table_gridlines;
    pic_item(ui, app, "Draw Table", None, "drawTable", "table.draw", json!({}));
    pic_item(ui, app, "View Gridlines", Some(gridlines), "gridlines", "table.viewGridlines", json!({}));
    pic_item(ui, app, "Borders and Shading…", None, "borders", "para.borders", json!({}));
}

/// A menu item with a small picture before its name (Layout's Margins, Text Direction, Orientation,
/// Columns, Breaks and Line Numbers; Table Design's Borders; Table Layout's Select, Delete and
/// AutoFit), as in Word. `checked` (`Some`) highlights the current setting.
pub(crate) fn pic_item(ui: &mut Ui, app: &mut WordApp, label: &str, checked: Option<bool>, pic: &str, id: &str, params: Value) {
    let enabled = crate::widgets::enabled(app, id);
    let atom = ui.id().with(("layout picture", pic));
    let on = checked.unwrap_or(false);
    let resp = ui
        .add_enabled_ui(enabled, |ui| {
            egui::Button::new((egui::Atom::custom(atom, vec2(24.0, 20.0)), tl!(label))).selected(on).min_size(vec2(220.0, 28.0)).atom_ui(ui)
        })
        .inner;
    if let Some(r) = resp.rect(atom) {
        let t = Tokens::get(ui.ctx());
        let (ink, accent) = if enabled { (t.icon, t.accent) } else { (t.text_disabled, t.text_disabled) };
        layout_picture(ui.painter(), r, pic, ink, accent);
    }
    resp.response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, on, tl!(label)));
    if resp.response.clicked() {
        crate::widgets::run_from_ribbon(app, id, params);
        ui.close();
    }
}

/// The small pictures of [`pic_item`], drawn on a 24 × 20 grid: pages, text lines, columns,
/// break marks and line numbers in `ink`, what the option changes in `accent`.
pub(crate) fn layout_picture(p: &egui::Painter, r: Rect, name: &str, ink: egui::Color32, accent: egui::Color32) {
    if !icons::home_menu::paint(p, r, name, ink) && !icons::layout_menu::paint(p, r, name, ink) && !icons::table_menu::paint(p, r, name, ink) {
        krub_icons::menu::paint(p, r, name, ink, accent);
    }
}

/// A menu button with an original icon, an accessible label, and the existing shortcut text.
pub(crate) fn menu_icon_button(ui: &mut Ui, icon: &str, label: &str, shortcut: &str, enabled: bool, selected: bool) -> egui::Response {
    let atom = ui.id().with(("menu icon", icon, label));
    let result = ui
        .add_enabled_ui(enabled, |ui| {
            egui::Button::new((egui::Atom::custom(atom, vec2(20.0, 18.0)), label))
                .shortcut_text(shortcut)
                .selected(selected)
                .min_size(vec2(200.0, 0.0))
                .atom_ui(ui)
        })
        .inner;
    if let Some(rect) = result.rect(atom) {
        let t = Tokens::get(ui.ctx());
        let (ink, accent) = if enabled { (t.icon, t.accent) } else { (t.text_disabled, t.text_disabled) };
        icons::paint(ui.painter(), rect.shrink(1.0), icon, ink, accent);
    }
    result.response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, selected, label));
    result.response
}

pub(crate) fn mi_icon(ui: &mut Ui, app: &mut WordApp, label: &str, icon: &str, id: &str, params: Value) {
    let shortcut = crate::widgets::shortcut_text(app, id);
    let enabled = crate::widgets::enabled(app, id);
    if menu_icon_button(ui, icon, tl!(label), &shortcut, enabled, false).clicked() {
        crate::widgets::run_from_ribbon(app, id, params);
        ui.close();
    }
}

/// Change List Level › and Define New Bullet… at the foot of the Bullets drop-down (#2786, ported
/// from WordCraft 83f18db1). Change List Level is for a paragraph in a list of its own numbering,
/// the one `para.listLevel` changes (a style's numbering it leaves alone).
pub(crate) fn bullet_list_items(ui: &mut Ui, app: &mut WordApp) {
    ui.separator();
    let in_list = app.session.doc.para_at(&app.session.sel.focus).and_then(|p| p.props.numbering).is_some_and(|n| n.num != 0);
    ui.add_enabled_ui(in_list, |ui| {
        ui.menu_button(tl!("Change List Level"), |ui| {
            for level in 0..9u8 {
                let label = format!("{} {}", tl!("Level"), level + 1);
                mi_text(ui, app, &label, "para.listLevel", json!({"level": level}));
            }
        });
    });
    mi(ui, app, "Define New Bullet…", "ui.dialog", json!({"name": "defineBullet"}));
}

/// A menu item whose text is already in the interface language.
pub(crate) fn mi_text(ui: &mut Ui, app: &mut WordApp, text: &str, id: &str, params: Value) {
    let sc = crate::widgets::shortcut_text(app, id);
    let enabled = crate::widgets::enabled(app, id);
    let resp = ui.add_enabled(enabled, egui::Button::new(text).shortcut_text(sc).min_size(vec2(200.0, 0.0)));
    if resp.clicked() {
        crate::widgets::run_from_ribbon(app, id, params);
        ui.close();
    }
}

/// The proofing languages the Language menu offers: tag and English name (translated when drawn).
pub(crate) const PROOFING_LANGUAGES: [(&str, &str); 9] = [
    ("en-US", "English (United States)"),
    ("en-GB", "English (United Kingdom)"),
    ("pl-PL", "Polish"),
    ("pt-BR", "Portuguese (Brazil)"),
    ("it-IT", "Italian"),
    ("hr-HR", "Croatian"),
    ("tr-TR", "Turkish"),
    ("ru-RU", "Russian"),
    ("cs-CZ", "Czech"),
];

/// The English name of a proofing language tag, as the status bar shows it (no tag: WordCraft's
/// default, US English): a listed language by its name; another region of one by the language
/// and the region ("French (Canada)"); a bare language subtag by its usual variant; else the tag.
pub(crate) fn language_name(tag: Option<&str>) -> String {
    let tag = tag.unwrap_or("en-US");
    let named =
        |t: &str| PROOFING_LANGUAGES.iter().chain(crate::dialogs_layout::LANGUAGES.iter()).find(|(l, _)| l.eq_ignore_ascii_case(t)).map(|(_, n)| *n);
    // Any language the Language dialog offers (French, German…) by its name, not its tag (#860).
    if let Some(name) = named(tag) {
        return name.to_string();
    }
    let mut parts = tag.split(['-', '_']);
    let primary = parts.next().unwrap_or("");
    // The region: the first two-letter or three-digit subtag after the language (and script).
    let region =
        parts.find(|p| (p.len() == 2 && p.chars().all(|c| c.is_ascii_alphabetic())) || (p.len() == 3 && p.chars().all(|c| c.is_ascii_digit())));
    match region {
        // A region the list doesn't have: the language's name without its own region or variant.
        Some(r) => {
            let language = crate::dialogs_layout::LANGUAGES
                .iter()
                .find(|(l, _)| l.split('-').next().is_some_and(|p| p.eq_ignore_ascii_case(primary)))
                .map(|(_, n)| n.split(" (").next().unwrap_or(n));
            match language {
                Some(language) => format!("{language} ({})", region_name(r)),
                None => tag.to_string(),
            }
        }
        // A bare language subtag ("pt", "pl"): the variant WordCraft proofs.
        None => match wordcraft_engine::proof::ProofLang::from_tag(Some(tag)) {
            wordcraft_engine::proof::ProofLang::Pl => "Polish".into(),
            wordcraft_engine::proof::ProofLang::Hr => "Croatian".into(),
            wordcraft_engine::proof::ProofLang::Pt => "Portuguese (Brazil)".into(),
            wordcraft_engine::proof::ProofLang::It => "Italian".into(),
            wordcraft_engine::proof::ProofLang::Tr => "Turkish".into(),
            wordcraft_engine::proof::ProofLang::Ru => "Russian".into(),
            wordcraft_engine::proof::ProofLang::Cs => "Czech".into(),
            wordcraft_engine::proof::ProofLang::En if tag.eq_ignore_ascii_case("en") => "English (United States)".into(),
            _ => tag.to_string(),
        },
    }
}

/// The English name of a region subtag the language list has no entry for (else the subtag).
fn region_name(code: &str) -> String {
    const REGIONS: [(&str, &str); 62] = [
        ("AE", "U.A.E."),
        ("AO", "Angola"),
        ("AR", "Argentina"),
        ("AT", "Austria"),
        ("AU", "Australia"),
        ("BA", "Bosnia and Herzegovina"),
        ("BE", "Belgium"),
        ("BO", "Bolivia"),
        ("BR", "Brazil"),
        ("BY", "Belarus"),
        ("CA", "Canada"),
        ("CH", "Switzerland"),
        ("CL", "Chile"),
        ("CN", "China"),
        ("CO", "Colombia"),
        ("CR", "Costa Rica"),
        ("CV", "Cabo Verde"),
        ("CY", "Cyprus"),
        ("DE", "Germany"),
        ("DO", "Dominican Republic"),
        ("EC", "Ecuador"),
        ("EG", "Egypt"),
        ("ES", "Spain"),
        ("FR", "France"),
        ("GB", "United Kingdom"),
        ("GT", "Guatemala"),
        ("HK", "Hong Kong SAR"),
        ("IE", "Ireland"),
        ("IN", "India"),
        ("IQ", "Iraq"),
        ("IT", "Italy"),
        ("JM", "Jamaica"),
        ("JO", "Jordan"),
        ("KW", "Kuwait"),
        ("KZ", "Kazakhstan"),
        ("LB", "Lebanon"),
        ("LI", "Liechtenstein"),
        ("LU", "Luxembourg"),
        ("MA", "Morocco"),
        ("MC", "Monaco"),
        ("ME", "Montenegro"),
        ("MO", "Macao SAR"),
        ("MX", "Mexico"),
        ("MY", "Malaysia"),
        ("MZ", "Mozambique"),
        ("NL", "Netherlands"),
        ("NZ", "New Zealand"),
        ("PE", "Peru"),
        ("PH", "Philippines"),
        ("PR", "Puerto Rico"),
        ("PT", "Portugal"),
        ("PY", "Paraguay"),
        ("QA", "Qatar"),
        ("RU", "Russia"),
        ("SG", "Singapore"),
        ("SY", "Syria"),
        ("TW", "Taiwan"),
        ("UA", "Ukraine"),
        ("US", "United States"),
        ("UY", "Uruguay"),
        ("VE", "Venezuela"),
        ("ZA", "South Africa"),
    ];
    REGIONS.iter().find(|(c, _)| c.eq_ignore_ascii_case(code)).map_or_else(|| code.to_ascii_uppercase(), |(_, n)| (*n).to_string())
}

/// The language at the caret (resolved through the styles), translated, for the status bar.
pub(crate) fn caret_language(app: &WordApp) -> String {
    tl!(&language_name(caret_lang_tag(app).as_deref())).to_string()
}

/// The language tag at the caret, resolved through the styles and the document default.
fn caret_lang_tag(app: &WordApp) -> Option<String> {
    let s = &app.session;
    let style = s.doc.para_at(&s.sel.focus).and_then(|p| p.props.style.clone());
    s.doc.styles.resolve_char(style.as_deref(), &s.typing_props()).lang
}

/// The Hunspell dictionary the caret's language needs but doesn't have (`pl_PL`), if any (#1167).
pub(crate) fn caret_missing_dictionary(app: &WordApp) -> Option<&'static str> {
    wordcraft_engine::proof::missing_dictionary(caret_lang_tag(app).as_deref())
}

/// Review › Language (and the status bar's language): set the selection's proofing language, or
/// leave it unchecked. The current choice has a check mark; a language outside the short list
/// (French, German…) shows checked above it. Set Proofing Language… opens the Language dialog
/// with every language and Set As Default, and Language Preferences… opens Options › Language
/// (#860: the short list alone left no way to pick French).
pub(crate) fn language_menu(ui: &mut Ui, app: &mut WordApp) {
    let st = app.session.run("format.state", &json!({})).unwrap_or_default();
    // The selection's own language; else (nothing set on the text) the one it gets from its
    // style or the document default, as the status bar shows.
    let current = match st.get("lang") {
        Some(Value::String(t)) => t.clone(),
        Some(Value::Null) if app.session.sel.is_collapsed() => caret_lang_tag(app).unwrap_or_else(|| "en-US".into()),
        _ => String::new(),
    };
    let no_proof = st.get("noProof").and_then(Value::as_bool).unwrap_or(false);
    let listed = |tag: &str| {
        current.eq_ignore_ascii_case(tag)
            || (tag == "pl-PL" && current.eq_ignore_ascii_case("pl"))
            || (tag == "pt-BR" && current.eq_ignore_ascii_case("pt"))
            || (tag == "it-IT" && current.eq_ignore_ascii_case("it"))
            || (tag == "hr-HR" && current.eq_ignore_ascii_case("hr"))
            || (tag == "tr-TR" && current.eq_ignore_ascii_case("tr"))
            || (tag == "cs-CZ" && wordcraft_engine::proof::ProofLang::from_tag(Some(&current)) == wordcraft_engine::proof::ProofLang::Cs)
            || (tag == "en-US" && current.eq_ignore_ascii_case("en"))
    };
    let checks = language_checks(&current, listed);
    if let Some(name) = checks.extra {
        mi_check(ui, app, &name, true, "review.language", json!({"lang": current}));
    }
    for ((tag, name), on) in PROOFING_LANGUAGES.iter().zip(checks.listed) {
        mi_check(ui, app, name, on, "review.language", json!({"lang": tag}));
    }
    ui.separator();
    mi_check(ui, app, "Do Not Check Spelling or Grammar", no_proof, "review.language", json!({"noProof": !no_proof}));
    ui.separator();
    mi_check(ui, app, "Set Proofing Language…", false, "review.language", json!({}));
    mi_check(ui, app, "Language Preferences…", false, "file.options", json!({"pane": "language"}));
}

/// The Language menu's check marks for the `current` tag: which short-list entries are checked
/// (`listed` matches a tag), and the name of the current language when it is none of them,
/// shown checked above. A language named like a short-list entry checks that entry, so no name
/// shows twice.
struct LanguageChecks {
    extra: Option<String>,
    listed: [bool; PROOFING_LANGUAGES.len()],
}

fn language_checks(current: &str, listed: impl Fn(&str) -> bool) -> LanguageChecks {
    let name = (!current.is_empty()).then(|| language_name(Some(current)));
    let on = PROOFING_LANGUAGES.map(|(t, n)| listed(t) || name.as_deref() == Some(n));
    LanguageChecks { extra: name.filter(|_| !on.contains(&true)), listed: on }
}

/// A menu item with a check mark when `checked` (the current choice of several).
pub(crate) fn mi_check(ui: &mut Ui, app: &mut WordApp, label: &str, checked: bool, id: &str, params: Value) {
    // An invisible mark keeps the unchecked labels aligned with the checked one.
    let mark = egui::RichText::new("✓");
    let mark = if checked { mark } else { mark.color(egui::Color32::TRANSPARENT) };
    let enabled = crate::widgets::enabled(app, id);
    let resp = ui.add_enabled(enabled, egui::Button::new((mark, tl!(label))).selected(checked).min_size(vec2(200.0, 0.0)));
    if resp.clicked() {
        crate::widgets::run_from_ribbon(app, id, params);
        ui.close();
    }
}

/// Word's text wrapping styles in menu order: label, model value, `arrange.wrap` parameter.
pub(crate) const WRAP_STYLES: [(&str, wordcraft_doc::para::Wrap, &str); 7] = {
    use wordcraft_doc::para::Wrap;
    [
        ("In Line with Text", Wrap::Inline, "inline"),
        ("Square", Wrap::Square, "square"),
        ("Tight", Wrap::Tight, "tight"),
        ("Through", Wrap::Through, "through"),
        ("Top and Bottom", Wrap::TopAndBottom, "topAndBottom"),
        ("Behind Text", Wrap::BehindText, "behindText"),
        ("In Front of Text", Wrap::InFrontOfText, "inFrontOfText"),
    ]
};

/// The Wrap Text menu for the selected picture, shape or text box: the seven styles (the
/// current one checked), then Move with Text / Fix Position on Page for a floating one. The
/// canvas context menu shows it; the ribbon's Wrap Text button can share it.
pub(crate) fn wrap_text_items(ui: &mut Ui, app: &mut WordApp) {
    use wordcraft_engine::cmd::objects::{moves_with_text, selected};
    let state = selected(&app.session).and_then(|(_, o)| o.frame().map(|(_, _, f)| (f.wrap, moves_with_text(f))));
    let wrap = state.map(|(w, _)| w);
    for (label, w, v) in WRAP_STYLES {
        mi_check(ui, app, label, wrap == Some(w), "arrange.wrap", json!({"wrap": v}));
    }
    ui.separator();
    // An inline object is neither: it's part of the text.
    let with_text = state.filter(|(w, _)| *w != wordcraft_doc::para::Wrap::Inline).map(|(_, m)| m);
    mi_check(ui, app, "Move with Text", with_text == Some(true), "arrange.moveWithText", json!({"value": true}));
    mi_check(ui, app, "Fix Position on Page", with_text == Some(false), "arrange.moveWithText", json!({"value": false}));
}

/// A quiet divider between related paragraph controls, sized to one small-button row.
fn paragraph_separator(ui: &mut Ui) {
    if crate::widgets::line(ui).is_some() {
        return;
    }
    let (r, _) = ui.allocate_exact_size(vec2(7.0, crate::widgets::SMALL_H), Sense::hover());
    let color = Tokens::get(ui.ctx()).border_strong;
    ui.painter().line_segment([pos2(r.center().x, r.top() + 4.0), pos2(r.center().x, r.bottom() - 4.0)], Stroke::new(1.0, color));
}

fn home(app: &mut WordApp, ui: &mut Ui) {
    let st = format_state(app, ui.ctx());
    let flag = |k: &str| st.get(k).and_then(Value::as_bool).unwrap_or(false);
    group(ui, "Clipboard", app, |ui, app| {
        crate::widgets::big_split(ui, app, "paste", "Paste", "Paste", "edit.paste", json!({}), |ui, app| {
            mi_icon(ui, app, "Paste", "paste", "edit.paste", json!({}));
            mi_icon(ui, app, "Keep Text Only", "textBox", "edit.pasteText", json!({}));
            mi_icon(ui, app, "Merge Formatting", "painter", "edit.pasteMerge", json!({}));
            mi_icon(ui, app, "Paste Special…", "paste", "edit.pasteSpecial", json!({}));
            // The Clipboard pane (the group had a launcher for it; groups have none now).
            ui.separator();
            mi_icon(ui, app, "Clipboard", "paste", "edit.clipboardPane", json!({}));
        });
        // Two rows: Cut and Copy, then the Format Painter.
        stack(ui, |ui| {
            crate::widgets::row(ui, |ui| {
                small(ui, app, "cut", None, "Cut", "edit.cut", json!({}), false);
                small(ui, app, "copy", None, "Copy", "edit.copy", json!({}), false);
            });
            ui.add_space(crate::widgets::ROW_GAP);
            crate::widgets::row(ui, |ui| {
                let active = app.session.painter.is_some();
                small(ui, app, "painter", None, "Format Painter", "edit.formatPainter", json!({}), active);
            });
        });
    });
    group(ui, "Font", app, |ui, app| {
        stack(ui, |ui| {
            crate::widgets::row(ui, |ui| {
                // The font list only on the pass that draws the box (the line draws one control a pass).
                let font_box = crate::widgets::wants(ui, "font").then(|| {
                    let font = st.get("font").and_then(Value::as_str).unwrap_or("").to_string();
                    let warning = app.font_box_warning(&font);
                    (app.previews.font_shown(&font), app.previews.families(), app.previews.font_preview_fn(), warning)
                });
                let font_w = if crate::widgets::line(ui).is_some() { 104.0 } else { 150.0 };
                if let Some((font, fams, prev, warning)) = font_box
                    && let Some(f) = crate::widgets::font_combo_warned(ui, "font", font_w, &font, &fams, Some(&*prev), warning.as_deref())
                {
                    let _ = if f == crate::widgets::ADD_FONT {
                        app.run("font.add", json!({}))
                    } else if f == crate::widgets::LOAD_SYSTEM_FONTS {
                        app.run("font.loadSystem", json!({}))
                    } else {
                        let f = app.previews.font_stored(&f);
                        app.run("format.font", json!({"name": f}))
                    };
                }
                let size = st
                    .get("size")
                    .and_then(Value::as_f64)
                    .map(|s| if s.fract() == 0.0 { format!("{s:.0}") } else { format!("{s}") })
                    .unwrap_or_default();
                let sizes: Vec<String> =
                    wordcraft_engine::cmd::format::SIZES.iter().map(|s| if s.fract() == 0.0 { format!("{s:.0}") } else { s.to_string() }).collect();
                let size_w = if crate::widgets::line(ui).is_some() { 40.0 } else { 52.0 };
                if let Some(v) = combo(ui, "size", size_w, &size, &sizes, None)
                    && let Ok(x) = v.trim().parse::<f64>()
                {
                    let _ = app.run("format.size", json!({"size": x}));
                }
                ui.add_space(3.0);
                // One-line ribbon (owner, 2026-11-23): the pair in one split button, the other in
                // its small menu.
                if crate::widgets::line(ui).is_some() {
                    split(ui, app, "grow", "Increase Font Size", "format.growFont", json!({}), false, None, |ui, app| {
                        mi_icon(ui, app, "Increase Font Size", "grow", "format.growFont", json!({}));
                        mi_icon(ui, app, "Decrease Font Size", "shrink", "format.shrinkFont", json!({}));
                    });
                }
                if crate::widgets::line(ui).is_none() {
                    small(ui, app, "grow", None, "Increase Font Size", "format.growFont", json!({}), false);
                    small(ui, app, "shrink", None, "Decrease Font Size", "format.shrinkFont", json!({}), false);
                }
                menu_button(ui, app, "case", None, "Change Case", false, "format.changeCase", |ui, app| {
                    mi(ui, app, "Sentence case.", "format.changeCase", json!({"mode": "sentence"}));
                    mi(ui, app, "lowercase", "format.changeCase", json!({"mode": "lower"}));
                    mi(ui, app, "UPPERCASE", "format.changeCase", json!({"mode": "upper"}));
                    mi(ui, app, "Capitalize Each Word", "format.changeCase", json!({"mode": "title"}));
                    mi(ui, app, "tOGGLE cASE", "format.changeCase", json!({"mode": "toggle"}));
                });
                small(ui, app, "clear", None, "Clear All Formatting", "format.clear", json!({}), false);
                // No Phonetic Guide or Enclose Characters (owner, 2026-11-25): KrubOffice has no
                // East Asian (CJK) support; the commands stay for documents and agents.
            });
            ui.add_space(crate::widgets::ROW_GAP);
            crate::widgets::row(ui, |ui| {
                small(ui, app, "bold", None, "Bold", "format.bold", json!({}), flag("bold"));
                small(ui, app, "italic", None, "Italic", "format.italic", json!({}), flag("italic"));
                // Underline keeps its menu on the line again now that the ribbon runs the full
                // width (owner, 2026-11-24).
                {
                    split(ui, app, "underline", "Underline", "format.underline", json!({}), flag("underline"), None, |ui, app| {
                        // Each style is drawn as a sample line, as in Word (a contributed patch); the name is its tooltip.
                        for (l, s) in [
                            ("Single", "single"),
                            ("Double", "double"),
                            ("Thick", "thick"),
                            ("Dotted", "dotted"),
                            ("Dashed", "dash"),
                            ("Dot-dash", "dotDash"),
                            ("Dot-dot-dash", "dotDotDash"),
                            ("Wave", "wave"),
                        ] {
                            underline_sample(ui, app, l, s);
                        }
                        mi(ui, app, "Words only", "format.underline", json!({"style": "words"}));
                        ui.separator();
                        mi(ui, app, "More Underlines…", "format.fontDialog", json!({}));
                    });
                }
                small(ui, app, "strike", None, "Strikethrough", "format.strikethrough", json!({}), flag("strike"));
                if crate::widgets::line(ui).is_some() {
                    split(
                        ui,
                        app,
                        "superscript",
                        "Superscript",
                        "format.superscript",
                        json!({}),
                        flag("superscript") || flag("subscript"),
                        None,
                        |ui, app| {
                            mi_icon(ui, app, "Superscript", "superscript", "format.superscript", json!({}));
                            mi_icon(ui, app, "Subscript", "subscript", "format.subscript", json!({}));
                        },
                    );
                    small(ui, app, "charborder", None, "Character Border", "format.border", json!({}), flag("border"));
                }
                if crate::widgets::line(ui).is_none() {
                    small(ui, app, "subscript", None, "Subscript", "format.subscript", json!({}), flag("subscript"));
                    small(ui, app, "superscript", None, "Superscript", "format.superscript", json!({}), flag("superscript"));
                    small(ui, app, "charborder", None, "Character Border", "format.border", json!({}), flag("border"));
                }
                // A contributed patch: Character Spacing on the ribbon (the Font dialog's Spacing box).
                menu_button(ui, app, "char-spacing", None, "Character Spacing", false, "", |ui, app| {
                    for (label, points) in [("Very Tight", -1.5), ("Tight", -0.5), ("Normal", 0.0), ("Loose", 1.0), ("Very Loose", 3.0)] {
                        mi(ui, app, label, "format.spacing", json!({"points": points}));
                    }
                    ui.separator();
                    mi(ui, app, "More Spacing…", "format.fontDialog", json!({}));
                });
                ui.add_space(4.0);
                menu_button(ui, app, "effects", None, "Text Effects and Typography", false, "", |ui, app| {
                    let theme = app.session.doc.settings.theme_colors.clone();
                    crate::wordart_menus::style_gallery(ui, app, "format.textEffects");
                    ui.separator();
                    ui.menu_button(tl!("Text Fill"), |ui| crate::wordart_menus::fill_menu(ui, app, &theme));
                    let outline = crate::wordart_menus::current_outline(app);
                    ui.menu_button(tl!("Text Outline"), |ui| crate::wordart_menus::outline_menu(ui, app, &theme, outline));
                    crate::wordart_menus::effects_menu(ui, app, &theme, false);
                    ui.separator();
                    mi(ui, app, "Outline", "format.outline", json!({}));
                    mi(ui, app, "Shadow", "format.shadow", json!({}));
                    mi(ui, app, "Small Caps", "format.smallCaps", json!({}));
                    mi(ui, app, "All Caps", "format.allCaps", json!({}));
                    mi(ui, app, "Double Strikethrough", "format.doubleStrikethrough", json!({}));
                });
                let hl = app.canvas.last_highlight.clone();
                split(ui, app, "highlight", "Text Highlight Color", "format.highlight", json!({"color": hl}), false, None, |ui, app| {
                    let grid = wordcraft_doc::props::Highlight::ALL;
                    egui::Grid::new("hl").spacing(vec2(3.0, 3.0)).show(ui, |ui| {
                        for (i, h) in grid.iter().enumerate().skip(1) {
                            let c = h.rgb().unwrap_or(wordcraft_doc::Rgb::WHITE);
                            let (r, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                            ui.painter().rect_filled(r, 2.0, egui::Color32::from_rgb(c.0, c.1, c.2));
                            if resp.on_hover_text(h.name()).clicked() {
                                app.canvas.last_highlight = h.ooxml().to_string();
                                let _ = app.run("format.highlight", json!({"color": h.ooxml()}));
                                ui.close();
                            }
                            if i % 5 == 0 {
                                ui.end_row();
                            }
                        }
                    });
                    mi(ui, app, "No Color", "format.highlight", json!({"color": "none"}));
                });
                let fc = app.canvas.last_font_color.clone();
                let sw = wordcraft_doc::Rgb::parse(&fc).map(crate::theme::c32);
                split(ui, app, "fontcolor", "Font Color", "format.color", json!({"color": fc}), false, sw, |ui, app| {
                    mi(ui, app, "Automatic", "format.color", json!({"color": "auto"}));
                    let theme = app.session.doc.settings.theme_colors.clone();
                    crate::colors::menu(ui, app, &theme, Target::cmd("format.color").remember(Remember::FontColor));
                });
            });
        });
    });
    group(ui, "Paragraph", app, |ui, app| {
        let rp = app.session.doc.para_at(&app.session.sel.focus).map(|p| app.session.doc.styles.resolve_para(&p.props));
        // The buttons show alignment as seen on the page (Align Right is a right-to-left paragraph's start).
        let align = rp.as_ref().map(|r| r.align.visual(r.bidi)).unwrap_or_default();
        let lists = wordcraft_engine::cmd::para::list_buttons(&app.session.doc, rp.as_ref().and_then(|r| r.numbering));
        stack(ui, |ui| {
            crate::widgets::row(ui, |ui| {
                split(ui, app, "bullets", "Bullets", "para.bullets", json!({}), lists.bullets, None, |ui, app| {
                    ui.label(egui::RichText::new(tl!("Bullet Library")).small().weak());
                    crate::widgets::hrow(ui, |ui| {
                        for c in ["•", "○", "▪", "◆", "➢", "✓", "–"] {
                            // The interface fonts have no ➢, so its tile draws the arrowhead.
                            let label = if c == "➢" { " " } else { c };
                            let resp = ui.button(egui::RichText::new(label).size(16.0));
                            if c == "➢" {
                                let m = resp.rect.center();
                                let pts = [(-4.0, -5.0), (5.0, 0.0), (-4.0, 5.0), (-1.5, 0.0)].map(|(x, y)| m + vec2(x, y));
                                ui.painter().add(egui::Shape::convex_polygon(vec![pts[0], pts[1], pts[3]], Tokens::get(ui.ctx()).text, Stroke::NONE));
                                ui.painter().add(egui::Shape::convex_polygon(vec![pts[3], pts[1], pts[2]], Tokens::get(ui.ctx()).text, Stroke::NONE));
                            }
                            if resp.clicked() {
                                let _ = app.run("para.bullets", json!({"kind": c}));
                                ui.close();
                            }
                        }
                    });
                    mi(ui, app, "None", "para.bullets", json!({"off": true}));
                    bullet_list_items(ui, app);
                });
                split(ui, app, "numbering", "Numbering", "para.numbering", json!({}), lists.numbering, None, |ui, app| {
                    for (l, k) in [
                        ("1. 2. 3.", "numbered"),
                        ("1) 2) 3)", "numberedParen"),
                        ("I. II. III.", "outline"),
                        ("A. B. C.", "upperLetter"),
                        ("a) b) c)", "lowerLetter"),
                        ("a. b. c.", "lowerLetterDot"),
                        ("i. ii. iii.", "lowerRoman"),
                    ] {
                        mi(ui, app, l, "para.numbering", json!({"kind": k}));
                    }
                    ui.separator();
                    // Word's three Thai number formats.
                    let thai = [("๑. ๒. ๓.", "thaiNumbers"), ("ก. ข. ค.", "thaiLetters"), ("หนึ่ง. สอง. สาม.", "thaiCounting")];
                    for (l, k) in thai {
                        mi(ui, app, l, "para.numbering", json!({"kind": k}));
                    }
                    ui.separator();
                    mi(ui, app, "Restart at 1", "para.restartNumbering", json!({}));
                    mi(ui, app, "None", "para.numbering", json!({"off": true}));
                });
                split(ui, app, "multilevel", "Multilevel List", "para.multilevel", json!({}), lists.multilevel, None, |ui, app| {
                    mi(ui, app, "1. 1.1. 1.1.1.", "para.multilevel", json!({"kind": "legal"}));
                    mi(ui, app, "I. A. 1. a.", "para.multilevel", json!({"kind": "outline"}));
                    mi(ui, app, "๑. ๑.๑. ๑.๑.๑.", "para.multilevel", json!({"kind": "thaiLegal"}));
                    mi(ui, app, "๑. ก. (๑) (ก)", "para.multilevel", json!({"kind": "thaiOutline"}));
                    saved_list_items(ui, app);
                    ui.separator();
                    mi(ui, app, "Define New Multilevel List…", "list.define", json!({}));
                    ui.separator();
                    for lv in 0..5u64 {
                        mi(ui, app, &format!("Change to Level {}", lv + 1), "para.listLevel", json!({"level": lv}));
                    }
                });
                paragraph_separator(ui);
                small(ui, app, "outdent", None, "Decrease Indent", "para.outdent", json!({}), false);
                small(ui, app, "indent", None, "Increase Indent", "para.indent", json!({}), false);
                paragraph_separator(ui);
                small(ui, app, "sort", None, "Sort", "para.sort", json!({}), false);
                let marks = app.session.view.marks;
                small(ui, app, "pilcrow", None, "Show/Hide ¶", "view.marks", json!({}), marks);
            });
            ui.add_space(crate::widgets::ROW_GAP);
            crate::widgets::row(ui, |ui| {
                use wordcraft_doc::Align as A;
                small(ui, app, "alignLeft", None, "Align Left", "para.alignLeft", json!({}), align == A::Left);
                small(ui, app, "alignCenter", None, "Center", "para.alignCenter", json!({}), align == A::Center);
                small(ui, app, "alignRight", None, "Align Right", "para.alignRight", json!({}), align == A::Right);
                small(ui, app, "justify", None, "Justify", "para.justify", json!({}), align == A::Justify);
                small(ui, app, "thaiDistribute", None, "Thai Distribution", "para.thaiDistribute", json!({}), align == A::ThaiDistribute);
                small(ui, app, "thaiDigits", None, "Thai Digits", "format.thaiDigits", json!({}), false);
                paragraph_separator(ui);
                menu_button(ui, app, "lineSpacing", None, "Line and Paragraph Spacing", false, "para.lineSpacing", |ui, app| {
                    for v in [1.0, 1.15, 1.5, 2.0, 2.5, 3.0] {
                        mi(ui, app, &format!("{v}"), "para.lineSpacing", json!({"value": v}));
                    }
                    ui.separator();
                    mi(ui, app, "Add Space Before Paragraph", "para.addSpaceBefore", json!({}));
                    mi(ui, app, "Remove Space After Paragraph", "para.removeSpaceAfter", json!({}));
                    mi(ui, app, "Line Spacing Options…", "para.dialog", json!({}));
                });
                paragraph_separator(ui);
                let swatch = crate::colors::parse(&app.canvas.last_shading).map(|c| egui::Color32::from_rgb(c.0, c.1, c.2));
                split(ui, app, "shading", "Shading", "para.shading", json!({"color": app.canvas.last_shading.clone()}), false, swatch, |ui, app| {
                    mi(ui, app, "No Color", "para.shading", json!({"color": null}));
                    let theme = app.session.doc.settings.theme_colors.clone();
                    crate::colors::menu(ui, app, &theme, Target::cmd("para.shading").remember(Remember::Shading));
                });
                // In a table the button sets the selected cells' bottom border, like its menu.
                let cmd = if crate::widgets::enabled(app, "table.borders") { "table.borders" } else { "para.borders" };
                split(ui, app, "borders", "Borders", cmd, json!({"kind": "bottom"}), false, None, border_items);
            });
        });
    });
    group(ui, "Styles", app, |ui, app| {
        if crate::widgets::line(ui).is_some() {
            style_box(app, ui, &st);
        } else {
            crate::previews::style_gallery(app, ui, &st);
        }
    });
    group(ui, "Editing", app, |ui, app| {
        // Two rows: Find, then Replace and Select.
        stack(ui, |ui| {
            crate::widgets::row(ui, |ui| {
                menu_button(ui, app, "find", None, "Find", false, "edit.find", |ui, app| {
                    // As in Word: Find searches in the Navigation pane; Advanced Find is the dialog (#712).
                    mi_icon(ui, app, "Find", "find", "edit.find", json!({}));
                    mi_icon(ui, app, "Advanced Find…", "replace", "edit.advancedFind", json!({}));
                    mi_icon(ui, app, "Go To…", "bookmark", "ui.dialog", json!({"name": "goto"}));
                });
            });
            ui.add_space(crate::widgets::ROW_GAP);
            crate::widgets::row(ui, |ui| {
                small(ui, app, "replace", None, "Replace", "ui.dialog", json!({"name": "replace"}), false);
                menu_button(ui, app, "select", None, "Select", false, "select.all", |ui, app| {
                    mi_icon(ui, app, "Select All", "select", "select.all", json!({}));
                    mi_icon(ui, app, "Select Paragraph", "pilcrow", "select.paragraph", json!({}));
                    mi_icon(ui, app, "Select Sentence", "textBox", "select.sentence", json!({}));
                });
            });
        });
    });
    group(ui, "Editor", app, |ui, app| {
        big(ui, app, "editor", "Editor", "review.spelling", json!({}), false);
    });
}

/// The one-line ribbon's style box (UI experiment, first drawn on `ui-rail-panel`): Normal, the
/// headings and the quick styles, then Styles Pane.
pub(crate) fn style_box(app: &mut WordApp, ui: &mut Ui, st: &Value) {
    let current = st.get("style").and_then(Value::as_str).unwrap_or("Normal").to_string();
    let sheet = &app.session.doc.styles;
    let mut ids: Vec<String> = sheet
        .styles
        .iter()
        .filter(|s| {
            s.kind == wordcraft_doc::StyleKind::Paragraph && (s.quick || crate::previews::GALLERY_STYLES.contains(&s.id.as_str())) && !s.hidden
        })
        .map(|s| s.id.clone())
        .collect();
    if !ids.contains(&current) {
        ids.insert(0, current.clone());
    }
    let names: Vec<String> = ids.iter().map(|id| sheet.get(id).map(crate::i18n::style_name).unwrap_or_else(|| id.clone())).collect();
    let shown = sheet.get(&current).map(crate::i18n::style_name).unwrap_or_else(|| current.clone());
    let mut items = names.clone();
    items.push(tl!("Styles Pane").to_string());
    if let Some(pick) = combo(ui, "para_style_box", 120.0, &shown, &items, None) {
        if pick == tl!("Styles Pane") {
            let _ = app.run("view.stylesPane", json!({"value": true}));
        } else if let Some(i) = names.iter().position(|n| *n == pick)
            && let Some(id) = ids.get(i)
        {
            let _ = app.run("para.style", json!({"style": id}));
        }
    }
}

fn insert(app: &mut WordApp, ui: &mut Ui) {
    // Two-size ribbon: each group's main command large, the rest small in two rows.
    group(ui, "Pages", app, |ui, app| {
        menu_button(ui, app, "coverPage", Some("Cover Page"), "Cover Page", true, "insert.coverPage", |ui, app| {
            mi(ui, app, "Studio cover", "insert.coverPage", json!({}));
        });
        stack(ui, |ui| {
            crate::widgets::row(ui, |ui| {
                small(ui, app, "blankPage", None, "Blank Page", "insert.blankPage", json!({}), false);
            });
            ui.add_space(crate::widgets::ROW_GAP);
            crate::widgets::row(ui, |ui| {
                small(ui, app, "pageBreak", None, "Page Break", "insert.pageBreak", json!({}), false);
            });
        });
    });
    group(ui, "Tables", app, |ui, app| {
        menu_button(ui, app, "table", Some("Table"), "Add a Table", true, "insert.table", |ui, app| {
            crate::dialogs::table_grid_picker(ui, app);
            ui.separator();
            pic_item(ui, app, "Insert Table…", None, "table", "ui.dialog", json!({"name": "insertTable"}));
            let drawing = app.canvas.table_tool == Some(crate::table_pen::TableTool::Draw);
            pic_item(ui, app, "Draw Table", Some(drawing), "drawTable", "table.draw", json!({}));
            pic_item(ui, app, "Convert Text to Table…", None, "textToTable", "table.fromText", json!({}));
            ui.menu_button(tl!("Quick Tables"), |ui| {
                mi(ui, app, "Tabular List", "table.quick", json!({"kind": "tabular"}));
                mi(ui, app, "Matrix", "table.quick", json!({"kind": "matrix"}));
                mi(ui, app, "Calendar", "table.quick", json!({"kind": "calendar"}));
            });
        });
    });
    group(ui, "Illustrations", app, |ui, app| {
        big(ui, app, "picture", "Pictures", "insert.picture", json!({}), false);
        menu_button(ui, app, "shapes", Some("Shapes"), "Shapes", true, "insert.shape", |ui, app| {
            // The shared Insert › Shapes menu (owner, 2026-11-19; `krub_icons::shape_gallery`):
            // the standard shapes by group, arrowed connectors at the end of Lines (glue their
            // ends to shapes by dragging them: shape.connect), then the Drawing Canvas.
            let tools = [
                ("lineArrow", tl!("Straight Arrow Connector")),
                ("elbowArrow", tl!("Elbow Arrow Connector")),
                ("curvedArrow", tl!("Curved Arrow Connector")),
            ];
            let extras = [("canvas", tl!("New Drawing Canvas"))];
            let opts = krub_icons::shape_gallery::Options { action_buttons: false, line_tools: &tools, extras: &extras };
            let picked = egui::ScrollArea::vertical()
                .max_height(440.0)
                .show(ui, |ui| krub_icons::shape_gallery::show(ui, &|s| crate::i18n::tc("shapes", s).to_string(), &opts))
                .inner;
            if let Some(name) = picked {
                let (id, params) = match name.as_str() {
                    "canvas" => ("insert.canvas", json!({})),
                    "lineArrow" => ("insert.shape", json!({"kind": "straightConnector", "arrow": "end"})),
                    "elbowArrow" => ("insert.shape", json!({"kind": "elbowConnector", "arrow": "end"})),
                    "curvedArrow" => ("insert.shape", json!({"kind": "curvedConnector", "arrow": "end"})),
                    other => ("insert.shape", json!({"kind": other})),
                };
                crate::widgets::run_from_ribbon(app, id, params);
                ui.close();
            }
        });
        big(ui, app, "chart", "Chart", "insert.chart", json!({}), false);
        stack(ui, |ui| {
            crate::widgets::row(ui, |ui| {
                small(ui, app, "icons", None, "Icons", "insert.icon", json!({}), false);
            });
            ui.add_space(crate::widgets::ROW_GAP);
            crate::widgets::row(ui, |ui| {
                small(ui, app, "smartArt", None, "Diagram", "insert.smartArt", json!({}), false);
            });
        });
    });
    group(ui, "Links", app, |ui, app| {
        big(ui, app, "link", "Link", "ui.dialog", json!({"name": "link"}), false);
        stack(ui, |ui| {
            crate::widgets::row(ui, |ui| {
                small(ui, app, "bookmark", None, "Bookmark", "ui.dialog", json!({"name": "bookmark"}), false);
            });
            ui.add_space(crate::widgets::ROW_GAP);
            crate::widgets::row(ui, |ui| {
                small(ui, app, "crossRef", None, "Cross-reference", "insert.crossReference", json!({}), false);
            });
        });
    });
    group(ui, "Comments", app, |ui, app| {
        big(ui, app, "newComment", "Comment", "review.newComment", json!({}), false);
    });
    group(ui, "Header & Footer", app, hf_menus);
    group(ui, "Text", app, |ui, app| {
        big(ui, app, "textBox", "Text\nBox", "insert.textBox", json!({}), false);
        stack(ui, |ui| {
            crate::widgets::row(ui, |ui| {
                menu_button(ui, app, "quickParts", Some("Quick Parts"), "Quick Parts", false, "insert.quickParts", |ui, app| {
                    let parts: Vec<String> = app.session.building_blocks.keys().cloned().collect();
                    for name in parts {
                        if ui.button(&name).clicked() {
                            let _ = app.run("insert.quickParts", json!({"insert": name}));
                            ui.close();
                        }
                    }
                    mi(ui, app, "Field…", "ui.dialog", json!({"name": "field"}));
                });
                menu_button(ui, app, "wordArt", Some("WordArt"), "WordArt", false, "", |ui, app| {
                    crate::wordart_menus::style_gallery(ui, app, "insert.wordArt")
                });
                small(ui, app, "dropCap", Some("Drop Cap"), "Drop Cap", "insert.dropCap", json!({}), false);
            });
            ui.add_space(crate::widgets::ROW_GAP);
            crate::widgets::row(ui, |ui| {
                small(ui, app, "signature", None, "Signature Line", "insert.signatureLine", json!({}), false);
                small(ui, app, "dateTime", None, "Date & Time", "insert.dateTime", json!({}), false);
                small(ui, app, "object", None, "Object", "insert.object", json!({}), false);
            });
        });
    });
    group(ui, "Symbols", app, |ui, app| {
        crate::equation_tab::insert_button(ui, app);
        menu_button(ui, app, "symbol", Some("Symbol"), "Symbol", true, "insert.symbol", |ui, app| {
            // Recently used symbols first (their font on hover), then common ones.
            let mut items: Vec<(String, String)> = app.ui.recent_symbols.iter().map(|r| (r.ch.clone(), r.font.clone())).collect();
            for c in [
                "©", "®", "™", "§", "¶", "€", "£", "¥", "°", "±", "≠", "≤", "≥", "÷", "×", "∞", "µ", "α", "β", "π", "Ω", "∑", "√", "→", "←", "✓",
                "★", "♥", "—", "…",
            ] {
                if !items.iter().any(|(ch, f)| ch == c && f.is_empty()) {
                    items.push((c.to_string(), String::new()));
                }
            }
            items.truncate(30);
            egui::Grid::new("syms").show(ui, |ui| {
                for (i, (c, font)) in items.iter().enumerate() {
                    let b = ui.button(egui::RichText::new(c).size(16.0));
                    let b = if font.is_empty() { b } else { b.on_hover_text(font) };
                    if b.clicked() {
                        let params = if font.is_empty() { json!({"char": c}) } else { json!({"char": c, "font": font}) };
                        if app.run("insert.symbol", params).is_ok() {
                            crate::dialogs_insert::remember_symbol(app, c, font);
                        }
                        ui.close();
                    }
                    if i % 6 == 5 {
                        ui.end_row();
                    }
                }
            });
            ui.separator();
            mi(ui, app, "More Symbols…", "ui.dialog", json!({"name": "symbol"}));
        });
    });
}

fn draw(app: &mut WordApp, ui: &mut Ui) {
    use wordcraft_doc::freeform::InkTool;
    use wordcraft_engine::cmd::draw::{DrawMode, lasso_selection};
    let mode = app.session.view.draw.mode;
    // The pen the colour and thickness menus change: the one in use, else the pen.
    let tool = mode.pen().unwrap_or(InkTool::Pen);
    let set = app.session.view.draw.settings(tool);
    // The gallery pen in use, if any: changing the colour or thickness changes it too.
    let custom =
        app.session.view.draw.pens.iter().position(|p| mode == DrawMode::Pen(p.kind) && p.color == set.color && (p.width - set.width).abs() < 0.01);
    let lasso_on = lasso_selection(&app.session).is_some();
    group(ui, "Drawing Tools", app, |ui, app| {
        big_toggle(ui, app, "select", "Select", "draw.select", mode == DrawMode::Select);
        big_toggle(ui, app, "lasso", "Lasso", "draw.lasso", mode == DrawMode::Lasso);
        big_toggle(ui, app, "eraser", "Eraser", "draw.eraser", mode == DrawMode::Eraser);
        big_toggle(ui, app, "pen", "Pen", "draw.pen", mode == DrawMode::Pen(InkTool::Pen) && custom.is_none());
        big_toggle(ui, app, "pencil", "Pencil", "draw.pencil", mode == DrawMode::Pen(InkTool::Pencil) && custom.is_none());
        big_toggle(ui, app, "highlight", "Highlighter", "draw.highlighter", mode == DrawMode::Pen(InkTool::Highlighter) && custom.is_none());
        // Pens added with Add Pen; right-click one to remove it.
        let pens = app.session.view.draw.pens.clone();
        for (i, pen) in pens.iter().enumerate() {
            if crate::widgets::line(ui).is_none() {
                gallery_pen(ui, app, i, *pen, custom == Some(i));
            }
        }
        menu_button(ui, app, "addPen", Some("Add\nPen"), "Add Pen", true, "", |ui, app| {
            for (label, kind) in [("Pen", InkTool::Pen), ("Pencil", InkTool::Pencil), ("Highlighter", InkTool::Highlighter)] {
                // A new pen starts as that kind's current colour and thickness.
                let cur = app.session.view.draw.settings(kind);
                mi_icon(
                    ui,
                    app,
                    label,
                    match kind {
                        InkTool::Pen => "pen",
                        InkTool::Pencil => "pencil",
                        InkTool::Highlighter => "highlight",
                    },
                    "draw.addPen",
                    json!({"kind": kind.name(), "color": cur.color.hex(), "width": cur.width}),
                );
            }
        });
        stack(ui, |ui| {
            let sw = Some(crate::theme::c32(set.color));
            split(ui, app, "inkColor", "Color", pen_command(tool), json!({}), false, sw, |ui, app| {
                let theme = app.session.doc.settings.theme_colors.clone();
                // With ink lasso-selected the colour goes to the selection, else to the pen.
                let cmd = if lasso_on { "draw.lassoColor" } else { pen_command(tool) };
                crate::colors::menu(ui, app, &theme, Target::cmd(cmd));
                if !lasso_on {
                    update_gallery_pen(app, custom, tool);
                }
            });
            menu_button(ui, app, "thickness", None, "Thickness", false, "draw.pen", |ui, app| {
                let widths: &[f32] = if tool == InkTool::Highlighter { &[4.0, 8.0, 12.0, 18.0, 24.0] } else { &[0.5, 1.0, 1.5, 2.5, 3.5, 5.0] };
                for w in widths {
                    let label = crate::i18n::fmt(tl!("{n} pt"), &[("n", &w.to_string())]);
                    mi_check(ui, app, &label, (set.width - w).abs() < 0.01, pen_command(tool), json!({"width": w}));
                }
                update_gallery_pen(app, custom, tool);
            });
        });
    });
    group(ui, "Convert", app, |ui, app| {
        let on = app.session.view.draw.ink_to_shape;
        big_toggle(ui, app, "inkToShape", "Ink to\nShape", "draw.inkToShape", on);
        big(ui, app, "inkToMath", "Ink to\nMath", "draw.inkToMath", json!({}), false);
    });
    group(ui, "Insert", app, |ui, app| {
        big(ui, app, "canvas", "Drawing\nCanvas", "insert.canvas", json!({}), false);
    });
    group(ui, "Replay", app, |ui, app| {
        let on = app.session.view.draw.replay;
        big_toggle(ui, app, "replay", "Ink\nReplay", "draw.replay", on);
    });
}

/// The command that picks up `tool`.
fn pen_command(tool: wordcraft_doc::freeform::InkTool) -> &'static str {
    use wordcraft_doc::freeform::InkTool;
    match tool {
        InkTool::Pen => "draw.pen",
        InkTool::Pencil => "draw.pencil",
        InkTool::Highlighter => "draw.highlighter",
    }
}

/// After the colour or thickness menu changed `tool`'s settings, the gallery pen that was in use
/// (`custom`) takes them too.
fn update_gallery_pen(app: &mut WordApp, custom: Option<usize>, tool: wordcraft_doc::freeform::InkTool) {
    let set = app.session.view.draw.settings(tool);
    if let Some(p) = custom.and_then(|i| app.session.view.draw.pens.get_mut(i))
        && p.kind == tool
    {
        p.color = set.color;
        p.width = set.width;
    }
}

/// A pen of the gallery: its kind's icon over a bar of its colour and thickness; a click picks it
/// up, right-click offers Remove Pen.
fn gallery_pen(ui: &mut Ui, app: &mut WordApp, i: usize, pen: wordcraft_engine::cmd::draw::CustomPen, active: bool) {
    use wordcraft_doc::freeform::InkTool;
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(34.0, CONTENT_H), Sense::click());
    crate::widgets::bg(ui, r, &resp, active, &t);
    let icon = match pen.kind {
        InkTool::Pen => "pen",
        InkTool::Pencil => "pencil",
        InkTool::Highlighter => "highlight",
    };
    let color = crate::theme::c32(pen.color);
    icons::paint(ui.painter(), Rect::from_center_size(pos2(r.center().x, r.min.y + 20.0), vec2(28.0, 28.0)), icon, t.icon, color);
    let bar = (pen.width * 0.6).clamp(1.5, 8.0);
    let y = r.min.y + 46.0;
    ui.painter().line_segment([pos2(r.min.x + 6.0, y), pos2(r.max.x - 6.0, y)], Stroke::new(bar, color));
    let kind = tl!(match pen.kind {
        InkTool::Pen => "Pen",
        InkTool::Pencil => "Pencil",
        InkTool::Highlighter => "Highlighter",
    });
    let size = crate::i18n::fmt(tl!("{n} pt"), &[("n", &pen.width.to_string())]);
    let resp = resp.on_hover_text(format!("{kind}: {size}, #{}", pen.color.hex()));
    if resp.clicked() {
        let _ = app.run(pen_command(pen.kind), json!({"color": pen.color.hex(), "width": pen.width}));
    }
    resp.context_menu(|ui| {
        if ui.button(tl!("Remove Pen")).clicked() {
            let _ = app.run("draw.removePen", json!({"index": i}));
            ui.close();
        }
    });
}

/// The Design tab's style set gallery in a box of its own, sized from its thumbnails and clipped
/// to it, with a gap on each side: the Colors and Fonts column can't be drawn over, or cut by,
/// the thumbnails (#638). Returns the box and the space the thumbnails used.
pub(crate) fn style_set_box(app: &mut WordApp, ui: &mut Ui) -> (Rect, Rect) {
    ui.add_space(4.0);
    let (gallery, _) = ui.allocate_exact_size(vec2(crate::previews::STYLE_SET_GALLERY_W, CONTENT_H), Sense::hover());
    let mut gui = ui.new_child(egui::UiBuilder::new().max_rect(gallery).layout(egui::Layout::left_to_right(egui::Align::Min)));
    gui.set_clip_rect(gallery.intersect(ui.clip_rect()));
    gui.spacing_mut().item_spacing = vec2(1.0, 1.0);
    crate::previews::style_set_gallery(app, &mut gui);
    ui.add_space(8.0);
    (gallery, gui.min_rect())
}

pub(crate) fn design(app: &mut WordApp, ui: &mut Ui) {
    group(ui, "Themes", app, |ui, app| {
        menu_button(ui, app, "themes", Some("Themes"), "Themes", true, "design.themes", |ui, app| {
            for (name, ..) in wordcraft_engine::cmd::design::THEMES {
                mi(ui, app, name, "design.theme", json!({"name": name}));
            }
        });
    });
    group(ui, "Document Formatting", app, |ui, app| {
        if crate::widgets::line(ui).is_none() {
            style_set_box(app, ui);
        }
        stack(ui, |ui| {
            menu_button(ui, app, "colors", Some("Colors"), "Theme Colors", false, "design.themeColors", |ui, app| {
                for (name, ..) in wordcraft_engine::cmd::design::THEMES {
                    mi(ui, app, name, "design.themeColors", json!({"name": name}));
                }
            });
            menu_button(ui, app, "fonts", Some("Fonts"), "Theme Fonts", false, "design.themeFonts", |ui, app| {
                for (name, h, b, _) in wordcraft_engine::cmd::design::THEMES {
                    mi_text(ui, app, &format!("{}: {h} / {b}", tl!(name)), "design.themeFonts", json!({"heading": h, "body": b}));
                }
            });
        });
        stack(ui, |ui| {
            crate::widgets::row(ui, |ui| {
                menu_button(ui, app, "paraSpacing", Some("Paragraph Spacing"), "Paragraph Spacing", false, "design.paragraphSpacing", |ui, app| {
                    for (l, v) in [
                        ("Default", "default"),
                        ("No Paragraph Space", "none"),
                        ("Compact", "compact"),
                        ("Tight", "tight"),
                        ("Open", "open"),
                        ("Relaxed", "relaxed"),
                        ("Double", "double"),
                    ] {
                        mi_icon(ui, app, l, &format!("paraSpacing_{v}"), "design.paragraphSpacing", json!({"value": v}));
                    }
                });
                small(ui, app, "effectsDesign", Some("Effects"), "Effects", "design.effects", json!({}), false);
            });
            ui.add_space(crate::widgets::ROW_GAP);
            crate::widgets::row(ui, |ui| {
                small(ui, app, "setDefault", Some("Set as Default"), "Set as Default", "design.setDefault", json!({}), false);
            });
        });
    });
    group(ui, "Page Background", app, |ui, app| {
        menu_button(ui, app, "watermark", Some("Watermark"), "Watermark", true, "design.watermark", |ui, app| {
            for w in ["DRAFT", "CONFIDENTIAL", "DO NOT COPY", "SAMPLE", "ASAP", "URGENT"] {
                mi(ui, app, w, "design.watermark", json!({"text": w}));
            }
            ui.separator();
            mi(ui, app, "Custom Watermark…", "ui.dialog", json!({"name": "watermark"}));
            mi(ui, app, "Remove Watermark", "design.watermark", json!({"remove": true}));
        });
        menu_button(ui, app, "pageColor", Some("Page\nColor"), "Page Color", true, "design.pageColor", |ui, app| {
            mi(ui, app, "No Color", "design.pageColor", json!({"color": null}));
            let theme = app.session.doc.settings.theme_colors.clone();
            crate::colors::menu(ui, app, &theme, Target::cmd("design.pageColor"));
        });
        big(ui, app, "pageBorders", "Page\nBorders", "design.pageBorders", json!({}), false);
    });
}

// Layout › Margins, Orientation, Size and Columns: the caret section's setting has a check
// mark, as Text Direction's does (#1206).
pub(crate) fn margins_items(ui: &mut Ui, app: &mut WordApp) {
    let current = wordcraft_engine::cmd::page::margins_preset_at(&app.session);
    // The two uniform presets name their margin in the measurement unit (`2,54 cm`, #838).
    for (l, k, uniform) in [
        ("Normal  {size} all", "normal", Some(72.0)),
        ("Narrow  {size} all", "narrow", Some(36.0)),
        ("Moderate", "moderate", None),
        ("Wide", "wide", None),
        ("Mirrored", "mirrored", None),
        ("Office 2003 Default", "office2003", None),
    ] {
        let label = match uniform {
            Some(pt) => crate::i18n::fmt(tl!(l), &[("size", &crate::options::length(pt))]),
            None => l.to_string(),
        };
        // Mirrored margins come from the document as well as from the preset (#1181).
        let on = current == Some(k) || (k == "mirrored" && app.session.doc.settings.mirror_margins);
        pic_item(ui, app, &label, Some(on), &format!("mg_{k}"), "layout.margins", json!({"preset": k}));
    }
    pic_item(ui, app, "Custom Margins…", None, "mg_custom", "ui.dialog", json!({"name": "pageSetup"}));
}

pub(crate) fn orientation_items(ui: &mut Ui, app: &mut WordApp) {
    let land = wordcraft_engine::cmd::page::sect(&app.session).landscape;
    pic_item(ui, app, "Portrait", Some(!land), "portrait", "layout.orientation", json!({"value": "portrait"}));
    pic_item(ui, app, "Landscape", Some(land), "landscape", "layout.orientation", json!({"value": "landscape"}));
}

pub(crate) fn columns_items(ui: &mut Ui, app: &mut WordApp) {
    let current = wordcraft_engine::cmd::page::columns_preset_at(&app.session);
    for (l, k, params) in [
        ("One", "one", json!({"count": 1})),
        ("Two", "two", json!({"count": 2})),
        ("Three", "three", json!({"count": 3})),
        ("Left", "left", json!({"preset": "left"})),
        ("Right", "right", json!({"preset": "right"})),
    ] {
        pic_item(ui, app, l, Some(current == Some(k)), &format!("cols_{k}"), "layout.columns", params);
    }
    ui.separator();
    pic_item(ui, app, "More Columns…", None, "colsMore", "ui.dialog", json!({"name": "columns"}));
}

fn layout(app: &mut WordApp, ui: &mut Ui) {
    group(ui, "Page Setup", app, |ui, app| {
        // Text Direction works on what the caret is in: table cells, a text box, else the section.
        let dir = wordcraft_engine::cmd::page::text_direction_at(&app.session);
        menu_button(ui, app, "textDirection", Some("Text\nDirection"), "Text Direction", true, "layout.textDirection", |ui, app| {
            use wordcraft_doc::props::TextDirection;
            for (label, d) in
                [("Horizontal", TextDirection::Horizontal), ("Rotate all text 90°", TextDirection::Down), ("Rotate all text 270°", TextDirection::Up)]
            {
                let pic = match d {
                    TextDirection::Down => "dirDown",
                    TextDirection::Up => "dirUp",
                    _ => "dirHorizontal",
                };
                pic_item(ui, app, label, Some(dir == d), pic, "layout.textDirection", json!({"value": d.ooxml()}));
            }
        });
        menu_button(ui, app, "margins", Some("Margins"), "Margins", true, "layout.margins", margins_items);
        menu_button(ui, app, "orientation", Some("Orientation"), "Orientation", true, "layout.orientation", orientation_items);
        menu_button(ui, app, "size", Some("Size"), "Size", true, "layout.size", size_menu);
        menu_button(ui, app, "columns", Some("Columns"), "Columns", true, "layout.columns", columns_items);
        // Large, as in OnlyOffice: each opens a set of choices.
        menu_button(ui, app, "breaks", Some("Breaks"), "Breaks", true, "layout.break", |ui, app| {
            ui.label(egui::RichText::new(tl!("Page Breaks")).strong());
            for (l, k) in [("Page", "page"), ("Column", "column"), ("Text Wrapping", "textWrapping")] {
                pic_item(ui, app, l, None, &format!("brk_{k}"), "layout.break", json!({"kind": k}));
            }
            ui.label(egui::RichText::new(tl!("Section Breaks")).strong());
            for (l, k) in [("Next Page", "nextPage"), ("Continuous", "continuous"), ("Even Page", "evenPage"), ("Odd Page", "oddPage")] {
                pic_item(ui, app, l, None, &format!("brk_{k}"), "layout.break", json!({"kind": k}));
            }
        });
        menu_button(ui, app, "lineNumbers", Some("Line\nNumbers"), "Line Numbers", true, "layout.lineNumbers", |ui, app| {
            for (l, v) in
                [("None", "none"), ("Continuous", "continuous"), ("Restart Each Page", "restartPage"), ("Restart Each Section", "restartSection")]
            {
                pic_item(ui, app, l, None, &format!("ln_{v}"), "layout.lineNumbers", json!({"value": v}));
            }
            ui.separator();
            pic_item(ui, app, "Line Numbering Options…", None, "lnOptions", "ui.dialog", json!({"name": "lineNumbers"}));
        });
        let hy = app.session.doc.settings.auto_hyphenation;
        menu_button(ui, app, "hyphenation", Some("Hyphenation"), "Hyphenation", true, "layout.hyphenation", |ui, app| {
            mi(ui, app, if hy { "✓ Automatic" } else { "Automatic" }, "layout.hyphenation", json!({"value": true}));
            mi(ui, app, if hy { "None" } else { "✓ None" }, "layout.hyphenation", json!({"value": false}));
            mi(ui, app, "Manual…", "ui.dialog", json!({"name": "manualHyphenation"}));
            ui.separator();
            mi(ui, app, "Hyphenation Options…", "ui.dialog", json!({"name": "hyphenation"}));
        });
    });
    group(ui, "Paragraph", app, |ui, app| {
        // One-line ribbon: indents and spacing are in the side panel's Paragraph tab.
        if crate::widgets::line(ui).is_some() {
            return;
        }
        let rp = app.session.doc.para_at(&app.session.sel.focus).map(|p| app.session.doc.styles.resolve_para(&p.props));
        let Some(rp) = rp else { return };
        // Two rows of fields, as the two-size ribbon's two rows; each label names what it sets
        // (no header row); labels and fields sit on shared columns, every field the same width.
        const FIELD: egui::Vec2 = egui::Vec2::new(64.0, 20.0);
        ui.spacing_mut().interact_size.y = FIELD.y;
        // The gap before the Spacing column sits inside its cells: `add_space` in a grid moves the
        // next cell in release builds but trips a debug assertion.
        let spaced = |ui: &mut Ui, text: egui::RichText| {
            crate::widgets::hrow(ui, |ui| {
                ui.add_space(8.0);
                ui.label(text);
            });
        };
        egui::Grid::new("layout_para").spacing(vec2(4.0, crate::widgets::ROW_GAP + 8.0)).min_row_height(FIELD.y).show(ui, |ui| {
            // Indents in the File › Options unit.
            let u = crate::options::unit();
            let k = u.pt_per_unit();
            let mut l = rp.indent_left / k;
            let mut b = rp.space_before;
            ui.label(egui::RichText::new(tl!("Left indent:")).small());
            if ui
                .add_sized(FIELD, crate::numfield::NumField::new(&mut l).speed(3.6 / k).range(-792.0 / k..=1584.0 / k).measure(u).max_decimals(2))
                .changed()
            {
                let _ = app.run("para.indents", json!({"left": l * k}));
            }
            spaced(ui, egui::RichText::new(tl!("Space before:")).small());
            if ui.add_sized(FIELD, crate::numfield::NumField::new(&mut b).speed(1.0).range(0.0..=1584.0).suffix(format!(" {}", tl!("pt")))).changed()
            {
                let _ = app.run("para.spacing", json!({"before": b}));
            }
            ui.end_row();
            let mut r = rp.indent_right / k;
            let mut a = rp.space_after;
            ui.label(egui::RichText::new(tl!("Right indent:")).small());
            if ui
                .add_sized(FIELD, crate::numfield::NumField::new(&mut r).speed(3.6 / k).range(-792.0 / k..=1584.0 / k).measure(u).max_decimals(2))
                .changed()
            {
                let _ = app.run("para.indents", json!({"right": r * k}));
            }
            spaced(ui, egui::RichText::new(tl!("Space after:")).small());
            if ui.add_sized(FIELD, crate::numfield::NumField::new(&mut a).speed(1.0).range(0.0..=1584.0).suffix(format!(" {}", tl!("pt")))).changed()
            {
                let _ = app.run("para.spacing", json!({"after": a}));
            }
            ui.end_row();
        });
    });
    group(ui, "Arrange", app, |ui, app| {
        big(ui, app, "position", "Position", "arrange.position", json!({}), false);
        wrap_menu(ui, app);
        // One two-row block: order (forward, backward, Selection Pane), then align, group, rotate.
        stack(ui, |ui| {
            crate::widgets::row(ui, |ui| {
                small(ui, app, "bringForward", None, "Bring Forward", "arrange.bringForward", json!({}), false);
                small(ui, app, "sendBackward", None, "Send Backward", "arrange.sendBackward", json!({}), false);
                small(ui, app, "selectionPane", None, "Selection Pane", "arrange.selectionPane", json!({}), false);
            });
            ui.add_space(crate::widgets::ROW_GAP);
            crate::widgets::row(ui, |ui| {
                small(ui, app, "align", None, "Align", "arrange.align", json!({}), false);
                group_menu(ui, app, None);
                rotate_menu(ui, app, None);
            });
        });
    });
}

/// Arrange › Rotate: turn 90° either way or flip (the angle and mirroring, not the pixels).
/// Arrange › Wrap Text: how the selected object sits in the text, and (Square, Tight, Through)
/// which of its sides text flows on.
fn wrap_menu(ui: &mut Ui, app: &mut WordApp) {
    menu_button(ui, app, "wrapText", Some("Wrap\nText"), "Wrap Text", true, "arrange.wrapText", wrap_items);
}

/// Wrap Text's menu: the wrapping styles and Wrap Text Sides.
pub(crate) fn wrap_items(ui: &mut Ui, app: &mut WordApp) {
    let (wrap, side) = wordcraft_engine::cmd::objects::selected(&app.session)
        .and_then(|(_, o)| o.frame().map(|(_, _, f)| (f.wrap, f.wrap_text)))
        .unwrap_or_default();
    {
        use wordcraft_doc::para::{Wrap, WrapText};
        for (l, w, v) in [
            ("In Line with Text", Wrap::Inline, "inline"),
            ("Square", Wrap::Square, "square"),
            ("Tight", Wrap::Tight, "tight"),
            ("Through", Wrap::Through, "through"),
            ("Top and Bottom", Wrap::TopAndBottom, "topAndBottom"),
            ("Behind Text", Wrap::BehindText, "behindText"),
            ("In Front of Text", Wrap::InFrontOfText, "inFrontOfText"),
        ] {
            mi_check(ui, app, l, wrap == w, "arrange.wrap", json!({"wrap": v}));
        }
        ui.separator();
        ui.menu_button(tl!("Wrap Text Sides"), |ui| {
            for (l, s, v) in [
                ("Both Sides", WrapText::BothSides, "bothSides"),
                ("Left Only", WrapText::Left, "left"),
                ("Right Only", WrapText::Right, "right"),
                ("Largest Only", WrapText::Largest, "largest"),
            ] {
                mi_check(ui, app, l, side == s, "arrange.wrapText", json!({"side": v}));
            }
        });
    }
}

fn rotate_menu(ui: &mut Ui, app: &mut WordApp, label: Option<&str>) {
    menu_button(ui, app, "rotate", label, "Rotate", false, "arrange.rotate", rotate_items);
}

/// Rotate's menu: quarter turns and flips.
pub(crate) fn rotate_items(ui: &mut Ui, app: &mut WordApp) {
    mi(ui, app, "Rotate Right 90°", "arrange.rotate", json!({"direction": "right"}));
    mi(ui, app, "Rotate Left 90°", "arrange.rotate", json!({"direction": "left"}));
    mi(ui, app, "Flip Vertical", "arrange.rotate", json!({"direction": "flipVertical"}));
    mi(ui, app, "Flip Horizontal", "arrange.rotate", json!({"direction": "flipHorizontal"}));
}

/// Size › Rotation: the selected object's exact angle (a drag of the field is one undo step).
pub(crate) fn rotation_field(ui: &mut Ui, app: &mut WordApp) {
    let deg0 = wordcraft_engine::cmd::objects::selected(&app.session).and_then(|(_, o)| o.frame().map(|(_, _, f)| f.spin().deg)).unwrap_or(0.0);
    ui.label(egui::RichText::new(tl!("Rotation:")).small());
    let mut deg = deg0;
    let r = ui.add(crate::numfield::NumField::new(&mut deg).speed(1.0).range(-360.0..=360.0).max_decimals(1).suffix("°"));
    if r.changed() {
        if r.dragged() && !r.drag_started() {
            app.session.join_next_undo();
        }
        let _ = app.run("arrange.rotation", json!({"degrees": deg}));
    }
}

/// Arrange › Group: Group (Shift+click objects to select several) and Ungroup.
fn group_menu(ui: &mut Ui, app: &mut WordApp, label: Option<&str>) {
    menu_button(ui, app, "group", label, "Group", false, "arrange.group", |ui, app| {
        mi(ui, app, "Group", "arrange.group", json!({}));
        mi(ui, app, "Ungroup", "arrange.ungroup", json!({}));
    });
}

fn references(app: &mut WordApp, ui: &mut Ui) {
    // One group for the table of contents and the lists of figures and tables (owner,
    // 2026-11-22): Manage Table of Contents opens the Table of Contents pane, where each is
    // chosen, inserted and set up; Update All brings all three up to date.
    group(ui, "Table of Contents", app, |ui, app| {
        big(ui, app, "toc", "Manage Table\nof Contents", "ui.listsPane", json!({}), false);
        stack(ui, |ui| {
            small(ui, app, "updateTable", Some("Update All"), "Update All", "references.updateLists", json!({}), false);
            menu_button(ui, app, "addText", Some("Add Text"), "Add Text", false, "references.addText", |ui, app| {
                mi(ui, app, "Do Not Show in TOC", "references.addText", json!({"level": 0}));
                for l in 1..=3u64 {
                    mi(ui, app, &format!("Level {l}"), "references.addText", json!({"level": l}));
                }
            });
        });
    });
    group(ui, "Footnotes", app, |ui, app| {
        big(ui, app, "footnote", "Insert\nFootnote", "references.footnote", json!({}), false);
        stack(ui, |ui| {
            crate::widgets::row(ui, |ui| {
                small(ui, app, "endnote", Some("Insert Endnote"), "Insert Endnote", "references.endnote", json!({}), false);
                small(ui, app, "nextFootnote", Some("Next Footnote"), "Next Footnote", "references.nextFootnote", json!({}), false);
            });
            ui.add_space(crate::widgets::ROW_GAP);
            crate::widgets::row(ui, |ui| {
                small(ui, app, "showNotes", Some("Show Notes"), "Show Notes", "references.notes", json!({}), false);
                // Footnote and Endnote options (the group had a launcher for them).
                small(ui, app, "properties", None, "Footnote and Endnote", "references.noteOptions", json!({}), false);
            });
        });
    });
    group(ui, "Research", app, |ui, app| {
        big(ui, app, "researcher", "Researcher", "references.researcher", json!({}), false);
    });
    group(ui, "Citations & Bibliography", app, |ui, app| {
        // The document's sources to cite, then a new one (Create Source, #402).
        menu_button(ui, app, "citation", Some("Insert\nCitation"), "Insert Citation", true, "", |ui, app| {
            let sources: Vec<(String, String)> =
                app.session.doc.sources.iter().map(|s| (s.tag.clone(), format!("{} — {}", s.author, s.title))).collect();
            for (tag, label) in sources {
                mi_text(ui, app, &label, "references.citation", json!({"tag": tag}));
            }
            if !app.session.doc.sources.is_empty() {
                ui.separator();
            }
            mi(ui, app, "Add New Source…", "ui.dialog", json!({"name": "createSource"}));
        });
        stack(ui, |ui| {
            let current = app.session.bib_style.clone();
            crate::widgets::row(ui, |ui| {
                small(ui, app, "sources", None, "Manage Sources", "references.sources", json!({}), false);
                let tip = crate::i18n::fmt(tl!("Style: {style}"), &[("style", &current)]);
                menu_button(ui, app, "styles", None, &tip, false, "references.citationStyle", |ui, app| {
                    for st in &wordcraft_engine::cmd::citations::STYLES {
                        mi_check(ui, app, st.name, st.name == current, "references.citationStyle", json!({"style": st.name}));
                    }
                });
            });
            ui.add_space(crate::widgets::ROW_GAP);
            crate::widgets::row(ui, |ui| {
                small(ui, app, "bibliography", None, "Bibliography", "references.bibliography", json!({}), false);
            });
        });
    });
    group(ui, "Captions", app, |ui, app| {
        big(ui, app, "caption", "Insert\nCaption", "references.caption", json!({}), false);
        stack(ui, |ui| {
            small(ui, app, "crossRef", Some("Cross-reference"), "Cross-reference", "insert.crossReference", json!({}), false);
        });
    });
    group(ui, "Index", app, |ui, app| {
        big(ui, app, "markEntry", "Mark\nEntry", "references.markEntry", json!({}), false);
        stack(ui, |ui| {
            small(ui, app, "index", Some("Insert Index"), "Insert Index", "references.index", json!({}), false);
            small(ui, app, "update", Some("Update Index"), "Update Index", "references.updateIndex", json!({}), false);
        });
    });
    group(ui, "Table of Authorities", app, |ui, app| {
        big(ui, app, "markCitation", "Mark\nCitation", "references.markCitation", json!({}), false);
        stack(ui, |ui| {
            small(
                ui,
                app,
                "tableOfAuthorities",
                Some("Insert Table of Authorities"),
                "Insert Table of Authorities",
                "references.tableOfAuthorities",
                json!({}),
                false,
            );
            small(ui, app, "update", Some("Update Table"), "Update Table", "references.updateAuthorities", json!({}), false);
        });
    });
}

fn mailings(app: &mut WordApp, ui: &mut Ui) {
    group(ui, "Create", app, |ui, app| {
        big(ui, app, "envelope", "Envelopes", "mailings.envelopes", json!({}), false);
        big(ui, app, "labels", "Labels", "mailings.labels", json!({}), false);
    });
    group(ui, "Start Mail Merge", app, |ui, app| {
        // The kinds of merge document the engine makes, the current one checked.
        menu_button(ui, app, "mailMerge", Some("Start Mail\nMerge"), "Start Mail Merge", true, "mailings.start", |ui, app| {
            let kind = app.session.merge.kind.clone();
            for (label, k) in [
                ("Letters", "letters"),
                ("E-mail Messages", "emails"),
                ("Envelopes", "envelopes"),
                ("Labels", "labels"),
                ("Directory", "directory"),
                ("Normal Word Document", "normal"),
            ] {
                let current = kind == k || (k == "normal" && kind.is_empty());
                mi_check(ui, app, label, current, "mailings.start", json!({"kind": k}));
            }
        });
        // The command needs data, so the button offers the two ways to give it (#240).
        menu_button(ui, app, "recipients", Some("Select\nRecipients"), "Select Recipients", true, "mailings.recipients", |ui, app| {
            mi(ui, app, "Type a New List…", "ui.dialog", json!({"name": "newRecipientList"}));
            mi(ui, app, "Use an Existing List…", "ui.openRecipientList", json!({}));
        });
        big(ui, app, "editRecipients", "Edit\nRecipient List", "mailings.editRecipients", json!({}), false);
    });
    group(ui, "Write & Insert Fields", app, |ui, app| {
        big(ui, app, "highlightFields", "Highlight\nMerge Fields", "mailings.highlightFields", json!({}), false);
        big(ui, app, "addressBlock", "Address\nBlock", "mailings.addressBlock", json!({}), false);
        big(ui, app, "greetingLine", "Greeting\nLine", "mailings.greetingLine", json!({}), false);
        big(ui, app, "mergeField", "Insert Merge\nField", "mailings.insertField", json!({}), false);
        stack(ui, |ui| {
            // The merge rules the engine knows; If and Skip Record If ask for their condition.
            menu_button(ui, app, "rules", Some("Rules"), "Rules", false, "mailings.rules", |ui, app| {
                mi(ui, app, "If…Then…Else…", "mailings.rules", json!({"rule": "IF"}));
                mi(ui, app, "Merge Record #", "mailings.rules", json!({"rule": "MERGEREC"}));
                mi(ui, app, "Next Record", "mailings.rules", json!({"rule": "NEXT"}));
                mi(ui, app, "Skip Record If…", "mailings.rules", json!({"rule": "SKIPIF"}));
            });
            small(ui, app, "matchFields", Some("Match Fields"), "Match Fields", "mailings.matchFields", json!({}), false);
        });
    });
    group(ui, "Preview Results", app, |ui, app| {
        big(ui, app, "preview", "Preview\nResults", "mailings.preview", json!({}), false);
        stack(ui, |ui| {
            crate::widgets::row(ui, |ui| {
                small(ui, app, "previous", None, "Previous Record", "mailings.previous", json!({}), false);
                small(ui, app, "next", None, "Next Record", "mailings.next", json!({}), false);
            });
            small(ui, app, "find", Some("Find Recipient"), "Find Recipient", "mailings.findRecipient", json!({}), false);
            small(ui, app, "checkErrors", Some("Check for Errors"), "Check for Errors", "mailings.checkErrors", json!({}), false);
        });
    });
    group(ui, "Finish", app, |ui, app| {
        big(ui, app, "finish", "Finish &\nMerge", "mailings.finish", json!({}), false);
    });
}

/// Zotero (`docs/zotero.md`): the Zotero desktop app does the work in its own window.
fn zotero(app: &mut WordApp, ui: &mut Ui) {
    group(ui, "Citations", app, |ui, app| {
        big(ui, app, "citation", "Add/Edit\nCitation", "ui.zotero.addEditCitation", json!({}), false);
        big(ui, app, "addNote", "Add\nNote", "ui.zotero.addNote", json!({}), false);
        big(ui, app, "pastCitation", "Move Past\nCitation", "caret.pastCitation", json!({}), false);
    });
    group(ui, "Bibliography", app, |ui, app| {
        big(ui, app, "bibliography", "Add/Edit\nBibliography", "ui.zotero.addEditBibliography", json!({}), false);
    });
    group(ui, "Document", app, |ui, app| {
        big(ui, app, "update", "Refresh", "ui.zotero.refresh", json!({}), false);
        big(ui, app, "docPrefs", "Document\nPreferences", "ui.zotero.setDocPrefs", json!({}), false);
        big(ui, app, "unlinkCitations", "Unlink\nCitations", "ui.zotero.removeCodes", json!({}), false);
    });
}

fn review(app: &mut WordApp, ui: &mut Ui) {
    group(ui, "Proofing", app, |ui, app| {
        big(ui, app, "spelling", "Spelling &\nGrammar", "review.spelling", json!({}), false);
        stack(ui, |ui| {
            small(ui, app, "thesaurus", Some("Thesaurus"), "Thesaurus", "review.thesaurus", json!({}), false);
            small(ui, app, "wordCount", Some("Word Count"), "Word Count", "ui.dialog", json!({"name": "wordCount"}), false);
        });
    });
    group(ui, "Speech", app, |ui, app| {
        big(ui, app, "readAloud", "Read\nAloud", "review.readAloud", json!({}), false);
    });
    group(ui, "Accessibility", app, |ui, app| {
        big(ui, app, "accessibility", "Check\nAccessibility", "file.accessibility", json!({}), false);
    });
    group(ui, "Language", app, |ui, app| {
        big(ui, app, "translate", "Translate", "review.translate", json!({}), false);
        menu_button(ui, app, "language", Some("Language"), "Language", true, "", language_menu);
    });
    group(ui, "Comments", app, |ui, app| {
        big(ui, app, "newComment", "New\nComment", "review.newComment", json!({}), false);
        stack(ui, |ui| {
            crate::widgets::row(ui, |ui| {
                small(ui, app, "deleteComment", Some("Delete"), "Delete Comment", "review.deleteComment", json!({}), false);
                small(ui, app, "prevComment", Some("Previous"), "Previous Comment", "review.previousComment", json!({}), false);
            });
            ui.add_space(crate::widgets::ROW_GAP);
            crate::widgets::row(ui, |ui| {
                small(ui, app, "nextComment", Some("Next"), "Next Comment", "review.nextComment", json!({}), false);
            });
        });
        let shown = app.session.view.comments_pane;
        big(ui, app, "showComments", "Show\nComments", "view.commentsPane", json!({"value": !shown}), false);
    });
    group(ui, "Tracking", app, |ui, app| {
        let on = app.session.doc.settings.track_changes;
        let r = big(ui, app, "trackChanges", if on { "Track\nChanges ✓" } else { "Track\nChanges" }, "review.trackChanges", json!({}), false);
        let _ = r;
        stack(ui, |ui| {
            menu_button(
                ui,
                app,
                "markup",
                Some(if app.session.view.show_markup { "All Markup" } else { "No Markup" }),
                "Display for Review",
                false,
                "review.markup",
                |ui, app| {
                    mi(ui, app, "All Markup", "review.markup", json!({"value": "all"}));
                    mi(ui, app, "No Markup", "review.markup", json!({"value": "noMarkup"}));
                    // Track Changes Options (the group had a launcher for them).
                    ui.separator();
                    mi(ui, app, "Track Changes Options", "review.trackingOptions", json!({}));
                },
            );
            small(ui, app, "reviewingPane", Some("Reviewing Pane"), "Reviewing Pane", "review.changes", json!({}), false);
        });
    });
    group(ui, "Changes", app, |ui, app| {
        menu_button(ui, app, "accept", Some("Accept"), "Accept", true, "review.accept", |ui, app| {
            mi(ui, app, "Accept This Change", "review.accept", json!({}));
            mi(ui, app, "Accept All Changes", "review.acceptAll", json!({}));
        });
        menu_button(ui, app, "reject", Some("Reject"), "Reject", true, "review.reject", |ui, app| {
            mi(ui, app, "Reject Change", "review.reject", json!({}));
            mi(ui, app, "Reject All Changes", "review.rejectAll", json!({}));
        });
        stack(ui, |ui| {
            small(ui, app, "prevChange", Some("Previous"), "Previous Change", "review.previousChange", json!({}), false);
            small(ui, app, "nextChange", Some("Next"), "Next Change", "review.nextChange", json!({}), false);
        });
    });
    group(ui, "Compare", app, |ui, app| {
        big(ui, app, "compare", "Compare", "review.compare", json!({}), false);
    });
    group(ui, "Protect", app, |ui, app| {
        big(ui, app, "blockAuthors", "Block\nAuthors", "review.blockAuthors", json!({}), false);
        big(ui, app, "restrict", "Restrict\nEditing", "review.restrict", json!({}), false);
    });
    group(ui, "Ink", app, |ui, app| {
        let hidden = app.session.view.hide_ink;
        big_toggle(ui, app, "hideInk", "Hide\nInk", "review.hideInk", hidden);
    });
}

/// Developer › Controls: content controls.
fn developer(app: &mut WordApp, ui: &mut Ui) {
    let design = app.session.view.design_mode;
    group(ui, "Controls", app, |ui, app| {
        let rows: [[(&str, &str, &str); 3]; 3] = [
            [
                ("ccRichText", "Rich Text Content Control", "developer.richText"),
                ("ccPlainText", "Plain Text Content Control", "developer.plainText"),
                ("ccPicture", "Picture Content Control", "developer.picture"),
            ],
            [
                ("ccGallery", "Building Block Gallery Content Control", "developer.buildingBlock"),
                ("ccCheckBox", "Check Box Content Control", "developer.checkBox"),
                ("ccComboBox", "Combo Box Content Control", "developer.comboBox"),
            ],
            [
                ("ccDropDown", "Drop-Down List Content Control", "developer.dropDown"),
                ("ccDate", "Date Picker Content Control", "developer.datePicker"),
                ("ccRepeat", "Repeating Section Content Control", "developer.repeatingSection"),
            ],
        ];
        // Three rows of small buttons fill the group's height, like every other stack.
        stack(ui, |ui| {
            for row in rows {
                crate::widgets::row(ui, |ui| {
                    for (icon, tip, id) in row {
                        small(ui, app, icon, None, tip, id, json!({}), false);
                    }
                });
            }
        });
        stack(ui, |ui| {
            small(ui, app, "designMode", Some("Design Mode"), "Design Mode", "developer.designMode", json!({}), design);
            small(ui, app, "properties", Some("Properties"), "Properties", "developer.properties", json!({}), false);
        });
    });
}

fn view(app: &mut WordApp, ui: &mut Ui) {
    let v = app.session.view.clone();
    group(ui, "Views", app, |ui, app| {
        big(ui, app, "readMode", "Read\nMode", "view.readMode", json!({}), false);
        big(ui, app, "printLayout", "Print\nLayout", "view.printLayout", json!({}), false);
        stack(ui, |ui| {
            crate::widgets::row(ui, |ui| {
                small(ui, app, "outline", Some("Outline"), "Outline", "view.outline", json!({}), false);
                small(ui, app, "draft", Some("Draft"), "Draft", "view.draft", json!({}), false);
            });
            ui.add_space(crate::widgets::ROW_GAP);
            crate::widgets::row(ui, |ui| {
                small(ui, app, "webLayout", Some("Web Layout"), "Web Layout", "view.webLayout", json!({}), false);
            });
        });
    });
    group(ui, "Immersive", app, |ui, app| {
        big(ui, app, "focus", "Focus", "view.focus", json!({}), false);
        big(ui, app, "immersive", "Immersive\nReader", "view.immersive", json!({}), false);
    });
    group(ui, "Show", app, |ui, app| {
        stack(ui, |ui| {
            crate::widgets::hrow(ui, |ui| {
                small(ui, app, "ruler", Some("Ruler"), "Ruler", "view.ruler", json!({}), v.ruler);
                small(ui, app, "gridlines", Some("Gridlines"), "Gridlines", "view.gridlines", json!({}), v.gridlines);
            });
            ui.add_space(crate::widgets::ROW_GAP);
            crate::widgets::hrow(ui, |ui| {
                small(ui, app, "textBoundaries", Some("Text Boundaries"), "Text Boundaries", "view.textBoundaries", json!({}), v.text_boundaries);
                let available = crate::canvas::can_hide_white_space(app);
                ui.add_enabled_ui(available, |ui| {
                    small(
                        ui,
                        app,
                        "hideWhiteSpace",
                        Some("Hide White Space"),
                        "Hide White Space",
                        "view.hideWhiteSpace",
                        json!({}),
                        v.hide_white_space && available,
                    )
                    .on_disabled_hover_text(tl!("Available in single-column Print Layout outside headers and footers."));
                });
            });
        });
        big_toggle(ui, app, "navPane", "Navigation\nPane", "view.navigationPane", v.nav_pane);
    });
    group(ui, "Zoom", app, |ui, app| {
        big(ui, app, "printPreview", "Print\nPreview", "ui.printPreview", json!({}), false);
        stack(ui, |ui| {
            crate::widgets::hrow(ui, |ui| {
                small(ui, app, "zoom", Some("Zoom"), "Zoom", "ui.dialog", json!({"name": "zoom"}), false);
                paragraph_separator(ui);
                small(ui, app, "zoomOut", None, "Zoom Out", "view.zoomOut", json!({}), false);
                small(ui, app, "zoom100", Some("100%"), "100%", "view.zoom100", json!({}), false);
                small(ui, app, "zoomIn", None, "Zoom In", "view.zoomIn", json!({}), false);
            });
            crate::widgets::hrow(ui, |ui| {
                small(ui, app, "onePage", Some("One Page"), "One Page", "view.onePage", json!({}), v.fit == "onePage");
                small(ui, app, "multiplePages", Some("Multiple Pages"), "Multiple Pages", "view.multiplePages", json!({}), v.multi_page);
            });
        });
        big_toggle(ui, app, "pageWidth", "Page\nWidth", "view.pageWidth", v.fit == "pageWidth");
    });
    group(ui, "Dark Mode", app, |ui, app| {
        big(ui, app, "darkMode", "Switch\nModes", "view.darkMode", json!({}), false);
        menu_button(ui, app, "interfaceTheme", Some("Interface\nTheme"), "Interface Theme", true, "", |ui, app| {
            for a in crate::theme::Appearance::ALL {
                if ui
                    .steady_scope(app.ui.theme == a, |ui| {
                        ui.add(egui::Button::selectable(app.ui.theme == a, tl!(a.label())).min_size(vec2(200.0, 0.0)))
                    })
                    .clicked()
                {
                    let _ = app.run("ui.theme", json!({"value": a.code()}));
                    ui.close();
                }
            }
        });
    });
    group(ui, "Window", app, |ui, app| {
        big(ui, app, "newWindow", "New\nWindow", "view.newWindow", json!({}), false);
        big(ui, app, "arrangeAll", "Arrange\nAll", "view.arrangeAll", json!({}), false);
        big(ui, app, "split", "Split", "view.split", json!({}), false);
        big(ui, app, "fullScreen", "Full\nScreen", "view.fullScreen", json!({}), v.full_screen);
    });
    group(ui, "Templates", app, |ui, app| {
        big(ui, app, "documentTemplate", "Templates", "tools.templates", json!({}), false);
    });
}

/// The Outlining tab (Outline view): outline levels, moving, expanding and collapsing, what is
/// shown, and closing the view.
fn outlining(app: &mut WordApp, ui: &mut Ui) {
    use wordcraft_engine::layout::outline::{BODY, para_level};
    let o = app.session.view.outline.clone();
    let level = app.session.doc.para_at(&app.session.sel.focus).map(|p| para_level(&app.session.doc, p)).unwrap_or(BODY);
    let name = |l: u8| if l >= BODY { tl!("Body Text").to_string() } else { crate::i18n::fmt(tl!("Level {n}"), &[("n", &l.to_string())]) };
    group(ui, "Outline Tools", app, |ui, app| {
        stack(ui, |ui| {
            crate::widgets::row(ui, |ui| {
                small(ui, app, "outlineTop", None, "Promote to Heading 1", "outline.promoteToHeading1", json!({}), false);
                small(ui, app, "outlinePromote", None, "Promote", "outline.promote", json!({}), false);
                let levels: Vec<String> = (1..=BODY).map(name).collect();
                if let Some(v) = combo(ui, "outlineLevel", 100.0, &name(level), &levels, None)
                    && let Some(i) = levels.iter().position(|x| *x == v)
                {
                    let l = if i + 1 >= usize::from(BODY) { json!("body") } else { json!(i + 1) };
                    let _ = app.run("outline.level", json!({ "level": l }));
                }
                small(ui, app, "outlineDemote", None, "Demote", "outline.demote", json!({}), false);
                small(ui, app, "outlineBody", None, "Demote to Body Text", "outline.demoteToBody", json!({}), false);
            });
            crate::widgets::row(ui, |ui| {
                small(ui, app, "outlineUp", None, "Move Up", "outline.moveUp", json!({}), false);
                small(ui, app, "outlineDown", None, "Move Down", "outline.moveDown", json!({}), false);
                small(ui, app, "outlineExpand", None, "Expand", "outline.expand", json!({}), false);
                small(ui, app, "outlineCollapse", None, "Collapse", "outline.collapse", json!({}), false);
                // On the one-line ribbon the label belongs to the Show Level box's key, so it is
                // drawn once, with the box.
                if crate::widgets::kept_named(ui, "outlineShowLevel", 150.0, "Show Level") {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(tl!("Show Level")).size(11.5));
                    ui.add_space(4.0);
                }
                let mut shown: Vec<String> = vec![tl!("All Levels").to_string()];
                shown.extend((1..BODY).map(name));
                let current = shown.get(usize::from(o.show_level)).cloned().unwrap_or_default();
                if let Some(v) = combo(ui, "outlineShowLevel", 90.0, &current, &shown, None)
                    && let Some(i) = shown.iter().position(|x| *x == v)
                {
                    let l = if i == 0 { json!("all") } else { json!(i) };
                    let _ = app.run("outline.showLevel", json!({ "level": l }));
                }
            });
            crate::widgets::row(ui, |ui| {
                let mut f = !o.hide_formatting;
                if crate::widgets::kept_named(ui, "outline.showFormatting", 170.0, "Show Text Formatting")
                    && ui.checkbox(&mut f, tl!("Show Text Formatting")).changed()
                {
                    let _ = app.run("outline.showFormatting", json!({ "value": f }));
                }
                let mut first = o.first_line_only;
                if crate::widgets::kept_named(ui, "outline.firstLineOnly", 160.0, "Show First Line Only")
                    && ui.checkbox(&mut first, tl!("Show First Line Only")).changed()
                {
                    let _ = app.run("outline.firstLineOnly", json!({ "value": first }));
                }
            });
        });
    });
    group(ui, "Close", app, |ui, app| {
        big(ui, app, "closeOutline", "Close Outline\nView", "outline.close", json!({}), false);
    });
}

fn help(app: &mut WordApp, ui: &mut Ui) {
    group(ui, "Help", app, |ui, app| {
        big(ui, app, "help", "Help", "ui.dialog", json!({"name": "about"}), false);
        if app.services.latest_release.is_some() {
            big(ui, app, "checkUpdates", "Check for\nUpdates", "ui.checkForUpdates", json!({}), false);
        }
    });
    group(ui, "Agents", app, |ui, app| {
        big(ui, app, "macros", "Commands", "ui.dialog", json!({"name": "commands"}), false);
    });
}

/// SmartArt Design: the selected graphic's items (Add Shape, Promote, Demote, Text Pane), its
/// layout (a gallery of previews), its colours and Reset.
/// A width or height box for a picture, shape or chart: `pt` (points) shown and typed in the unit of
/// File › Options › Advanced (inches, centimetres…; `5 cm` typed in inches converts), as the table
/// boxes are. Returns the box and the value in points.
pub(crate) fn size_box(ui: &mut Ui, pt: f32, min_pt: f32, max_pt: f32) -> (egui::Response, f32) {
    let unit = crate::options::unit();
    let k = unit.pt_per_unit();
    let mut v = pt / k;
    let r = ui.add(crate::numfield::NumField::new(&mut v).speed(1.0 / k).range(min_pt / k..=max_pt / k).measure(unit).max_decimals(2));
    (r, v * k)
}

/// The table style gallery (Table pane): a few tiles with arrows to page through them and "more", which
/// opens every table style in a panel grouped as Plain, Grid and List tables (the document's own
/// first), with New, Modify, Delete and Clear below. `current` is the caret table's style id and
/// `builtin` whether that style is one of ours (`None`: not in a table).
pub(crate) fn table_style_gallery(
    ui: &mut Ui,
    app: &mut WordApp,
    styles: &[(bool, String, String)],
    current_id: Option<&str>,
    builtin: Option<bool>,
    visible: usize,
) {
    let t = Tokens::get(ui.ctx());
    let tile = vec2(62.0, 52.0);
    // The panel's tiles are larger, like Word's, so a style's bands and borders read clearly.
    let panel_tile = vec2(92.0, 68.0);
    let pitch = tile.x + 3.0;
    let key = egui::Id::new("table_gallery_start");
    let start = ui.data(|d| d.get_temp::<usize>(key)).unwrap_or(0).min(styles.len().saturating_sub(1));
    let (area, _) = ui.allocate_exact_size(vec2(visible as f32 * pitch + 22.0, CONTENT_H), Sense::hover());
    ui.painter().rect(area.shrink(1.0), 4.0, t.input, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    for (k, (_, id, name)) in styles.iter().skip(start).take(visible).enumerate() {
        let r = Rect::from_min_size(pos2(area.min.x + 3.0 + k as f32 * pitch, area.center().y - tile.y / 2.0), tile);
        if crate::previews::table_style_tile(ui, app, id, name, r, current_id == Some(id.as_str())).clicked() {
            let _ = app.run("table.style", json!({"style": id}));
        }
    }
    // Scroll arrows and "more".
    let ar = Rect::from_min_max(pos2(area.max.x - 17.0, area.min.y + 2.0), pos2(area.max.x - 2.0, area.max.y - 2.0));
    let thirds = ar.height() / 3.0;
    let mut more = None;
    for (i, (icon, delta)) in [("chevronUp", -1i32), ("chevronDown", 1), ("more", 0)].iter().enumerate() {
        let br = Rect::from_min_size(pos2(ar.min.x, ar.min.y + thirds * i as f32), vec2(ar.width(), thirds));
        let resp = ui.interact(br, ui.id().with(("table_gal_arrow", i)), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(br, 2.0, t.hover);
        }
        crate::icons::paint(ui.painter(), br.shrink(2.0), icon, t.icon, t.accent);
        let tip = match delta {
            0 => "More",
            d if *d < 0 => "Previous",
            _ => "Next",
        };
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tl!(tip)));
        if *delta == 0 {
            more = Some(resp);
        } else if resp.clicked() {
            let n = if *delta < 0 { start.saturating_sub(visible) } else { (start + visible).min(styles.len().saturating_sub(1)) };
            ui.data_mut(|d| d.insert_temp(key, n));
        }
    }
    let Some(more) = more else { return };
    let can_modify = builtin.is_some();
    // Built-in table styles can't be deleted.
    let can_delete = builtin == Some(false);
    egui::Popup::menu(&more).show(|ui| {
        let cols = 5usize;
        let gap = 4.0;
        ui.set_width(cols as f32 * (panel_tile.x + gap) + 8.0);
        let group_of = |builtin: bool, id: &str| match (builtin, id) {
            (false, _) => 0,
            (true, i) if i.starts_with("GridTable") => 2,
            (true, i) if i.starts_with("ListTable") => 3,
            _ => 1,
        };
        egui::ScrollArea::vertical().max_height(380.0).auto_shrink([false, true]).id_salt("table_styles_panel").show(ui, |ui| {
            for (g, title) in [(0, "Custom"), (1, "Plain Tables"), (2, "Grid Tables"), (3, "List Tables")] {
                let members: Vec<&(bool, String, String)> = styles.iter().filter(|(b, id, _)| group_of(*b, id) == g).collect();
                if members.is_empty() {
                    continue;
                }
                ui.label(egui::RichText::new(tl!(title)).font(crate::theme::semibold(12.0)));
                ui.add_space(2.0);
                for row in members.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = vec2(gap, gap);
                        for (_, id, name) in row {
                            let (r, _) = ui.allocate_exact_size(panel_tile, Sense::hover());
                            if crate::previews::table_style_tile(ui, app, id, name, r, current_id == Some(id.as_str())).clicked() {
                                let _ = app.run("table.style", json!({"style": id}));
                                ui.close();
                            }
                        }
                    });
                }
                ui.add_space(4.0);
            }
        });
        ui.separator();
        if ui.add_enabled(can_modify, egui::Button::new(tl!("Modify Table Style…")).min_size(vec2(200.0, 0.0))).clicked() {
            let _ = app.run("ui.dialog", json!({"name": "modifyTableStyle"}));
            ui.close();
        }
        if ui.add_enabled(can_delete, egui::Button::new(tl!("Delete Table Style")).min_size(vec2(200.0, 0.0))).clicked() {
            if let Err(e) = app.run("table.deleteStyle", json!({})) {
                app.status(e);
            }
            ui.close();
        }
        if ui.add_enabled(can_modify, egui::Button::new(tl!("Clear")).min_size(vec2(200.0, 0.0))).clicked() {
            if let Err(e) = app.run("table.style", json!({"style": "TableNormal"})) {
                app.status(e);
            }
            ui.close();
        }
        if ui.add(egui::Button::new(tl!("New Table Style…")).min_size(vec2(200.0, 0.0))).clicked() {
            let _ = app.run("ui.dialog", json!({"name": "newTableStyle"}));
            ui.close();
        }
    });
}

/// Table pane › Cell Size: the caret cell's row height and column width, in inches. They show
/// the size on the page (a row without a set height shows the height its text gives it) and
/// editing runs `table.rowHeight` / `table.columnWidth`; dragging one is a single Undo.
/// The caret's cell size in points, as the boxes show it: the row's height, the cell's width, and
/// whether the height is exact (not "at least"). `None` outside a table.
pub(crate) fn cell_size_now(app: &mut WordApp) -> Option<(f32, f32, bool)> {
    let story = app.session.sel.focus.story;
    let (tp, r, c) = app.session.sel.focus.path.cell()?;
    let t = app.session.doc.table(story, &tp)?;
    let row = t.rows.get(r).map(|x| x.props.clone()).unwrap_or_default();
    let stored_w = t.grid.get(t.grid_col(r, c)).copied();
    let laid = app.session.layout().pages.iter().flat_map(|pg| pg.items.iter()).find_map(|it| match it {
        wordcraft_layout::Placed::Cell { rect, table, row, cell, story: st, .. } if *table == tp && *row == r && *cell == c && *st == story => {
            Some(*rect)
        }
        _ => None,
    });
    // The row's own height: a vertically merged cell's box spans several rows, so take the
    // shortest box in this row.
    let row_h = app
        .session
        .layout()
        .pages
        .iter()
        .flat_map(|pg| pg.items.iter())
        .filter_map(|it| match it {
            wordcraft_layout::Placed::Cell { rect, table, row, story: st, .. } if *table == tp && *row == r && *st == story => Some(rect.h),
            _ => None,
        })
        .reduce(f32::min);
    let h0 = row.height.or(row_h).unwrap_or(0.0);
    let w0 = laid.map(|x| x.w).or(stored_w).unwrap_or(0.0);
    Some((h0, w0, row.height_rule == wordcraft_doc::props::HeightRule::Exact))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordcraft_engine::Session;

    fn png_data() -> String {
        let img = image::RgbaImage::from_fn(20, 10, |x, y| {
            if x > 2 && x < 17 && y > 2 && y < 7 { image::Rgba([200, 30, 30, 255]) } else { image::Rgba([255, 255, 255, 255]) }
        });
        let mut b = Vec::new();
        image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut b), image::ImageFormat::Png).unwrap();
        wordcraft_engine::cmd::insert::base64_encode(&b)
    }

    /// The user icon's menu (owner, 2026-11-18; the Editing button's menu before) offers View only:
    /// choosing it makes the document read only (typing does nothing); Editing brings it back.
    #[test]
    fn editing_menu_offers_view_only() {
        use egui_kittest::kittest::Queryable;
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.run("text.insert", json!({"text": "Hello"})).unwrap();
        let mut h = egui_kittest::Harness::builder().with_size(vec2(1440.0, 300.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        h.run_steps(4);
        let text = |h: &egui_kittest::Harness<WordApp>| h.state().session.doc.plain_text(wordcraft_doc::StoryRef::Body);
        h.get_by_label("Account").click();
        h.run_steps(4);
        h.get_by_label("View only — no changes or suggestions").click();
        h.run_steps(4);
        assert!(h.state().session.view_only, "the menu entry turns view only on");
        assert!(h.state_mut().run("text.insert", json!({"text": "!"})).is_err(), "typing is refused");
        assert_eq!(text(&h), "Hello");
        h.get_by_label("Account").click();
        h.run_steps(4);
        h.get_by_label("Editing — edit the document directly").click();
        h.run_steps(4);
        assert!(!h.state().session.view_only, "Editing turns it off again");
        h.state_mut().run("text.insert", json!({"text": "!"})).unwrap();
        assert_eq!(text(&h), "Hello!");
    }

    /// Opens the one-line ribbon's More (`label`, "More" in English), which draws the whole tab as
    /// the two-row ribbon; that copy of a button is the lowest one with its label.
    fn open_more(h: &mut egui_kittest::Harness<WordApp>, label: &str) {
        use egui_kittest::kittest::Queryable;
        let more = h.get_all_by_label(label).map(|n| n.rect()).min_by(|a, b| a.min.y.total_cmp(&b.min.y)).expect("the line has More");
        click_at(h, more.center());
        h.run_steps(4);
    }

    /// A primary click at `pos` (window points).
    fn click_at(h: &mut egui_kittest::Harness<WordApp>, pos: egui::Pos2) {
        h.hover_at(pos);
        h.step();
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() });
            h.step();
        }
    }

    /// The lowest button labelled `label`: in More's two-row drawing when it is open.
    fn lowest(h: &egui_kittest::Harness<WordApp>, label: &str) -> egui::Rect {
        use egui_kittest::kittest::Queryable;
        h.get_all_by_label(label).map(|n| n.rect()).max_by(|a, b| a.min.y.total_cmp(&b.min.y)).unwrap_or_else(|| panic!("no {label}"))
    }

    /// The highest button labelled `label`: the one on the line.
    fn highest(h: &egui_kittest::Harness<WordApp>, label: &str) -> egui::Rect {
        use egui_kittest::kittest::Queryable;
        h.get_all_by_label(label).map(|n| n.rect()).min_by(|a, b| a.min.y.total_cmp(&b.min.y)).unwrap_or_else(|| panic!("no {label}"))
    }

    /// Home › Paste on the line is a split button: the icon pastes, and the strip beside it opens
    /// the Paste menu.
    #[test]
    fn paste_is_a_split_button_whose_strip_opens_the_menu() {
        use egui_kittest::kittest::Queryable;
        let app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        let mut h = egui_kittest::Harness::builder().with_size(vec2(1440.0, 900.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        h.run_steps(6);
        let main = highest(&h, "Paste");
        let arrow = highest(&h, "Paste options");
        assert!(arrow.min.x >= main.max.x - 0.5 && (arrow.center().y - main.center().y).abs() < 1.0, "the strip is beside it: {arrow:?} {main:?}");
        assert!(h.query_by_label("Keep Text Only").is_none(), "the menu starts closed");
        click_at(&mut h, arrow.center());
        h.run_steps(4);
        assert!(h.query_by_label("Keep Text Only").is_some(), "the line's strip opens the Paste menu");
    }

    /// The one bar (owner, 2026-11-18) ends with the user icon alone: no search box, Share, Report
    /// a problem or Discord button beside it; search lives in its menu. No Discord link is left
    /// anywhere either: not on the Help tab, not in File, not as a command.
    #[test]
    fn the_bar_ends_with_the_user_icon_and_no_discord() {
        use egui_kittest::kittest::Queryable;
        let app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        let mut h = egui_kittest::Harness::builder().with_size(vec2(1440.0, 300.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        h.run_steps(4);
        let account = h.get_by_label("Account").rect();
        assert!(account.max.x > 1400.0 && account.min.y < krub_icons::chrome::ONE_BAR_H, "the user icon ends the bar: {account:?}");
        for gone in ["Share", "Report a problem", "Search commands and help", "Comments"] {
            assert!(h.query_by_label(gone).is_none(), "{gone} is not on the bar");
        }
        h.get_by_label("Account").click();
        h.run_steps(4);
        for item in ["Search commands and help", "Editing — edit the document directly"] {
            assert!(h.query_by_label(item).is_some(), "the user menu has {item}");
        }
        assert!(h.query_by_label("Comments").is_none(), "Comments moved to the file name's menu (owner, 2026-11-23)");
        h.key_press(egui::Key::Escape);
        h.run_steps(2);
        let stem = h.state().title_stem();
        h.get_by_label_contains(&format!("{stem} — ")).click();
        h.run_steps(4);
        assert!(h.query_by_label("Comments").is_some(), "the file name's menu has Comments");
        h.key_press(egui::Key::Escape);
        h.run_steps(2);
        h.state_mut().ui.tab = "Help".into();
        h.run_steps(4);
        assert!(h.query_by_label_contains("Discord").is_none(), "the Help tab has no Discord button");
        h.state_mut().ui.backstage = true;
        h.run_steps(4);
        assert!(h.query_by_label_contains("Discord").is_none(), "File has no Discord item");
        assert!(h.state_mut().run("ui.discord", json!({})).is_err(), "ui.discord is no longer a command");
    }

    /// The View tab's Show buttons are in More's tray, side by side in their group, and Text
    /// Boundaries (on by default) turns the outline around the text area off and on from the line.
    #[test]
    fn view_tab_has_a_text_boundaries_toggle() {
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.run("ui.tab", json!({"tab": "View"})).unwrap();
        let mut h = egui_kittest::Harness::builder().with_size(vec2(1440.0, 900.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        h.run_steps(6);
        open_more(&mut h, "More");
        let tile = |label: &str| lowest(&h, label);
        let (ruler, gridlines, boundaries) = (tile("Ruler"), tile("Gridlines"), tile("Text Boundaries"));
        assert!(ruler.min.y > 100.0, "in the tray, under the line: {ruler:?}");
        assert!((gridlines.center().y - ruler.center().y).abs() < 1.0 || gridlines.min.y > ruler.max.y, "Show's buttons together");
        assert!(boundaries.min.y >= ruler.min.y - 0.5);
        assert!(h.state().session.view.text_boundaries, "on by default");
        h.key_press(egui::Key::Escape);
        h.run_steps(3);
        let at = highest(&h, "Text Boundaries").center();
        click_at(&mut h, at);
        h.run_steps(3);
        assert!(!h.state().session.view.text_boundaries, "the line's button turns it off");
        let at = highest(&h, "Text Boundaries").center();
        click_at(&mut h, at);
        h.run_steps(3);
        assert!(h.state().session.view.text_boundaries, "and on again");
    }

    /// Table Styles (the gallery in the right panel's Table pane): "More" opens a panel of every style in groups, with New,
    /// Modify, Delete and Clear below; picking a tile styles the caret's table.
    #[test]
    fn table_styles_more_panel_lists_groups_and_applies_a_style() {
        use egui_kittest::kittest::Queryable;
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.run("insert.table", json!({"rows": 2, "cols": 2})).unwrap();
        app.run("ui.tab", json!({"tab": "Table Design"})).unwrap();
        let mut h = egui_kittest::Harness::builder().with_size(vec2(1440.0, 900.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        h.run_steps(4);
        // The gallery's More, in the right panel's Table pane (the line's More is above it).
        let at = lowest(&h, "More").center();
        click_at(&mut h, at);
        h.run_steps(4);
        for group in ["Plain Tables", "Grid Tables", "List Tables"] {
            assert!(h.query_by_label(group).is_some(), "{group} group");
        }
        for action in ["Modify Table Style…", "Delete Table Style", "Clear", "New Table Style…"] {
            assert!(h.query_by_label(action).is_some(), "{action}");
        }
        let tile = h.get_all_by_label("Grid Table 4 – Blue").last().unwrap().rect().center();
        h.hover_at(tile);
        h.step();
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton { pos: tile, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() });
            h.step();
        }
        h.run_steps(3);
        let state = h.state();
        let (tp, _, _) = state.session.sel.focus.path.cell().unwrap();
        let style = state.session.doc.table(state.session.sel.focus.story, &tp).unwrap().props.style.clone();
        assert_eq!(style.as_deref(), Some("GridTable4AccentBlue"));
        // Clear applies the plain Normal Table style.
        h.state_mut().run("table.style", json!({"style": "TableNormal"})).expect("Clear");
        let state = h.state();
        let (tp, _, _) = state.session.sel.focus.path.cell().unwrap();
        let style = state.session.doc.table(state.session.sel.focus.story, &tp).unwrap().props.style.clone();
        assert_eq!(style.as_deref(), Some("TableNormal"));
    }

    /// In Thai, More's tray names View's Show buttons in Thai, in one group.
    #[test]
    fn show_group_rows_share_edges_in_thai() {
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.ui.language = "th".into();
        app.run("ui.tab", json!({"tab": "View"})).unwrap();
        let mut h = egui_kittest::Harness::builder().with_size(vec2(1440.0, 900.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        h.run_steps(6);
        open_more(&mut h, "เพิ่มเติม");
        let rect = |label: &str| lowest(&h, label);
        let (ruler, grid, boundaries, white) = (rect("ไม้บรรทัด"), rect("เส้นตาราง"), rect("ขอบเขตข้อความ"), rect("ซ่อนพื้นที่ว่าง"));
        for r in [grid, boundaries, white] {
            assert!(r.min.y > 100.0 && (r.min.y - ruler.min.y).abs() < 200.0, "the Show group's tiles together: {ruler:?} {r:?}");
        }
    }

    /// Home's line has Thai Distribution right after Justify (and it sets that alignment, which lays
    /// Thai out with its extra space shared between characters).
    #[test]
    fn home_tab_has_a_thai_distribution_button_after_justify() {
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.run("text.insert", json!({"text": "สวัสดีครับ"})).unwrap();
        let mut h = egui_kittest::Harness::builder().with_size(vec2(1440.0, 900.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        h.run_steps(6);
        let justify = highest(&h, "Justify");
        let thai = highest(&h, "Thai Distribution");
        assert!(thai.min.x > justify.max.x - 1.0 && (thai.center().y - justify.center().y).abs() < 1.0, "{justify:?} {thai:?}");
        click_at(&mut h, thai.center());
        h.run_steps(4);
        let align = h.state().session.doc.para_at(&wordcraft_doc::Pos::body(0, 0)).and_then(|p| p.props.align);
        assert_eq!(align, Some(wordcraft_doc::Align::ThaiDistribute));
    }

    /// A primary-button drag from `a` to `b`, a frame per step.
    fn drag(h: &mut egui_kittest::Harness<WordApp>, a: egui::Pos2, b: egui::Pos2) {
        let button = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        h.hover_at(a);
        h.step();
        h.event(button(a, true));
        h.step();
        for i in 1..=10 {
            h.hover_at(a + (b - a) * (i as f32 / 10.0));
            h.step();
        }
        h.event(button(b, false));
        h.run_steps(4);
    }

    fn line_window() -> egui_kittest::Harness<'static, WordApp> {
        let app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        let mut h = egui_kittest::Harness::builder().with_size(vec2(1440.0, 900.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        h.run_steps(6);
        h
    }

    /// Owner, 2026-11-24: More's tray lists every button of the tab; a tile dragged onto the line
    /// goes where it is dropped, and the arrangement is the tab's own.
    #[test]
    fn a_tile_dragged_onto_the_line_goes_where_it_is_dropped() {
        let mut h = line_window();
        assert!(h.state().ui.ribbon_line.is_empty(), "the tab's own line to start with");
        open_more(&mut h, "More");
        let cut = lowest(&h, "Cut");
        assert!(cut.min.y > 100.0, "Cut is in the tray: {cut:?}");
        let (bold, italic) = (highest(&h, "Bold"), highest(&h, "Italic"));
        let drop = pos2((bold.max.x + italic.min.x) / 2.0, bold.center().y);
        drag(&mut h, cut.center(), drop);
        let order = h.state().ui.ribbon_line.get("Home").cloned().expect("Home is arranged now");
        let at = |k: &str| order.iter().position(|x| x == k);
        assert_eq!(at("edit.cut"), at("format.bold").map(|i| i + 1), "between Bold and Italic: {order:?}");
        let on_line = highest(&h, "Cut");
        assert!(on_line.min.y < 100.0 && on_line.min.x > bold.min.x, "on the line now: {on_line:?}");
        assert!(!h.state().ui.ribbon_line.contains_key("Insert"), "the other tabs keep theirs");
    }

    /// With the tray open, a button dragged off the line comes off it; Reset brings the tab's own
    /// line back. A click on the line with the tray closed never moves anything.
    #[test]
    fn a_button_dragged_off_the_line_comes_off_and_reset_brings_it_back() {
        use egui_kittest::kittest::Queryable;
        let mut h = line_window();
        let bold = highest(&h, "Bold");
        drag(&mut h, bold.center(), bold.center() + vec2(0.0, 400.0));
        assert!(h.state().ui.ribbon_line.is_empty(), "the tray is closed: the line stays as it is");
        open_more(&mut h, "More");
        let bold = highest(&h, "Bold");
        drag(&mut h, bold.center(), bold.center() + vec2(0.0, 400.0));
        let order = h.state().ui.ribbon_line.get("Home").cloned().unwrap_or_default();
        assert!(!order.is_empty() && !order.iter().any(|k| k == "format.bold"), "Bold is off the line: {order:?}");
        assert!(highest(&h, "Bold").min.y > 100.0, "only its tile is left, in the tray");
        h.get_by_label("Reset").click();
        h.run_steps(4);
        assert!(h.state().ui.ribbon_line.is_empty());
        assert!(highest(&h, "Bold").min.y < 100.0, "back on the line");
    }

    /// A tile's + adds its button at the line's end and − takes it off (also from the keyboard), and
    /// the arrangement is kept with the other settings.
    #[test]
    fn the_tile_menu_arranges_the_line_and_it_is_kept() {
        use egui_kittest::kittest::Queryable;
        let mut h = line_window();
        open_more(&mut h, "More");
        let cut = lowest(&h, "Cut");
        assert!(cut.min.y > 100.0);
        // The tile's + adds it at the line's end.
        let add =
            h.get_all_by_label("Add to the line").map(|n| n.rect()).find(|r| (r.center().y - cut.center().y).abs() < 12.0 && r.min.x > cut.min.x);
        click_at(&mut h, add.expect("Cut's tile has +").center());
        h.run_steps(3);
        let order = h.state().ui.ribbon_line.get("Home").cloned().unwrap();
        assert_eq!(order.last().map(String::as_str), Some("edit.cut"), "added at the end: {order:?}");
        let saved = serde_json::to_string(&h.state().prefs()).unwrap();
        let mut b = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        b.apply_prefs(serde_json::from_str(&saved).unwrap());
        assert_eq!(b.ui.ribbon_line.get("Home"), Some(&order), "kept between runs");
    }

    /// The Outlining tab fits the one-line ribbon: each control is drawn once on the line (the
    /// check boxes and the Show Level label used to be drawn in every pass, on top of each other),
    /// and More's tray doesn't repeat the names those controls show themselves.
    #[test]
    fn outlining_draws_each_control_once_on_the_line() {
        use egui_kittest::kittest::Queryable;
        let mut h = line_window();
        h.state_mut().run("view.outline", json!({})).unwrap();
        h.run_steps(8);
        assert_eq!(h.state().ui.tab, "Outlining");
        let on_line = |h: &egui_kittest::Harness<WordApp>, label: &str| h.get_all_by_label(label).filter(|n| n.rect().min.y < 100.0).count();
        for label in ["Show Text Formatting", "Show First Line Only", "Promote", "Move Up"] {
            assert_eq!(on_line(&h, label), 1, "{label} once on the line");
        }
        open_more(&mut h, "More");
        let tray = |h: &egui_kittest::Harness<WordApp>, label: &str| h.get_all_by_label(label).filter(|n| n.rect().min.y > 100.0).count();
        assert_eq!(tray(&h, "Show First Line Only"), 1, "its tile shows the check box's own name only");
        assert_eq!(tray(&h, "Promote"), 2, "a button's tile: the button, then its name");
    }

    /// Owner, 2026-11-25: More's tray on Review showed no Protect or Ink group. They were there,
    /// below a fixed 560 pt scroll height whose bar only showed on hover. The tray now takes the
    /// window's height and its last group shows in a 900 pt window.
    #[test]
    fn the_review_tray_shows_its_last_groups() {
        use egui_kittest::kittest::Queryable;
        let mut h = line_window();
        h.state_mut().ui.tab = "Review".into();
        h.run_steps(6);
        open_more(&mut h, "More");
        h.run_steps(8);
        let reset = h.get_by_label("Reset").rect();
        let ink = lowest(&h, "Hide Ink");
        assert!(ink.max.y < reset.min.y, "Hide Ink {ink:?} shows above the tray's footer {reset:?}");
        assert!(h.get_all_by_label("Restrict Editing").any(|n| n.rect().max.y < reset.min.y));
    }

    /// The style set gallery fills exactly its own box and the next control starts after a gap,
    /// so Colors and Fonts can't overlap the thumbnails (#638).
    #[test]
    fn design_style_set_gallery_stays_in_its_box() {
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        let mut seen = None;
        let mut h = egui_kittest::Harness::builder().with_size(vec2(1440.0, 200.0)).build_ui(|ui| {
            ui.horizontal_top(|ui| {
                let (gallery, used) = style_set_box(&mut app, ui);
                seen = Some((gallery, used, ui.cursor().min.x));
            });
        });
        h.run_steps(2);
        drop(h);
        let (gallery, used, next_x) = seen.unwrap();
        assert!((gallery.width() - crate::previews::STYLE_SET_GALLERY_W).abs() < 0.01);
        assert!(used.right() <= gallery.right() + 0.01, "{used:?} {gallery:?}");
        assert!(next_x >= gallery.right() + 8.0 - 0.01, "{next_x} {gallery:?}");
    }

    /// #658: menu buttons grey out (and don't open) while the command their menu is about is
    /// disabled: the Table Layout menus outside a table; menus with no command stay on.
    #[test]
    fn menu_buttons_follow_their_command() {
        use crate::widgets::menu_enabled;
        let mut a = crate::WordApp::new(Session::new(wordcraft_doc::Document::from_text("Text")), crate::Services::default());
        for id in ["table.selectTable", "table.deleteTable", "table.autofit"] {
            assert!(!menu_enabled(&a, id), "{id} is off outside a table");
        }
        assert!(menu_enabled(&a, ""), "a menu without a command is always on");
        assert!(menu_enabled(&a, "insert.header"));
        a.run("insert.table", json!({"rows": 2, "cols": 2})).unwrap();
        for id in ["table.selectTable", "table.deleteTable", "table.autofit"] {
            assert!(menu_enabled(&a, id), "{id} is on inside a table");
        }
    }

    /// Every menu picture (Layout's Text Direction, Orientation, Columns, Breaks, Line Numbers and
    /// Margins; the table menus' Borders, Select, Delete and AutoFit) draws something: a misspelt
    /// name would leave an empty space before the label.
    #[test]
    fn layout_menu_pictures_all_draw() {
        let names = [
            "dirHorizontal",
            "dirDown",
            "dirUp",
            "portrait",
            "landscape",
            "cols_one",
            "cols_two",
            "cols_three",
            "cols_left",
            "cols_right",
            "colsMore",
            "brk_page",
            "brk_column",
            "brk_textWrapping",
            "brk_nextPage",
            "brk_continuous",
            "brk_evenPage",
            "brk_oddPage",
            "ln_none",
            "ln_continuous",
            "ln_restartPage",
            "ln_restartSection",
            "lnOptions",
            "mg_normal",
            "mg_narrow",
            "mg_moderate",
            "mg_wide",
            "mg_mirrored",
            "mg_office2003",
            "mg_custom",
            "tb_all",
            "tb_outside",
            "tb_inside",
            "tb_none",
            "tb_top",
            "tb_bottom",
            "tb_left",
            "tb_right",
            "tb_insideH",
            "tb_insideV",
            "tb_diagDown",
            "tb_diagUp",
            "hline",
            "gridlines",
            "sel_cell",
            "sel_column",
            "sel_row",
            "sel_table",
            "del_cell",
            "del_column",
            "del_row",
            "del_table",
            "fit_contents",
            "fit_window",
            "fit_fixed",
        ];
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        ctx.run_ui(egui::RawInput::default(), |_| {}).drop_without_applying_deltas();
        for name in names {
            let out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let r = Rect::from_min_size(pos2(10.0, 10.0), vec2(24.0, 20.0));
                layout_picture(ui.painter(), r, name, egui::Color32::BLACK, egui::Color32::BLUE);
            });
            assert!(out.shapes.len() >= 2, "{name} draws its picture");
            out.drop_without_applying_deltas();
        }
    }

    /// Insert › Shapes shows each shape's picture beside its name; choosing one inserts it.
    #[test]
    fn shapes_menu_items_insert_their_shape() {
        use egui_kittest::kittest::Queryable;
        let mut a = crate::WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        a.run("ui.tab", json!({"tab": "Insert"})).unwrap();
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
            |ui, app: &mut crate::WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            a,
        );
        h.run_steps(4);
        h.get_by_label("Shapes").click();
        h.run_steps(3);
        h.get_by_label("Oval").click();
        h.run_steps(3);
        assert!(has_shape_selected(&h.state().session), "choosing Oval inserted a shape and selected it");
    }

    /// #1206: Margins, Orientation, Size and Columns mark the caret section's setting, as Word
    /// does. Every value differs from a new document's, so a default can't satisfy it.
    #[test]
    fn page_setup_menus_mark_the_current_setting() {
        use egui_kittest::kittest::{By, NodeT, Queryable};
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::from_text("Text")), crate::Services::default());
        app.run("layout.margins", json!({"preset": "narrow"})).unwrap();
        app.run("layout.orientation", json!({"value": "landscape"})).unwrap();
        app.run("layout.size", json!({"name": "A4"})).unwrap();
        app.run("layout.columns", json!({"preset": "right"})).unwrap();
        let mut h = egui_kittest::Harness::builder().with_size(vec2(400.0, 1200.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                margins_items(ui, app);
                orientation_items(ui, app);
                size_menu(ui, app);
                columns_items(ui, app);
            },
            app,
        );
        h.run_steps(2);
        let checked: Vec<String> = h
            .query_all(By::new().predicate(|n| n.toggled() == Some(egui::accesskit::Toggled::True)))
            .filter_map(|n| n.accesskit_node().label())
            .collect();
        let first_words: Vec<&str> = checked.iter().filter_map(|l| l.trim_start_matches('✓').split_whitespace().next()).collect();
        assert_eq!(first_words, ["Narrow", "Landscape", "A4", "Right"], "{checked:?}");
    }

    /// #860 review: a tag the language list doesn't have shows by language and region, not as
    /// the tag; Portuguese (Angola) isn't Brazil's, and no name shows twice in the menu.
    #[test]
    fn other_regions_show_by_name_once() {
        assert_eq!(language_name(Some("fr-CA")), "French (Canada)");
        assert_eq!(language_name(Some("en-AU")), "English (Australia)");
        assert_eq!(language_name(Some("de_AT")), "German (Austria)");
        assert_eq!(language_name(Some("pt-AO")), "Portuguese (Angola)");
        assert_eq!(language_name(Some("sr-Latn-BA")), "Serbian (Bosnia and Herzegovina)");
        assert_eq!(language_name(Some("en-ZZ")), "English (ZZ)");
        assert_eq!(language_name(Some("xx-YY")), "xx-YY");
        let exact = |cur: &'static str| move |t: &str| t.eq_ignore_ascii_case(cur);
        for tag in ["fr-CA", "pt-AO", "en-AU"] {
            let c = language_checks(tag, exact(tag));
            let name = c.extra.expect("shown above the list");
            assert!(!PROOFING_LANGUAGES.iter().any(|(_, n)| *n == name), "{name} is listed once");
            assert!(!c.listed.contains(&true), "{tag}: nothing in the list checked");
        }
        // A tag named like a list entry checks that entry instead of showing it again.
        let c = language_checks("PT-br-x-test", |_| false);
        assert_eq!(c.extra, None);
        assert_eq!(c.listed, PROOFING_LANGUAGES.map(|(t, _)| t == "pt-BR"));
    }

    #[test]
    fn czech_has_a_name_in_the_language_menu() {
        assert!(PROOFING_LANGUAGES.contains(&("cs-CZ", "Czech")));
        for tag in ["cs", "cs-CZ", "CS-cz", "ces", "CZE"] {
            assert_eq!(language_name(Some(tag)), "Czech", "{tag}");
        }
    }

    #[test]
    fn portuguese_has_a_name_in_the_language_menu() {
        assert!(PROOFING_LANGUAGES.iter().any(|(t, _)| *t == "pt-BR"));
        assert_eq!(language_name(Some("pt-BR")), "Portuguese (Brazil)");
        assert_eq!(language_name(Some("pt")), "Portuguese (Brazil)");
        assert_eq!(language_name(Some("PT-BR".to_string().as_str())), "Portuguese (Brazil)");
    }

    #[test]
    fn turkish_has_a_name_in_the_language_menu() {
        assert!(PROOFING_LANGUAGES.iter().any(|(t, _)| *t == "tr-TR"));
        assert_eq!(language_name(Some("tr-TR")), "Turkish");
        assert_eq!(language_name(Some("tr")), "Turkish");
        assert_eq!(language_name(Some("TR-TR")), "Turkish");
    }

    #[test]
    fn a_picture_selection_follows_the_picture() {
        let mut s = Session::new(wordcraft_doc::Document::new());
        assert!(!has_picture_selected(&s));
        assert!(!contextual_tabs(&s).contains(&"Picture Format"));
        s.run("insert.picture", &json!({"data": png_data()})).unwrap();
        // Its commands are in the Picture pane (`object_pane`), not a tab.
        assert!(has_picture_selected(&s) && !contextual_tabs(&s).contains(&"Picture Format"));
        s.run("select.collapse", &json!({"end": true})).unwrap();
        s.run("text.insert", &json!({"text": "x"})).unwrap();
        assert!(!has_picture_selected(&s));
    }

    /// The width and height boxes of a picture follow File › Options › Advanced › Measurement
    /// units (not a fixed `pt`), as the table's do; a typed length in another unit converts.
    #[test]
    fn picture_size_boxes_show_the_options_unit() {
        use egui_kittest::kittest::Queryable;
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.session.run("insert.picture", &json!({"data": png_data()})).unwrap();
        app.session.prefs.units = Some(wordcraft_geom::Unit::Centimeters);
        crate::options::sync(&app);
        app.run("ui.tab", json!({"tab": "Picture Format"})).unwrap();
        let mut h = egui_kittest::Harness::builder().with_size(vec2(1440.0, 900.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        h.run_steps(4);
        let values: Vec<String> = h.get_all_by_role(egui::accesskit::Role::TextInput).filter_map(|n| n.value()).collect();
        assert!(values.iter().any(|v| v.ends_with("cm")), "a size box in cm: {values:?}");
        assert!(!values.iter().any(|v| v.ends_with("pt")), "no size box in points: {values:?}");
    }

    #[test]
    fn a_shape_selection_follows_the_shape() {
        // Its commands are in the Shape pane (`object_pane`), shown while `has_shape_selected`.
        let mut s = Session::new(wordcraft_doc::Document::new());
        s.run("insert.shape", &json!({"kind": "rectangle"})).unwrap();
        assert!(has_shape_selected(&s));
        assert!(!contextual_tabs(&s).contains(&"Shape Format") && !contextual_tabs(&s).contains(&"Picture Format"));
        s.run("select.collapse", &json!({"end": true})).unwrap();
        s.run("text.insert", &json!({"text": "x"})).unwrap();
        assert!(!has_shape_selected(&s));
    }

    /// Groups have no launchers (owner, 2026-11-13); every dialog or pane they opened is still
    /// in the ribbon, at the end of a related menu or as a button of its group.
    #[test]
    fn launcher_dialogs_have_other_ways_in() {
        // The ribbon's own code, without these tests.
        let src = include_str!("ribbon.rs").split("#[cfg(test)]").next().unwrap();
        for id in ["edit.clipboardPane", "references.noteOptions", "review.trackingOptions", "\"pageSetup\"", "para.dialog"] {
            assert!(src.contains(id), "{id} is not reachable from the ribbon");
        }
        let gallery = include_str!("previews.rs");
        assert!(gallery.contains("view.stylesPane"), "the Styles gallery's More button opens the Styles pane");
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.run("layout.pageSetup", json!({})).unwrap();
    }

    #[test]
    fn smart_art_picker_inserts_and_design_tab_follows_the_selection() {
        let mut app = crate::WordApp::new(Session::new(wordcraft_doc::Document::new()), Default::default());
        // Insert › SmartArt without a layout opens the picker.
        let _ = app.run("insert.smartArt", json!({}));
        assert_eq!(app.dialog.as_ref().map(crate::dialogs::Dialog::name), Some("insertSmartArt"));
        app.dialog = None;
        app.run("insert.smartArt", json!({"layout": "cycle"})).unwrap();
        assert!(has_smart_art_selected(&app.session) && !contextual_tabs(&app.session).contains(&"SmartArt Design"));
        app.run("smartArt.textPane", json!({})).unwrap();
        assert!(app.session.view.smart_art_pane);
        app.session.run("select.collapse", &json!({"end": true})).unwrap();
        app.session.run("text.insert", &json!({"text": "x"})).unwrap();
        assert!(!has_smart_art_selected(&app.session));
    }

    #[test]
    fn a_chart_selection_follows_the_chart_and_its_commands_open_dialogs() {
        let mut app = crate::WordApp::new(Session::new(wordcraft_doc::Document::new()), Default::default());
        // Insert › Chart without a type opens the gallery; choosing inserts.
        let _ = app.run("insert.chart", json!({}));
        assert_eq!(app.dialog.as_ref().map(crate::dialogs::Dialog::name), Some("insertChart"));
        app.dialog = None;
        app.run("insert.chart", json!({"type": "doughnut"})).unwrap();
        assert!(has_chart_selected(&app.session) && !contextual_tabs(&app.session).contains(&"Chart Design"));
        let _ = app.run("chart.editData", json!({}));
        assert_eq!(app.dialog.as_ref().map(crate::dialogs::Dialog::name), Some("chartData"));
        let _ = app.run("chart.type", json!({}));
        assert_eq!(app.dialog.as_ref().map(crate::dialogs::Dialog::name), Some("changeChartType"));
        app.session.run("select.collapse", &json!({"end": true})).unwrap();
        app.session.run("text.insert", &json!({"text": "x"})).unwrap();
        assert!(!has_chart_selected(&app.session));
    }

    /// No group's content runs into its label: View › Show stacked four check boxes in a column
    /// made for three, so "Hide White Space" was drawn under "Show". Every tab, every language.
    #[test]
    fn ribbon_groups_fit_above_their_labels() {
        let app = WordApp::new(Session::new(wordcraft_engine::sample::sample_document()), crate::Services::default());
        // Wide enough that no tab hides groups behind its scroll arrows.
        let mut h = egui_kittest::Harness::builder().with_size(vec2(2560.0, 900.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        let mut tall = Vec::new();
        for lang in crate::i18n::Lang::all() {
            h.state_mut().run("ui.language", json!({"value": lang.code()})).unwrap();
            for tab in TABS.iter().filter(|t| **t != "File") {
                h.state_mut().run("ui.tab", json!({"tab": tab})).unwrap();
                h.run_steps(2);
                crate::widgets::TALL_GROUPS.with(|g| g.borrow_mut().clear());
                h.run_steps(1);
                let found = crate::widgets::TALL_GROUPS.with(|g| g.take());
                tall.extend(found.into_iter().map(|g| format!("{} {tab} › {g}", lang.code())));
            }
        }
        tall.sort();
        tall.dedup();
        assert!(tall.is_empty(), "groups taller than the ribbon: {tall:#?}");
    }

    /// The tabs that appear for an object (a table, picture, shape, chart, SmartArt, equation)
    /// fit the two-row ribbon too, in every language.
    #[test]
    fn contextual_groups_fit_above_their_labels() {
        // Tables, pictures, shapes, charts and SmartArt have panes instead (`table_pane`, `object_pane`).
        let cases: [(&str, Value, &[&str]); 1] = [("insert.equation", json!({}), &["Equation"])];
        let mut tall = Vec::new();
        for (cmd, params, tabs) in cases {
            let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
            app.run(cmd, params).unwrap();
            let mut h = egui_kittest::Harness::builder().with_size(vec2(2560.0, 900.0)).build_ui_state(
                |ui, app: &mut WordApp| {
                    let ctx = ui.ctx().clone();
                    app.logic(&ctx);
                    app.ui(ui);
                },
                app,
            );
            for lang in crate::i18n::Lang::all() {
                h.state_mut().run("ui.language", json!({"value": lang.code()})).unwrap();
                for tab in tabs.iter() {
                    assert!(contextual_tabs(&h.state().session).contains(tab), "{cmd} shows {tab}");
                    h.state_mut().run("ui.tab", json!({"tab": tab})).unwrap();
                    h.run_steps(2);
                    crate::widgets::TALL_GROUPS.with(|g| g.borrow_mut().clear());
                    h.run_steps(1);
                    tall.extend(crate::widgets::TALL_GROUPS.with(|g| g.take()).into_iter().map(|g| format!("{} {tab} › {g}", lang.code())));
                }
            }
        }
        tall.sort();
        tall.dedup();
        assert!(tall.is_empty(), "groups taller than the ribbon: {tall:#?}");
    }

    /// Going into a header opens the Header & Footer pane (owner, 2026-11-19; a contextual tab
    /// before) and leaves the ribbon on its tab; Close Header and Footer, in the pane, ends
    /// editing and the pane goes.
    #[test]
    fn the_header_footer_pane_comes_and_goes_with_the_header() {
        use egui_kittest::kittest::Queryable;
        let app = WordApp::new(Session::new(wordcraft_doc::Document::from_text("Text")), crate::Services::default());
        let mut h = egui_kittest::Harness::builder().with_size(vec2(1440.0, 900.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        h.state_mut().run("ui.tab", json!({"tab": "Review"})).unwrap();
        h.run_steps(2);
        h.state_mut().run("insert.editHeader", json!({})).unwrap();
        h.run_steps(3);
        assert!(!contextual_tabs(&h.state().session).contains(&"Header & Footer"), "no tab any more");
        assert_eq!(h.state().ui.tab, "Review", "the ribbon stays on its tab");
        assert_eq!(crate::panes::right_active(&h.ctx), Some("headerFooter"));
        // Everything inside the shared width: a row too wide pushes the pane past the window's
        // edge and under the page, where clicks don't reach it.
        let dock = egui::PanelState::load(&h.ctx, egui::Id::new("right_dock")).map(|p| p.outer_rect).unwrap();
        assert!(dock.right() <= 1440.5, "the pane stays in the window: {dock:?}");
        for label in
            ["Go to Header", "Go to Footer", "Previous Section", "Next Section", "Link to Previous", "Different First Page", "Header from Top"]
        {
            let r = h.get_by_label(label).rect();
            assert!(r.left() >= dock.left() && r.right() <= dock.right() + 0.5, "{label} at {r:?} is outside the pane {dock:?}");
        }
        // Even grids (owner, 2026-11-22): the three menus, Insert's three tiles, Navigation's two
        // columns, and Position's two boxes each share one width and one row.
        for row in [
            &["Header", "Footer", "Page Number"][..],
            &["Date & Time", "Field", "Picture from File"],
            &["Go to Header", "Go to Footer"],
            &["Previous Section", "Next Section"],
            &["Header from Top", "Footer from Bottom"],
        ] {
            let r: Vec<egui::Rect> = row.iter().map(|l| h.get_by_label(l).rect()).collect();
            for x in &r {
                assert!((x.width() - r[0].width()).abs() < 0.5 && (x.top() - r[0].top()).abs() < 0.5, "{row:?} are one even row");
            }
        }
        let (a, b) = (h.get_by_label("Go to Header").rect(), h.get_by_label("Previous Section").rect());
        assert!((a.left() - b.left()).abs() < 0.5 && (a.width() - b.width()).abs() < 0.5, "Navigation's columns line up");
        // A click on an option reaches it.
        h.get_by_label("Different First Page").click();
        h.run_steps(3);
        assert!(wordcraft_engine::cmd::hf::state(&mut h.state_mut().session).different_first, "the click ticks the box");
        // At the pane's end: scrolled into view first.
        h.get_by_label("Close Header and Footer").scroll_to_me();
        h.run_steps(10);
        h.get_by_label("Close Header and Footer").click();
        h.run_steps(3);
        assert!(!h.state().session.in_header_footer(), "Close ends editing");
        assert_ne!(crate::panes::right_active(&h.ctx), Some("headerFooter"), "and the pane goes");
    }

    /// #860: Review › Language (and the status bar) reach every proofing language, not just the
    /// short list: Set Proofing Language… opens the Language dialog (French, Set As Default),
    /// Language Preferences… opens Options › Language, and French text shows by name, checked.
    #[test]
    fn language_menu_reaches_every_language() {
        use egui_kittest::kittest::{By, NodeT, Queryable};
        let item = |label: &'static str| By::new().label_contains(label);
        let app = WordApp::new(Session::new(wordcraft_doc::Document::from_text("Bonjour")), crate::Services::default());
        let mut h =
            egui_kittest::Harness::builder().with_size(vec2(400.0, 400.0)).build_ui_state(|ui, app: &mut WordApp| language_menu(ui, app), app);
        h.run();
        assert!(h.query(item("French (France)")).is_none(), "the short list only, on an English document");
        assert!(!h.get(item("Set Proofing Language…")).accesskit_node().is_disabled(), "never greyed out on a fresh document");
        assert!(!h.get(item("English (United States)")).accesskit_node().is_disabled());
        h.get(item("Set Proofing Language…")).click();
        h.run();
        assert_eq!(h.state().dialog.as_ref().map(crate::dialogs::Dialog::name), Some("language"));
        h.state_mut().dialog = None;
        // French for the whole document: the menu and the status bar name it.
        h.state_mut().run("review.language", json!({"lang": "fr-FR", "default": true})).unwrap();
        h.run();
        assert_eq!(caret_language(h.state()), "French (France)");
        assert!(h.query(item("French (France)")).is_some(), "the current language is listed (only while it is the current one)");
        h.get(item("Language Preferences…")).click();
        h.run();
        assert!(h.state().ui.backstage, "File › Options opened");
        assert_eq!(h.state().ui.options_pane, "language");
        assert_eq!(language_name(Some("de-DE")), "German (Germany)");
    }

    #[test]
    fn resolve_tab_falls_back_when_contextual_tab_expires() {
        let all = ["Home", "Table Design", "Picture Format"];
        assert_eq!(resolve_tab("Picture Format", &all), "Picture Format");
        assert_eq!(resolve_tab("Picture Format", &["Home"]), "Home");
        assert_eq!(resolve_tab("Table Design", &["Home"]), "Home");
        assert_eq!(resolve_tab("Home", &["Home"]), "Home");
    }
}
