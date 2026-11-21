//! The Character dialog (owner, 2026-11-16; LibreOffice's Character dialog as the reference,
//! drawn our own way): right-click › Character… and the Font group's dialog. Five pages in a
//! list on the left: Font (Western and Thai text side by side), Font Effects, Position,
//! Background and Border, a preview of the selected text, and Reset, Cancel and OK. No East
//! Asian page or font (KrubOffice doesn't plan to support East Asian languages). OK is one
//! formatting step that sets only what was changed, so formatting the dialog can't show, or the
//! user left alone, stays as it is.

use egui::{Ui, vec2};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::WordApp;
use crate::dialogs_style::{LINES, UNDERLINES, heading};
use crate::theme::Tokens;
use crate::widgets::SteadyUi;

/// The pages, in order: (id, name, icon).
pub const PAGES: [(&str, &str, &str); 5] = [
    ("font", "Font", "fonts"),
    ("effects", "Font Effects", "effects"),
    ("position", "Position", "superscript"),
    ("background", "Background", "shading"),
    ("border", "Border", "borders"),
];

/// Styles of a font column: (bold, italic, label).
const STYLES: [(bool, bool, &str); 4] = [(false, false, "Regular"), (true, false, "Bold"), (false, true, "Italic"), (true, true, "Bold Italic")];

/// Languages for Thai (complex-script) text: (BCP 47 tag, English name).
const COMPLEX_LANGUAGES: [(&str, &str); 3] = [("th-TH", "Thai"), ("ar-SA", "Arabic"), ("he-IL", "Hebrew")];

/// Border widths offered, points.
const WIDTHS: [f32; 6] = [0.25, 0.5, 0.75, 1.0, 1.5, 2.25];

/// The dialog's fields. Text fields are empty, and check boxes off, when the selection's values
/// differ; OK sends only the fields changed since the dialog opened (`start`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CharForm {
    pub page: String,
    // Font: Western (Latin) text, then Thai (complex-script) text.
    pub font: String,
    pub size: String,
    pub bold: bool,
    pub italic: bool,
    pub lang: String,
    pub font_cs: String,
    pub size_cs: String,
    pub bold_cs: bool,
    pub italic_cs: bool,
    pub lang_cs: String,
    // Font Effects.
    /// `RRGGBB` or `auto`.
    pub color: String,
    /// `Underline`'s name (`none`, `single`…).
    pub underline: String,
    /// `RRGGBB` or `auto`.
    pub underline_color: String,
    /// 0 none, 1 single, 2 double.
    pub strike: u8,
    pub caps: bool,
    pub small_caps: bool,
    pub hidden: bool,
    pub outline: bool,
    pub shadow: bool,
    pub emboss: bool,
    pub engrave: bool,
    // Position.
    /// 0 normal, 1 superscript, 2 subscript.
    pub vert: u8,
    /// Raised (+) or lowered (−), points.
    pub raise: f32,
    /// Width scale, percent.
    pub scale: f32,
    /// Character spacing, points.
    pub spacing: f32,
    pub kern: bool,
    /// Kern from this font size up, points.
    pub kern_at: f32,
    // Background: `RRGGBB`, or empty for none.
    pub shading: String,
    // Border: on, and its line.
    pub border: bool,
    pub border_style: String,
    pub border_width: f32,
    /// `RRGGBB` or `auto`.
    pub border_color: String,
    /// The fields as the dialog opened (for OK and Reset).
    #[serde(skip)]
    pub start: Option<Box<CharForm>>,
}

