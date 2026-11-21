use super::{Rect, Tokens, Ui};

/// PhutKrub's tables for the shared About dialog: who wrote DeckCraft and which AI models helped.
struct Host;

impl krub_icons::about::Host for Host {
    fn contributors(&mut self, ui: &mut Ui) {
        crate::credits::contributors_ui(ui);
    }
    fn models(&mut self, ui: &mut Ui) {
        crate::credits::models_ui(ui);
    }
}

/// The About dialog's tabs as kept in the dialog's `tab` field, in the shared order.
const TABS: [&str; 4] = ["about", "contributors", "models", "licences"];

/// About PhutKrub (owner, 2026-11-18): the About dialog shared by the three KrubOffice apps
/// (`krub_icons::about`), drawn with its own frame. `true` once it closed.
pub(super) fn window(ctx: &egui::Context, d: &mut super::Dialog) -> bool {
    let t = Tokens::get(ctx);
    let sha = option_env!("DECKCRAFT_BUILD_SHA").map(|s| s.get(..9).unwrap_or(s)).filter(|s| !s.is_empty());
    let (source_label, source_url) = if cfg!(target_arch = "wasm32") {
        ("github.com/chanonss/phutkrub-web".to_string(), "https://github.com/chanonss/phutkrub-web")
    } else {
        (tl!("KrubOffice on GitHub").to_string(), "https://github.com/chanonss/kruboffice-desktop")
    };
    let info = krub_icons::about::Info {
        name: tl!("PhutKrub").to_string(),
        english: "PhutKrub",
        logo: krub_icons::logo::App::PhutKrub,
        color: t.band,
        version: crate::APP_VERSION,
        build: sha,
        tagline: tl!("A free, open-source presentation app for Thai and English.").to_string(),
        upstream: "DeckCraft",
        upstream_url: "https://github.com/storytold/deckcraft",
        source_label,
        source_url,
        credits: Vec::new(),
    };
    let current = d.fields.get("tab").cloned().unwrap_or_default();
    let mut tab = TABS.iter().position(|k| *k == current).unwrap_or(0);
    let closed = krub_icons::about::show(ctx, &info, &mut tab, &|s: &str| tl!(s).to_string(), &mut Host);
    d.fields.insert("tab".into(), TABS.get(tab).copied().unwrap_or("about").into());
    closed
}

/// What PhutKrub is, in one line, and where it comes from (shared by the About window and File › About).
pub fn description() -> &'static str {
    if cfg!(target_arch = "wasm32") {
        tl!("A free presentation app for Thai and English.\nBased on DeckCraft by the ArtCraft team. Fork by Chanon Sarasuk.")
    } else {
        tl!("A free, open-source presentation app for Thai and English.\nBased on DeckCraft by the ArtCraft team. Fork by Chanon Sarasuk.")
    }
}

/// KrubOffice's repository (the desktop app) or the built web app, and the original project.
pub fn project_links(ui: &mut Ui) {
    if cfg!(target_arch = "wasm32") {
        ui.hyperlink_to("github.com/chanonss/phutkrub-web", "https://github.com/chanonss/phutkrub-web");
    } else {
        ui.hyperlink_to(tl!("KrubOffice: github.com/chanonss/kruboffice-desktop"), "https://github.com/chanonss/kruboffice-desktop");
    }
    ui.hyperlink_to(tl!("Original project: github.com/storytold/deckcraft"), "https://github.com/storytold/deckcraft");
}

/// "Version X.Y.Z" plus, for release builds, the short commit and build date. The version comes
/// from `[workspace.package] version` (the single source of truth, `cargo xtask version`); the
/// release workflow sets `DECKCRAFT_BUILD_SHA` / `DECKCRAFT_BUILD_DATE` at build time.
pub fn version_line() -> String {
    let mut s = format!("{} {}", tl!("Version"), crate::APP_VERSION);
    let sha = option_env!("DECKCRAFT_BUILD_SHA").map(|s| s.get(..9).unwrap_or(s)).filter(|s| !s.is_empty());
    let date = option_env!("DECKCRAFT_BUILD_DATE").filter(|s| !s.is_empty());
    match (sha, date) {
        (Some(sha), Some(date)) => s.push_str(&format!(" ({sha}, {date})")),
        (Some(x), None) | (None, Some(x)) => s.push_str(&format!(" ({x})")),
        (None, None) => {}
    }
    s
}

/// PhutKrub's logo (the glass พ of its app icon), on File › Home. It was DeckCraft's orange
/// play-button mark (owner, 2026-11-21: "not our logo").
pub fn paint_logo(p: &egui::Painter, r: Rect) {
    krub_icons::logo::paint(p, r, krub_icons::logo::App::PhutKrub, 1.0);
}

#[cfg(test)]
mod about_credits_tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    use crate::dialogs::Dialog;
    use crate::{Services, SlideApp};

    fn tab_of(h: &Harness<'_, SlideApp>) -> String {
        let d = h.state().dialog.as_ref().expect("the About dialog is open");
        d.fields.get("tab").cloned().unwrap_or_else(|| "about".into())
    }

    /// The reported symptom (#843): in the real dialog the tabs stopped answering once Contributors
    /// or Models was open. Click through every tab of the shared About and back.
    #[test]
    fn about_dialog_tabs_keep_responding() {
        let mut app = SlideApp::new(deckcraft_engine::Session::with_new(), Services::default());
        app.dialog = Some(Dialog::new("about"));
        let mut h = Harness::builder().with_size(egui::vec2(1440.0, 900.0)).with_step_dt(1.0 / 60.0).build_ui_state(
            |ui, app: &mut SlideApp| {
                app.logic(ui.ctx());
                app.ui(ui);
            },
            app,
        );
        h.run_steps(4);
        assert_eq!(tab_of(&h), "about");
        for (label, key) in
            [("Contributors", "contributors"), ("AI models", "models"), ("Licences", "licences"), ("AI models", "models"), ("About", "about")]
        {
            h.get_by_label(label).click();
            h.run_steps(3);
            assert_eq!(tab_of(&h), key, "clicking {label} switches the tab");
        }
        h.get_by_label("Close").click();
        h.run_steps(3);
        assert!(h.state().dialog.is_none(), "Close closes it");
    }
}
