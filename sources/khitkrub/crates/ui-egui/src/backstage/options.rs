use super::{Key, Language, Modifiers, SheetApp, Ui, dim, i18n, json, section, vec2};

/// File › Options in the shared KrubOffice layout (`krub_icons::options`): categories on the
/// left, the chosen one's settings on the right. General (interface), Display (formula bar,
/// active row, and this workbook's scroll bars and sheet tabs), Formulas (calculation, saved
/// with the workbook), Save (autosave), Language and Advanced (AutoComplete). Display options
/// for the workbook and Formulas need an open workbook. Every change runs its command through
/// `run_or_alert`.
pub(super) fn options(app: &mut SheetApp, ui: &mut Ui, pane: &mut String) {
    let lang = app.ui.language;
    let t = crate::theme::Tokens::get(ui.ctx());
    let style = krub_icons::options::Style {
        text: t.text,
        icon: t.text,
        selected: t.selected,
        hover: t.hover,
        border: t.separator,
        regular: crate::theme::ui_font(14.0),
        semibold: crate::theme::ui_bold(14.0),
        announce: |resp, active, label| resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, active, label)),
    };
    let has_workbook = app.session.active().is_some();
    let mut shown = vec!["general", "display", "save", "language", "advanced"];
    if has_workbook {
        shown.push("formulas");
    }
    krub_icons::options::page(
        ui,
        &style,
        &shown,
        |name| category_name(lang, name).to_string(),
        pane,
        |ui, id| match id {
            "display" => display(app, ui, lang),
            "formulas" => formulas(app, ui, lang),
            "save" => {
                section(ui, lang, "Options|Save workbooks");
                let mut autosave = app.ui.autosave;
                if ui.checkbox(&mut autosave, lang.tr("Save .xlsx workbooks automatically after every change")).changed() {
                    app.toggle_autosave();
                }
            }
            "language" => {
                section(ui, lang, "Interface language");
                language_picker(app, ui, lang);
                ui.add_space(4.0);
                dim(ui, lang.tr("Menus and commands change; your workbooks don't."));
            }
            "advanced" => {
                section(ui, lang, "Options|Editing");
                let mut autocomplete = app.ui.autocomplete;
                if ui.checkbox(&mut autocomplete, lang.tr("Enable AutoComplete for cell values")).changed() {
                    app.run_or_alert("options.autoComplete", json!({"on": autocomplete}));
                }
            }
            _ => {
                section(ui, lang, "Options|Interface");
                general_options(app, ui, lang);
            }
        },
    );
}

/// A category's name in the interface language.
fn category_name(lang: Language, english: &str) -> &str {
    lang.tr(match english {
        "Display" => "Options|Display",
        "Formulas" => "Options|Formulas",
        "Save" => "Options|Save",
        "Language" => "Options|Language",
        "Advanced" => "Options|Advanced",
        "Proofing" => "Options|Proofing",
        "Accessibility" => "Options|Accessibility",
        _ => "Options|General",
    })
}

/// Display: the formula bar and active row, and this workbook's scroll bars and sheet tabs.
fn display(app: &mut SheetApp, ui: &mut Ui, lang: Language) {
    section(ui, lang, "Options|Show");
    let mut formula_bar = app.session.prefs.formula_bar;
    if ui.checkbox(&mut formula_bar, lang.tr("Show formula bar")).changed() {
        app.run_or_alert("view.formulaBar", json!({"on": formula_bar}));
    }
    let mut highlight_row = app.ui.highlight_active_row;
    if ui.checkbox(&mut highlight_row, lang.tr("Highlight the active cell's row")).changed() {
        app.run_or_alert("options.highlightActiveRow", json!({"on": highlight_row}));
    }
    let Some(view) = app.session.active().map(|d| d.wb.view) else { return };
    ui.add_space(12.0);
    // Excel's Advanced › Display options for this workbook: saved with the workbook (#3101).
    section(ui, lang, "Display options for this workbook");
    for (on, label, id) in [
        (view.show_horizontal_scroll, "Show horizontal scroll bar", "view.showHorizontalScrollBar"),
        (view.show_vertical_scroll, "Show vertical scroll bar", "view.showVerticalScrollBar"),
        (view.show_sheet_tabs, "Show sheet tabs", "view.showSheetTabs"),
    ] {
        let mut on = on;
        if ui.checkbox(&mut on, lang.tr(label)).changed() {
            app.run_or_alert(id, json!({"on": on}));
        }
    }
}