impl CharForm {
    /// The form for the formatting at the selection (`format.state`), with fonts as the font
    /// lists name them.
    pub fn from_state(app: &mut WordApp, st: &Value) -> CharForm {
        let s = |k: &str| st.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let b = |k: &str| st.get(k).and_then(Value::as_bool).unwrap_or(false);
        let n = |k: &str| st.get(k).and_then(Value::as_f64).map(|v| v as f32);
        let num = |k: &str| n(k).map(trim_num).unwrap_or_default();
        let border = st.get("borderLine").filter(|v| !v.is_null()).cloned();
        let border_style = border.as_ref().and_then(|b| b.get("style")).and_then(Value::as_str).unwrap_or("none").to_string();
        let border_on = border_style != "none" && st.get("border").and_then(Value::as_bool).unwrap_or(false);
        let border_color = border
            .as_ref()
            .and_then(|b| b.get("color"))
            .and_then(|c| serde_json::from_value::<wordcraft_doc::Rgb>(c.clone()).ok())
            .map_or("auto".to_string(), |c| c.hex());
        let vert = if b("superscript") {
            1
        } else if b("subscript") {
            2
        } else {
            0
        };
        let strike = if b("doubleStrike") {
            2
        } else if b("strike") {
            1
        } else {
            0
        };
        let kern = n("kern").unwrap_or(0.0);
        let mut f = CharForm {
            page: "font".into(),
            font: app.previews.font_shown(&s("font")),
            size: num("size"),
            bold: b("bold"),
            italic: b("italic"),
            lang: s("lang"),
            font_cs: app.previews.font_shown(&s("fontCs")),
            size_cs: num("sizeCs"),
            bold_cs: b("boldCs"),
            italic_cs: b("italicCs"),
            // Thai text without a language set shows Thai (owner, 2026-11-16); OK stores it only if
            // another language is chosen.
            lang_cs: Some(s("langBidi")).filter(|l| !l.is_empty()).unwrap_or_else(|| "th-TH".into()),
            color: s("color"),
            underline: st.get("underlineStyle").and_then(Value::as_str).unwrap_or("").to_string(),
            underline_color: st.get("underlineColor").and_then(Value::as_str).map_or("auto".to_string(), str::to_string),
            strike,
            caps: b("caps"),
            small_caps: b("smallCaps"),
            hidden: b("hidden"),
            outline: b("outline"),
            shadow: b("shadow"),
            emboss: b("emboss"),
            engrave: b("engrave"),
            vert,
            raise: n("position").unwrap_or(0.0),
            scale: n("scale").unwrap_or(100.0),
            spacing: n("spacing").unwrap_or(0.0),
            kern: kern > 0.0,
            kern_at: if kern > 0.0 { kern } else { 14.0 },
            shading: s("shading"),
            border: border_on,
            border_style: if border_on { border_style } else { "single".into() },
            border_width: border.as_ref().and_then(|b| b.get("width")).and_then(Value::as_f64).map_or(0.5, |w| w as f32),
            border_color,
            start: None,
        };
        f.start = Some(Box::new(f.clone()));
        f
    }

    /// The character properties the fields stand for (fonts as the lists name them).
    fn props(&self) -> serde_json::Map<String, Value> {
        let mut p = serde_json::Map::new();
        let mut put = |k: &str, v: Value| {
            p.insert(k.into(), v);
        };
        if !self.font.trim().is_empty() {
            put("font", json!(self.font.trim()));
        }
        if !self.font_cs.trim().is_empty() {
            put("fontCs", json!(self.font_cs.trim()));
        }
        if let Ok(v) = self.size.trim().parse::<f32>() {
            put("size", json!(v.clamp(1.0, 1638.0)));
        }
        if let Ok(v) = self.size_cs.trim().parse::<f32>() {
            put("sizeCs", json!(v.clamp(1.0, 1638.0)));
        }
        put("bold", json!(self.bold));
        put("italic", json!(self.italic));
        put("boldCs", json!(self.bold_cs));
        put("italicCs", json!(self.italic_cs));
        if !self.lang.is_empty() {
            put("lang", json!(self.lang));
        }
        if !self.lang_cs.is_empty() {
            put("langBidi", json!(self.lang_cs));
        }
        if let Some(c) = wordcraft_doc::Rgb::parse(&self.color) {
            put("color", json!({"Rgb": [c.0, c.1, c.2]}));
        } else if self.color.eq_ignore_ascii_case("auto") {
            put("color", json!("Auto"));
        }
        if !self.underline.is_empty() {
            put("underline", json!(self.underline));
        }
        put("underlineColor", wordcraft_doc::Rgb::parse(&self.underline_color).map_or(Value::Null, |c| json!(c.hex())));
        put("strike", json!(self.strike == 1));
        put("doubleStrike", json!(self.strike == 2));
        put("caps", json!(self.caps));
        put("smallCaps", json!(self.small_caps));
        put("hidden", json!(self.hidden));
        put("outline", json!(self.outline));
        put("shadow", json!(self.shadow));
        put("emboss", json!(self.emboss));
        put("engrave", json!(self.engrave));
        put("vertAlign", json!(["baseline", "superscript", "subscript"][usize::from(self.vert.min(2))]));
        put("position", json!(self.raise));
        put("scale", json!(self.scale));
        put("spacing", json!(self.spacing));
        put("kern", json!(if self.kern { self.kern_at } else { 0.0 }));
        if let Some(c) = wordcraft_doc::Rgb::parse(&self.shading) {
            put("shading", json!(c.hex()));
        }
        if self.border {
            let color = wordcraft_doc::Rgb::parse(&self.border_color).map(|c| c.hex());
            put("border", json!({"style": self.border_style, "width": self.border_width, "color": color, "space": 0.0}));
        }
        p
    }

