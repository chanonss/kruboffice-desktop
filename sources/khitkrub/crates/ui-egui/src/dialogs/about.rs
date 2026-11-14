use super::Dialog;

/// Help ▸ About KhitKrub: the app, its contributors and the AI models that helped. The credits
/// are compiled in (`crate::credits`, docs/contributors.md).
pub(super) fn about(ui: &mut egui::Ui, d: &mut Dialog) {
    let lang = crate::i18n::current(ui.ctx());
    ui.horizontal(|ui| {
        for tab in ["About", "Contributors", "Models"] {
            if ui.selectable_label(d.tab == tab, lang.tr(tab)).clicked() {
                d.tab = tab.to_string();
            }
        }
    });
    ui.separator();
    match d.tab.as_str() {
        "Contributors" => crate::credits::contributors_ui(ui),
        "Models" => crate::credits::models_ui(ui),
        _ => {
            // The same About as KhianKrub's and PhutKrub's: name, version, what it is, where it comes from.
            ui.heading(lang.tr("KhitKrub"));
            ui.label(version_line(lang));
            let description = if cfg!(target_arch = "wasm32") {
                "A free spreadsheet for Thai and English.\nBased on GridCraft by the ArtCraft team. Fork by Chanon Sarasuk."
            } else {
                "A free, open-source spreadsheet for Thai and English.\nBased on GridCraft by the ArtCraft team. Fork by Chanon Sarasuk."
            };
            ui.label(lang.tr(description));
            ui.add_space(6.0);
            if cfg!(target_arch = "wasm32") {
                ui.hyperlink_to("github.com/chanonss/khitkrub-web", "https://github.com/chanonss/khitkrub-web");
            } else {
                ui.hyperlink_to(lang.tr("KrubOffice: github.com/chanonss/kruboffice-desktop"), "https://github.com/chanonss/kruboffice-desktop");
            }
            ui.hyperlink_to(lang.tr("Original project: github.com/storytold/gridcraft"), "https://github.com/storytold/gridcraft");
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(lang.tr(
                    "MIT OR Apache-2.0. Copyright (c) 2026 ArtCraft Team and the GridCraft contributors. Modifications copyright (c) 2026 Chanon Sarasuk.",
                ))
                .small()
                .weak(),
            );
        }
    }
}

/// "Version 0.3.0", plus the source commit when the release build sets `GRIDCRAFT_BUILD_SHA`.
pub fn version_line(lang: crate::i18n::Language) -> String {
    let mut s = format!("{} {}", lang.tr("Version"), crate::APP_VERSION);
    if let Some(sha) = option_env!("GRIDCRAFT_BUILD_SHA").filter(|s| !s.is_empty()) {
        s.push_str(&format!(" ({})", sha.get(..8).unwrap_or(sha)));
    }
    s
}