/// Formulas: the active workbook's calculation settings.
fn formulas(app: &mut SheetApp, ui: &mut Ui, lang: Language) {
    let Some(calc) = app.session.active().map(|d| d.wb.calc.clone()) else { return };
    section(ui, lang, "Workbook calculation");
    calc_options(app, ui, lang, &calc);
    ui.add_space(6.0);
    dim(ui, lang.tr("Calculation settings are saved with the workbook."));
}

/// General › Interface: ribbon, Draw and Automate tabs, interface fonts and the display theme.
fn general_options(app: &mut SheetApp, ui: &mut Ui, lang: Language) {
    ui.checkbox(&mut app.ui.show_ribbon_tab_numbers, lang.tr("Show numbers on ribbon tabs"));
    ui.checkbox(&mut app.ui.ribbon_tabs_centered, lang.tr("Center the ribbon tabs (off: start at the left)"));
    ui.checkbox(&mut app.ui.show_draw_tab, lang.tr("Show the Draw tab"));
    ui.checkbox(&mut app.ui.show_automate_tab, lang.tr("Show the Automate tab"));
    if let Some(load) = &app.services.load_system_ui_fonts {
        if ui.button(lang.tr("Use system interface fonts")).clicked() {
            load();
        }
        dim(ui, lang.tr("Your browser may ask for access to local fonts. Font files stay on this computer."));
    } else if cfg!(target_arch = "wasm32") {
        dim(ui, lang.tr("This browser cannot load system interface fonts. Bundled interface fonts are used."));
    }
    egui::Grid::new("backstage_options_general").num_columns(2).spacing(vec2(18.0, 10.0)).show(ui, |ui| {
        let label = ui.label(lang.tr("Display theme"));
        let mode = app.ui.theme_mode();
        let current = match mode {
            "system" => "System",
            "dark" => "Dark",
            _ => "Light",
        };
        craft_ui::ComboBox::from_id_salt("backstage_theme")
            .width(160.0)
            .selected_text(lang.tr(current))
            .show_ui(ui, |ui| {
                for (value, label) in [("system", "System"), ("light", "Light"), ("dark", "Dark")] {
                    if ui.selectable_label(mode == value, lang.tr(label)).clicked() {
                        app.run_or_alert("view.theme", json!({"mode": value}));
                    }
                }
            })
            .response
            .labelled_by(label.id);
        ui.end_row();
        let label = ui.label(lang.tr("Mouse pointer"));
        let current = app.ui.pointer;
        ui.horizontal(|ui| {
            current.preview(ui, 20.0);
            craft_ui::ComboBox::from_id_salt("backstage_pointer")
                .width(160.0)
                .selected_text(lang.tr(current.label()))
                .show_ui(ui, |ui| {
                    for p in crate::pointer::Pointer::ALL {
                        let row = ui.horizontal(|ui| {
                            p.preview(ui, 20.0);
                            ui.selectable_label(current == p, lang.tr(p.label()))
                        });
                        if row.inner.clicked() {
                            app.ui.pointer = p;
                        }
                    }
                })
                .response
                .labelled_by(label.id);
        });
        ui.end_row();
    });
    dim(ui, lang.tr("Changes only the arrow. The pointers for typing text, links and resizing stay the system's."));
}

/// The interface language picker.
fn language_picker(app: &mut SheetApp, ui: &mut Ui, lang: Language) {
    ui.horizontal(|ui| {
        let label = ui.label(lang.tr("Interface language"));
        craft_ui::ComboBox::from_id_salt("backstage_language")
            .width(160.0)
            .selected_text(lang.name())
            .show_ui(ui, |ui| {
                for l in i18n::Language::for_picker() {
                    if ui.selectable_label(l == lang, l.name()).clicked() {
                        app.run_or_alert("app.language.set", json!({"language": l.code()}));
                    }
                }
            })
            .response
            .labelled_by(label.id);
    });
}