    /// OK's `format.set` parameters: the properties changed since the dialog opened (fonts by
    /// their stored names), and the settings to remove. `None` when nothing changed.
    pub fn changes(&self, stored: &mut dyn FnMut(&str) -> String) -> Option<Value> {
        let start = self.start.as_deref().map(CharForm::props).unwrap_or_default();
        let mut changed: serde_json::Map<String, Value> = self.props().into_iter().filter(|(k, v)| start.get(k) != Some(v)).collect();
        for k in ["font", "fontCs"] {
            if let Some(f) = changed.get(k).and_then(Value::as_str).map(str::to_string) {
                changed.insert(k.into(), json!(stored(&f)));
            }
        }
        let was = self.start.as_deref();
        let mut clear = Vec::new();
        if self.shading.is_empty() && was.is_some_and(|w| !w.shading.is_empty()) {
            clear.push("shading");
        }
        if !self.border && was.is_some_and(|w| w.border) {
            clear.push("border");
        }
        if changed.is_empty() && clear.is_empty() {
            return None;
        }
        Some(json!({"props": changed, "clear": clear}))
    }
}

/// `12` rather than `12.0`, `10.5` as it is.
fn trim_num(v: f32) -> String {
    if (v - v.round()).abs() < 0.001 { format!("{}", v.round()) } else { format!("{v}") }
}

/// The text the preview shows: the selection (its first line, shortened), else a sample.
pub fn preview_text(app: &WordApp) -> String {
    let text = app.session.selected_text();
    let line = text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    let short: String = line.chars().take(48).collect();
    if short.is_empty() { tl!("AaBbCc ภาษาไทย").to_string() } else { short }
}

