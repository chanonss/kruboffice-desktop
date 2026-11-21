use super::Dialog;

/// KhitKrub's tables for the shared About dialog: who wrote GridCraft and which AI models helped
/// (compiled in, `crate::credits`, docs/contributors.md).
struct Host;

impl krub_icons::about::Host for Host {
    fn contributors(&mut self, ui: &mut egui::Ui) {
        crate::credits::contributors_ui(ui);
    }
    fn models(&mut self, ui: &mut egui::Ui) {
        crate::credits::models_ui(ui);
    }
}

/// Help ▸ About KhitKrub (owner, 2026-11-18): the About dialog shared by the three KrubOffice
/// apps (`krub_icons::about`), drawn with its own frame. `true` once it closed.
pub(super) fn window(ctx: &egui::Context, d: &mut Dialog) -> bool {
    let lang = crate::i18n::current(ctx);
    let t = crate::theme::Tokens::get(ctx);
    let build = option_env!("GRIDCRAFT_BUILD_SHA").filter(|s| !s.is_empty()).map(|s| s.get(..8).unwrap_or(s));
    let (source_label, source_url) = if cfg!(target_arch = "wasm32") {
        ("github.com/chanonss/khitkrub-web".to_string(), "https://github.com/chanonss/khitkrub-web")
    } else {
        (lang.tr("KrubOffice on GitHub").to_string(), "https://github.com/chanonss/kruboffice-desktop")
    };
    let info = krub_icons::about::Info {
        name: lang.tr("KhitKrub").to_string(),
        english: "KhitKrub",
        logo: krub_icons::logo::App::KhitKrub,
        color: t.band,
        version: crate::APP_VERSION,
        build,
        tagline: lang.tr("A free, open-source spreadsheet for Thai and English.").to_string(),
        upstream: "GridCraft",
        upstream_url: "https://github.com/storytold/gridcraft",
        source_label,
        source_url,
        credits: Vec::new(),
    };
    let names = ["About", "Contributors", "Models", "Licences"];
    let mut tab = names.iter().position(|n| *n == d.tab).unwrap_or(0);
    let closed = krub_icons::about::show(ctx, &info, &mut tab, &|s: &str| lang.tr(s).to_string(), &mut Host);
    d.tab = names.get(tab).copied().unwrap_or("About").to_string();
    closed
}