/// The calculation mode, and iterative calculation, of the active workbook.
fn calc_options(app: &mut SheetApp, ui: &mut Ui, lang: Language, calc: &gridcraft_engine::model::CalcSettings) {
    use gridcraft_engine::model::CalcMode;
    for (mode, id, label) in [
        (CalcMode::Automatic, "automatic", "Automatic"),
        (CalcMode::AutomaticExceptTables, "automaticExceptTables", "Automatic Except for Data Tables"),
        (CalcMode::Manual, "manual", "Manual"),
    ] {
        if ui.radio(calc.mode == mode, lang.tr(label)).clicked() && calc.mode != mode {
            app.run_or_alert("formulas.calculationOptions", json!({"mode": id}));
        }
    }
    ui.add_space(4.0);
    let mut iterative = calc.iterative;
    if ui.checkbox(&mut iterative, lang.tr("Enable iterative calculation")).changed() {
        app.run_or_alert("formulas.calculationOptions", json!({"iterative": iterative}));
    }
    // Maximum Iterations and Maximum Change apply only while iteration is on, as in Excel.
    iteration_options(app, ui, lang, calc, iterative);
    // TODO: add "Set precision as displayed" here once `formulas.calculationOptions` takes
    // `precisionAsDisplayed` (PR #1366); the model already stores it.
}

/// Maximum Iterations and Maximum Change, live only while iterative calculation is on.
fn iteration_options(app: &mut SheetApp, ui: &mut Ui, lang: Language, calc: &gridcraft_engine::model::CalcSettings, iterative: bool) {
    ui.add_enabled_ui(iterative, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            egui::Grid::new("backstage_options_iteration").num_columns(2).spacing(vec2(12.0, 6.0)).show(ui, |ui| {
                let label = ui.label(lang.tr("Maximum Iterations:"));
                let id = ui.make_persistent_id("backstage_max_iterations");
                // Text left in the field when iteration is switched off (it loses focus) is dropped.
                let committed = crate::widgets::committed_text(ui, id, &calc.max_iterations.to_string(), Some(80.0));
                if let Some(response) = ui.ctx().read_response(id) {
                    response.labelled_by(label.id);
                }
                if let Some(text) = committed.filter(|_| iterative) {
                    match parse_max_iterations(&text) {
                        Some(n) if n != calc.max_iterations => {
                            app.run_or_alert("formulas.calculationOptions", json!({"maxIterations": n}));
                        }
                        Some(_) => {}
                        None => refuse(app, ui, "Maximum Iterations must be a whole number from 1 to 32767."),
                    }
                }
                ui.end_row();
                let label = ui.label(lang.tr("Maximum Change:"));
                let id = ui.make_persistent_id("backstage_max_change");
                let committed = crate::widgets::committed_text(ui, id, &calc.max_change.to_string(), Some(80.0));
                if let Some(response) = ui.ctx().read_response(id) {
                    response.labelled_by(label.id);
                }
                if let Some(text) = committed.filter(|_| iterative) {
                    match parse_max_change(&text, lang.formula_locale()) {
                        Some(d) if d != calc.max_change => {
                            app.run_or_alert("formulas.calculationOptions", json!({"maxChange": d}));
                        }
                        Some(_) => {}
                        None => refuse(app, ui, "Maximum Change must be a number of 0 or more."),
                    }
                }
                ui.end_row();
            });
        });
    });
}

/// Refuses a typed setting with a message, leaving the workbook as it was. The Enter that
/// committed the field is consumed so it doesn't also dismiss the message box in this frame.
fn refuse(app: &mut SheetApp, ui: &mut Ui, text: &str) {
    ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
    app.message = Some(("KhitKrub".into(), app.ui.language.tr(text).into()));
}

/// Maximum Iterations as typed: a whole number from 1 to 32767, or `None` (Excel refuses others).
fn parse_max_iterations(text: &str) -> Option<u32> {
    let n = text.trim().parse::<u32>().ok()?;
    (1..=gridcraft_engine::model::CalcSettings::MAX_ITERATIONS).contains(&n).then_some(n)
}

/// Maximum Change as typed: a finite number of 0 or more, or `None`. The interface's decimal
/// separator is accepted as well as a dot.
fn parse_max_change(text: &str, locale: gridcraft_engine::formula::FormulaLocale) -> Option<f64> {
    let d = text.trim().replace(locale.decimal_separator(), ".").parse::<f64>().ok()?;
    (d.is_finite() && d >= 0.0).then_some(d)
}

/// The platform the app was built for, e.g. "macOS (aarch64)"; the web build reports "Web".
pub(super) fn platform() -> String {
    let os = match std::env::consts::OS {
        "macos" => "macOS",
        "windows" => "Windows",
        "linux" => "Linux",
        "freebsd" => "FreeBSD",
        "openbsd" => "OpenBSD",
        "netbsd" => "NetBSD",
        _ if cfg!(target_arch = "wasm32") => return "Web".into(),
        other => other,
    };
    format!("{os} ({})", std::env::consts::ARCH)
}