/// The Character window. Returns true to close.
pub fn body(
    app: &mut WordApp,
    ui: &mut Ui,
    f: &mut CharForm,
    sample: &str,
    bullet: &mut Option<Box<crate::dialogs_bullet::DefineBulletForm>>,
) -> bool {
    // A fixed area, so the window keeps its size from page to page.
    const PAGE_H: f32 = 330.0;
    ui.allocate_ui_with_layout(vec2(700.0, PAGE_H), egui::Layout::left_to_right(egui::Align::Min), |ui| {
        ui.set_height(PAGE_H);
        ui.vertical(|ui| {
            ui.set_width(150.0);
            for (id, name, icon) in PAGES {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(20.0, 20.0), egui::Sense::hover());
                    let t = Tokens::get(ui.ctx());
                    crate::icons::paint(ui.painter(), r, icon, t.icon, t.accent);
                    if ui.steady_label(f.page == id, tl!(name)).clicked() {
                        f.page = id.to_string();
                    }
                });
            }
        });
        let x = ui.cursor().min.x + 4.0;
        let top = ui.cursor().min.y;
        ui.painter().vline(x, top..=top + PAGE_H, ui.visuals().widgets.noninteractive.bg_stroke);
        ui.add_space(12.0);
        ui.vertical(|ui| {
            ui.set_width(530.0);
            match f.page.as_str() {
                "effects" => effects_page(app, ui, f),
                "position" => position_page(ui, f),
                "background" => background_page(app, ui, f),
                "border" => border_page(app, ui, f),
                _ => font_page(app, ui, f),
            }
        });
    });
    ui.separator();
    preview(ui, f, sample);
    ui.add_space(6.0);
    let mut reset = false;
    let mut out = (false, false);
    ui.horizontal(|ui| {
        reset = ui.button(tl!("Reset")).on_hover_text(tl!("Back to the formatting the text had when this opened")).clicked();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let ok = ui.add(crate::widgets::primary_button(ui, tl!("OK"))).clicked();
            let cancel = ui.button(tl!("Cancel")).clicked();
            out = (ok, cancel);
        });
    });
    if reset && let Some(start) = f.start.clone() {
        let page = f.page.clone();
        *f = (*start).clone();
        f.page = page;
        f.start = Some(start);
    }
    let (ok, cancel) = out;
    if (ok || cancel) && hand_back_bullet(app, f, bullet, ok) {
        return true;
    }
    if ok && let Some(params) = f.changes(&mut |name| app.previews.font_stored(name)) {
        let _ = app.run("format.set", params);
    }
    ok || cancel
}

/// The Character dialog formatting Define New Bullet's bullet (#2786, ported from WordCraft
/// 83f18db1): it opens on the Font page with the bullet's font, size, bold, italic and colour, and
/// carries the form back (see `Dialog::Font::bullet`).
pub fn for_bullet(app: &mut WordApp, form: crate::dialogs_bullet::DefineBulletForm) -> Option<crate::dialogs::Dialog> {
    let mut d = crate::dialogs::Dialog::open("font", app)?;
    if let crate::dialogs::Dialog::Font { form: f, bullet, .. } = &mut d {
        f.page = "font".into();
        if !form.font.is_empty() {
            f.font = app.previews.font_shown(&form.font);
        }
        f.size = form.size.clone();
        f.bold = form.bold;
        f.italic = form.italic;
        f.color = if form.color.is_empty() { "auto".into() } else { form.color.clone() };
        let start = (**f).clone();
        f.start = Some(Box::new(start));
        *bullet = Some(Box::new(form));
    }
    Some(d)
}

/// OK or Cancel while the dialog formats Define New Bullet's bullet (#2786): back to that dialog,
/// with the font (its stored English name, and only when the user changed it, so a bullet in the
/// text's font keeps following the text), size, bold, italic and colour on OK. False when the
/// dialog isn't formatting a bullet.
fn hand_back_bullet(app: &mut WordApp, f: &CharForm, bullet: &mut Option<Box<crate::dialogs_bullet::DefineBulletForm>>, ok: bool) -> bool {
    let Some(mut form) = bullet.take() else { return false };
    if ok {
        let was = f.start.as_ref().map(|s| s.font.trim().to_string()).unwrap_or_default();
        if f.font.trim() != was {
            form.font = app.previews.font_stored(f.font.trim());
        }
        form.size = f.size.trim().to_string();
        form.bold = f.bold;
        form.italic = f.italic;
        form.color = if f.color.eq_ignore_ascii_case("auto") { String::new() } else { f.color.clone() };
    }
    app.dialog = Some(crate::dialogs::Dialog::Insert(Box::new(crate::dialogs_insert::InsertDialog::DefineBullet(*form))));
    true
}

fn font_page(app: &mut WordApp, ui: &mut Ui, f: &mut CharForm) {
    let fams = app.previews.families();
    let prev = app.previews.font_preview_fn();
    let western: Vec<(&str, &str)> =
        crate::dialogs_layout::LANGUAGES.iter().copied().filter(|(t, _)| !["ja", "zh", "ko", "ar", "he"].iter().any(|p| t.starts_with(p))).collect();
    let mut column = |ui: &mut Ui,
                      title: &str,
                      salt: &str,
                      font: &mut String,
                      size: &mut String,
                      bold: &mut bool,
                      italic: &mut bool,
                      lang: &mut String,
                      langs: &[(&str, &str)]| {
        ui.vertical(|ui| {
            ui.set_width(255.0);
            heading(ui, title);
            egui::Grid::new(salt).num_columns(2).spacing(vec2(8.0, 8.0)).show(ui, |ui| {
                ui.label(tl!("Font:"));
                let stored = app.previews.font_stored(font);
                let warning = app.font_dialog_warning(&stored);
                if let Some(pick) =
                    crate::widgets::font_combo_warned(ui, &format!("{salt}_font"), 180.0, font, &fams, Some(&*prev), warning.as_deref())
                {
                    if pick == crate::widgets::ADD_FONT {
                        let _ = app.run("font.add", json!({}));
                    } else if pick == crate::widgets::LOAD_SYSTEM_FONTS {
                        let _ = app.run("font.loadSystem", json!({}));
                    } else {
                        *font = pick;
                    }
                }
                ui.end_row();
                ui.label(tl!("Typeface:"));
                let shown = STYLES.iter().find(|(b, i, _)| *b == *bold && *i == *italic).map_or("", |(_, _, l)| *l);
                egui::ComboBox::from_id_salt(format!("{salt}_style")).width(170.0).selected_text(tl!(shown)).show_ui(ui, |ui| {
                    for (b, i, l) in STYLES {
                        if ui.steady_label(*bold == b && *italic == i, tl!(l)).clicked() {
                            (*bold, *italic) = (b, i);
                            ui.close();
                        }
                    }
                });
                ui.end_row();
                ui.label(tl!("Size:"));
                ui.add(egui::TextEdit::singleline(size).desired_width(60.0));
                ui.end_row();
                ui.label(tl!("Language:"));
                let shown = langs.iter().find(|(t, _)| t.eq_ignore_ascii_case(lang)).map_or_else(|| lang.clone(), |(_, n)| tl!(n).to_string());
                egui::ComboBox::from_id_salt(format!("{salt}_lang")).width(170.0).selected_text(shown).show_ui(ui, |ui| {
                    for (t, n) in langs {
                        if ui.steady_label(lang.eq_ignore_ascii_case(t), tl!(n)).clicked() {
                            *lang = t.to_string();
                            ui.close();
                        }
                    }
                });
                ui.end_row();
            });
        });
    };
    ui.horizontal_top(|ui| {
        column(ui, "Western text", "ch_latin", &mut f.font, &mut f.size, &mut f.bold, &mut f.italic, &mut f.lang, &western);
        ui.add_space(12.0);
        column(
            ui,
            "Thai text (complex script)",
            "ch_thai",
            &mut f.font_cs,
            &mut f.size_cs,
            &mut f.bold_cs,
            &mut f.italic_cs,
            &mut f.lang_cs,
            &COMPLEX_LANGUAGES,
        );
    });
    ui.add_space(8.0);
    if ui.small_button(tl!("Thai text like Western text")).on_hover_text(tl!("Copy the Western font, size and typeface to Thai text")).clicked() {
        f.font_cs = f.font.clone();
        f.size_cs = f.size.clone();
        f.bold_cs = f.bold;
        f.italic_cs = f.italic;
    }
}

fn effects_page(app: &mut WordApp, ui: &mut Ui, f: &mut CharForm) {
    let theme = app.session.doc.settings.theme_colors.clone();
    heading(ui, "Font Color");
    egui::Grid::new("ch_color").num_columns(2).spacing(vec2(10.0, 8.0)).show(ui, |ui| {
        ui.label(tl!("Font color:"));
        crate::dialogs::color_field(ui, &theme, &mut f.color, "Automatic", "auto");
        ui.end_row();
    });
    heading(ui, "Text Decoration");
    egui::Grid::new("ch_decoration").num_columns(2).spacing(vec2(10.0, 8.0)).show(ui, |ui| {
        ui.label(tl!("Underlining:"));
        ui.horizontal(|ui| {
            crate::line_looks::names(ui, egui::Id::new("ch_ul"), 140.0, &mut f.underline, &UNDERLINES);
            crate::dialogs::color_field(ui, &theme, &mut f.underline_color, "Automatic", "auto");
        });
        ui.end_row();
        ui.label(tl!("Strikethrough:"));
        let names = ["(none)", "Single", "Double"];
        egui::ComboBox::from_id_salt("ch_strike").width(140.0).selected_text(tl!(names[usize::from(f.strike.min(2))])).show_ui(ui, |ui| {
            for (i, l) in names.iter().enumerate() {
                ui.steady_value(&mut f.strike, i as u8, tl!(l));
            }
        });
        ui.end_row();
    });
    heading(ui, "Effects");
    egui::Grid::new("ch_fx").num_columns(4).spacing(vec2(16.0, 6.0)).show(ui, |ui| {
        if ui.checkbox(&mut f.caps, tl!("All caps")).changed() && f.caps {
            f.small_caps = false;
        }
        if ui.checkbox(&mut f.small_caps, tl!("Small caps")).changed() && f.small_caps {
            f.caps = false;
        }
        ui.checkbox(&mut f.hidden, tl!("Hidden"));
        ui.end_row();
        ui.checkbox(&mut f.outline, tl!("Outline"));
        ui.checkbox(&mut f.shadow, tl!("Shadow"));
        // Emboss and engrave exclude each other, as in Word.
        if ui.checkbox(&mut f.emboss, tl!("Emboss")).changed() && f.emboss {
            f.engrave = false;
        }
        if ui.checkbox(&mut f.engrave, tl!("Engrave")).changed() && f.engrave {
            f.emboss = false;
        }
        ui.end_row();
    });
}

fn position_page(ui: &mut Ui, f: &mut CharForm) {
    heading(ui, "Position");
    egui::Grid::new("ch_pos").num_columns(2).spacing(vec2(16.0, 6.0)).show(ui, |ui| {
        ui.radio_value(&mut f.vert, 0, tl!("Normal"));
        ui.horizontal(|ui| {
            ui.label(tl!("Raise (+) or lower (−) by (pt):"));
            ui.add(crate::numfield::NumField::new(&mut f.raise).range(-1584.0..=1584.0).speed(0.5).max_decimals(1));
        });
        ui.end_row();
        ui.radio_value(&mut f.vert, 1, tl!("Superscript"));
        ui.end_row();
        ui.radio_value(&mut f.vert, 2, tl!("Subscript"));
        ui.end_row();
    });
    heading(ui, "Scale and Spacing");
    egui::Grid::new("ch_spacing").num_columns(2).spacing(vec2(10.0, 8.0)).show(ui, |ui| {
        ui.label(tl!("Scale width (%):"));
        ui.add(crate::numfield::NumField::new(&mut f.scale).range(1.0..=600.0).speed(1.0).max_decimals(0));
        ui.end_row();
        ui.label(tl!("Character spacing (pt):"));
        ui.add(crate::numfield::NumField::new(&mut f.spacing).speed(0.1).range(-20.0..=20.0).spinner());
        ui.end_row();
        ui.checkbox(&mut f.kern, tl!("Kerning for fonts (pt) and up:"));
        ui.add_enabled(f.kern, crate::numfield::NumField::new(&mut f.kern_at).range(1.0..=1638.0).speed(0.5));
        ui.end_row();
    });
}

fn background_page(app: &mut WordApp, ui: &mut Ui, f: &mut CharForm) {
    let theme = app.session.doc.settings.theme_colors.clone();
    heading(ui, "Background");
    ui.horizontal(|ui| {
        if ui.steady_label(f.shading.is_empty(), tl!("None")).clicked() {
            f.shading.clear();
        }
        if ui.steady_label(!f.shading.is_empty(), tl!("Color")).clicked() && f.shading.is_empty() {
            f.shading = "D9D9D9".into();
        }
    });
    if !f.shading.is_empty() {
        ui.add_space(6.0);
        egui::Grid::new("ch_bg").num_columns(2).spacing(vec2(10.0, 8.0)).show(ui, |ui| {
            ui.label(tl!("Color:"));
            crate::dialogs::color_field(ui, &theme, &mut f.shading, "No Color", "");
            ui.end_row();
        });
    }
    ui.add_space(6.0);
    ui.label(egui::RichText::new(tl!("Shading behind the selected characters (not the highlight color).")).small().weak());
}

fn border_page(app: &mut WordApp, ui: &mut Ui, f: &mut CharForm) {
    let theme = app.session.doc.settings.theme_colors.clone();
    heading(ui, "Line Arrangement");
    ui.horizontal(|ui| {
        if ui.steady_label(!f.border, tl!("None")).clicked() {
            f.border = false;
        }
        if ui.steady_label(f.border, tl!("Box")).clicked() {
            f.border = true;
        }
    });
    heading(ui, "Line");
    ui.add_enabled_ui(f.border, |ui| {
        egui::Grid::new("ch_border").num_columns(2).spacing(vec2(10.0, 8.0)).show(ui, |ui| {
            ui.label(tl!("Style:"));
            crate::line_looks::names(ui, egui::Id::new("ch_border_style"), 140.0, &mut f.border_style, &LINES);
            ui.end_row();
            ui.label(tl!("Color:"));
            crate::dialogs::color_field(ui, &theme, &mut f.border_color, "Automatic", "auto");
            ui.end_row();
            ui.label(tl!("Width:"));
            egui::ComboBox::from_id_salt("ch_border_width")
                .width(140.0)
                .selected_text(format!("{} {}", trim_num(f.border_width), tl!("pt")))
                .show_ui(ui, |ui| {
                    for w in WIDTHS {
                        ui.steady_value(&mut f.border_width, w, format!("{} {}", trim_num(w), tl!("pt")));
                    }
                });
            ui.end_row();
        });
    });
    ui.add_space(6.0);
    ui.label(egui::RichText::new(tl!("Word files keep one border all round the characters, so there are no separate sides.")).small().weak());
}

/// The selected text as the fields would format it (size, typeface, colour, underline,
/// strikethrough, caps, position, shading and border; the page's own font isn't drawn).
fn preview(ui: &mut Ui, f: &CharForm, sample: &str) {
    let size = f.size.trim().parse::<f32>().unwrap_or(16.0).clamp(8.0, 30.0) * if f.vert == 0 { 1.0 } else { 0.65 };
    let color = wordcraft_doc::Rgb::parse(&f.color).map(crate::theme::c32).unwrap_or_else(|| ui.visuals().text_color());
    let font = if f.bold { crate::theme::semibold(size) } else { egui::FontId::new(size, egui::FontFamily::Proportional) };
    let mut fmt = egui::TextFormat { font_id: font, color, italics: f.italic, ..Default::default() };
    if !matches!(f.underline.as_str(), "" | "none") {
        let uc = wordcraft_doc::Rgb::parse(&f.underline_color).map(crate::theme::c32).unwrap_or(color);
        fmt.underline = egui::Stroke::new(if f.underline == "thick" { 2.0 } else { 1.0 }, uc);
    }
    if f.strike > 0 {
        fmt.strikethrough = egui::Stroke::new(f.strike as f32, color);
    }
    if let Some(c) = wordcraft_doc::Rgb::parse(&f.shading) {
        fmt.background = crate::theme::c32(c);
    }
    fmt.valign = match f.vert {
        1 => egui::Align::TOP,
        2 => egui::Align::BOTTOM,
        _ => egui::Align::Center,
    };
    let text = if f.caps { sample.to_uppercase() } else { sample.to_string() };
    let t = Tokens::get(ui.ctx());
    let frame_h = 58.0;
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), frame_h), egui::Sense::hover());
    ui.painter().rect_stroke(r, 4.0, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    let mut job = egui::text::LayoutJob::single_section(text, fmt);
    job.wrap.max_width = r.width() - 16.0;
    job.wrap.max_rows = 1;
    let galley = ui.painter().layout_job(job);
    let at = r.center() - galley.size() / 2.0;
    if f.border {
        let bc = wordcraft_doc::Rgb::parse(&f.border_color).map(crate::theme::c32).unwrap_or(color);
        let br = egui::Rect::from_min_size(at, galley.size()).expand(2.0);
        ui.painter().rect_stroke(br, 0.0, egui::Stroke::new(f.border_width.clamp(0.5, 3.0), bc), egui::StrokeKind::Outside);
    }
    ui.painter().galley(at, galley, color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordcraft_engine::Session;

    fn app_with(text: &str) -> WordApp {
        let mut app = WordApp::new(Session::new(wordcraft_doc::Document::from_text(text)), crate::Services::default());
        app.run("select.text", json!({"text": text})).unwrap();
        app
    }

    fn form(app: &mut WordApp) -> CharForm {
        let st = app.session.run("format.state", &json!({})).unwrap();
        CharForm::from_state(app, &st)
    }

    /// OK with nothing changed changes nothing; changing the Thai column sets only the
    /// complex-script font and size, and leaves Western text as it was.
    #[test]
    fn ok_sends_only_what_changed() {
        let mut app = app_with("กขค abc");
        let f = form(&mut app);
        assert!(f.changes(&mut |s| s.to_string()).is_none(), "nothing changed");
        assert_eq!(f.lang_cs, "th-TH", "Thai text without a language shows Thai");
        let mut f = form(&mut app);
        f.font_cs = "Tahoma".into();
        f.size_cs = "20".into();
        let params = f.changes(&mut |s| s.to_string()).unwrap();
        let props = params["props"].as_object().unwrap();
        assert_eq!(props.keys().cloned().collect::<std::collections::BTreeSet<_>>(), ["fontCs", "sizeCs"].iter().map(|s| s.to_string()).collect());
        app.run("format.set", params).unwrap();
        let st = app.session.run("format.state", &json!({})).unwrap();
        assert_eq!(st["fontCs"], "Tahoma");
        assert_eq!(st["sizeCs"], 20.0);
        assert_ne!(st["font"], "Tahoma", "Western text keeps its font");
    }

    /// Effects, position, shading and a border go on in one step; Background › None and Border ›
    /// None take them off again, in one step too.
    #[test]
    fn effects_shading_and_border_go_on_and_off() {
        let mut app = app_with("Kilns");
        let mut f = form(&mut app);
        f.strike = 2;
        f.small_caps = true;
        f.vert = 1;
        f.scale = 150.0;
        f.underline = "double".into();
        f.shading = "FFF2CC".into();
        f.border = true;
        f.border_style = "double".into();
        f.border_width = 1.5;
        let depth = app.session.undo_depth();
        app.run("format.set", f.changes(&mut |s| s.to_string()).unwrap()).unwrap();
        assert_eq!(app.session.undo_depth(), depth + 1, "one step");
        let st = app.session.run("format.state", &json!({})).unwrap();
        assert_eq!((st["doubleStrike"].as_bool(), st["smallCaps"].as_bool(), st["superscript"].as_bool()), (Some(true), Some(true), Some(true)));
        assert_eq!((st["scale"].as_f64(), st["underlineStyle"].as_str(), st["shading"].as_str()), (Some(150.0), Some("double"), Some("FFF2CC")));
        assert_eq!(st["borderLine"]["style"], "double");
        let mut f = form(&mut app);
        assert!(f.border && f.shading == "FFF2CC" && f.strike == 2, "the form reads them back");
        f.shading.clear();
        f.border = false;
        app.run("format.set", f.changes(&mut |s| s.to_string()).unwrap()).unwrap();
        let st = app.session.run("format.state", &json!({})).unwrap();
        assert!(st["shading"].is_null() && !st["border"].as_bool().unwrap_or(true), "{st}");
    }
}
