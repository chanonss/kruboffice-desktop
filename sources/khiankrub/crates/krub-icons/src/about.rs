//! The About dialog, the same in every KrubOffice app (owner, 2026-11-18): a centred dialog over
//! the app with a neutral title row, the app's logo, name, version and one-line description, a
//! segmented control ([`crate::pane_ui::segmented_at`]) with About, Contributors, AI models and
//! Licences, and one Close button in the app's colour.
//!
//! An app passes its [`Info`] (names, colour, version, links), a translator for the English
//! strings ([`NAMES`]) and a [`Host`] that draws what each app keeps its own way (the sortable
//! contributor and model tables). [`show`] draws one frame and says whether the dialog closed.
//! The apps run different egui versions, so this uses only what they share.

use egui::{Color32, FontId, RichText, Sense, Stroke, Ui, vec2};

/// The dialog's width, wide enough for the apps' contributor tables.
pub const WIDTH: f32 = 640.0;

/// The gold of the KrubOffice logos' letters.
const GOLD: Color32 = Color32::from_rgb(0xE8, 0xC1, 0x5A);

/// Where KrubOffice takes problem reports.
pub const REPORT_URL: &str = "https://github.com/chanonss/kruboffice-desktop/issues";
/// The KrubOffice front page.
pub const SITE_URL: &str = "https://kruboffice.app/";

/// What the dialog shows about one app.
pub struct Info<'a> {
    /// The app's name in the interface language ("เขียนครับ"), and in English ("KhianKrub").
    pub name: String,
    pub english: &'a str,
    /// The letter on the logo tile (ข, ค, พ) and the tile's colour (the app's band colour).
    pub letter: &'a str,
    pub color: Color32,
    pub version: &'a str,
    /// A short build id, when the build has one.
    pub build: Option<&'a str>,
    /// One line: what the app is.
    pub tagline: String,
    /// The project it is based on ("WordCraft") and its repository.
    pub upstream: &'a str,
    pub upstream_url: &'a str,
    /// The app's own source or site repository: its link's label and address.
    pub source_label: String,
    pub source_url: &'a str,
    /// Extra credit rows on the Contributors tab: (role, names), already translated.
    pub credits: Vec<(String, String)>,
}

/// The tabs, in the segmented control's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    About,
    Contributors,
    Models,
    Licences,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::About, Tab::Contributors, Tab::Models, Tab::Licences];

    fn icon(self) -> &'static str {
        match self {
            Tab::About => "info",
            Tab::Contributors => "user",
            Tab::Models => "agent",
            Tab::Licences => "file-name",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Tab::About => "About",
            Tab::Contributors => "Contributors",
            Tab::Models => "AI models",
            Tab::Licences => "Licences",
        }
    }
}

/// What each app draws its own way.
pub trait Host {
    /// The contributor table (who wrote the upstream project's code).
    fn contributors(&mut self, ui: &mut Ui);
    /// The AI model table (which models helped write the code).
    fn models(&mut self, ui: &mut Ui);
    /// Whether this build has the two tables (a web build may leave them out).
    fn has_credits(&self) -> bool {
        true
    }
}

/// The English strings the dialog shows, for the apps' translation tables.
pub const NAMES: [&str; 22] = [
    "About {app}",
    "About",
    "Contributors",
    "AI models",
    "Licences",
    "Version {version}",
    "Version",
    "Platform",
    "Based on",
    "Adapted by",
    "Licence",
    "{project} by the ArtCraft team",
    "MIT or Apache-2.0",
    "Original project {project}",
    "Report a problem",
    "Close",
    "Adapted for Thai",
    "Original project",
    "The ArtCraft team and the {project} contributors",
    "Icons: Tabler Icons by Paweł Kuna",
    "Fonts and the other open-source parts are listed in the NOTICE and ATTRIBUTION files that come with the program.",
    "Web",
];

/// Fills `{key}` placeholders in a translated string.
fn fill(s: &str, pairs: &[(&str, &str)]) -> String {
    pairs.iter().fold(s.to_string(), |acc, (k, v)| acc.replace(&format!("{{{k}}}"), v))
}

/// The platform this build runs on: "Web" (translated), or the system and processor.
pub fn platform(tr: &dyn Fn(&str) -> String) -> String {
    if cfg!(target_arch = "wasm32") {
        return tr("Web");
    }
    let os = match std::env::consts::OS {
        "macos" => "macOS",
        "windows" => "Windows",
        "linux" => "Linux",
        "freebsd" => "FreeBSD",
        other => other,
    };
    format!("{os} ({})", std::env::consts::ARCH)
}

/// The tabs this build shows.
pub fn tabs(host: &dyn Host) -> Vec<Tab> {
    Tab::ALL.into_iter().filter(|t| host.has_credits() || !matches!(t, Tab::Contributors | Tab::Models)).collect()
}

/// Draws the dialog for one frame over the app. `tab` is the open tab ([`Tab::ALL`] order, kept by
/// the app so a command can open the dialog on Contributors). Returns `true` once it closed: its
/// ×, Close, Escape or a click outside.
pub fn show(ctx: &egui::Context, info: &Info, tab: &mut usize, tr: &dyn Fn(&str) -> String, host: &mut dyn Host) -> bool {
    let id = crate::pane_ui::id("krub_about");
    let shown = tabs(host);
    let mut close = false;
    let modal = egui::Modal::new(id).show(ctx, |ui| {
        ui.set_width(WIDTH);
        close |= title_row(ui, &fill(&tr("About {app}"), &[("app", &info.name)]));
        ui.add_space(6.0);
        header(ui, info, tr);
        ui.add_space(8.0);
        let segments: Vec<crate::pane_ui::Segment> = shown.iter().map(|t| crate::pane_ui::Segment { icon: t.icon(), label: tr(t.name()) }).collect();
        let mut pick = shown.iter().position(|t| Tab::ALL.get(*tab) == Some(t)).unwrap_or(0);
        if crate::pane_ui::segmented_at(ui, id.with("tabs"), &segments, &mut pick) {
            *tab = shown.get(pick).and_then(|t| Tab::ALL.iter().position(|a| a == t)).unwrap_or(0);
        }
        let body = shown.get(pick).copied().unwrap_or(Tab::About);
        egui::ScrollArea::vertical().id_salt(("krub_about_body", pick)).max_height(300.0).auto_shrink([false, true]).show(ui, |ui| {
            ui.set_min_height(220.0);
            match body {
                Tab::About => about(ui, info, tr),
                Tab::Contributors => {
                    credit_rows(ui, info, tr);
                    ui.add_space(6.0);
                    host.contributors(ui);
                }
                Tab::Models => host.models(ui),
                Tab::Licences => licences(ui, info, tr),
            }
        });
        ui.add_space(6.0);
        ui.separator();
        close |= footer(ui, info, tr);
    });
    close || modal.should_close()
}

/// The title and the × that closes the dialog; `true` when the × was clicked.
fn title_row(ui: &mut Ui, title: &str) -> bool {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).font(FontId::proportional(14.0)).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (r, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(r, 4.0, ui.visuals().widgets.hovered.weak_bg_fill);
            }
            crate::paint_named(ui.painter(), r.shrink(3.0), "close", ui.visuals().text_color(), false, false);
            resp.on_hover_text("×").clicked()
        })
        .inner
    })
    .inner
}

/// The logo tile, the name, the version and the one-line description.
fn header(ui: &mut Ui, info: &Info, tr: &dyn Fn(&str) -> String) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(56.0, 56.0), Sense::hover());
        ui.painter().rect_filled(r, 12.0, info.color);
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, info.letter, FontId::proportional(32.0), GOLD);
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new(format!("{} · {}", info.name, info.english)).font(FontId::proportional(20.0)).strong());
            let mut version = fill(&tr("Version {version}"), &[("version", info.version)]);
            if let Some(b) = info.build {
                version = format!("{version} ({b})");
            }
            ui.label(RichText::new(version).weak());
            ui.label(RichText::new(&info.tagline).weak());
        });
    });
}

/// A label and value row of the About tab.
fn row(ui: &mut Ui, label: &str, value: &str) {
    ui.label(RichText::new(label).weak());
    ui.label(value);
    ui.end_row();
}

fn about(ui: &mut Ui, info: &Info, tr: &dyn Fn(&str) -> String) {
    egui::Grid::new("krub_about_rows").num_columns(2).min_col_width(130.0).spacing(vec2(12.0, 6.0)).show(ui, |ui| {
        let mut version = info.version.to_string();
        if let Some(b) = info.build {
            version = format!("{version} ({b})");
        }
        row(ui, &tr("Version"), &version);
        row(ui, &tr("Platform"), &platform(tr));
        row(ui, &tr("Based on"), &fill(&tr("{project} by the ArtCraft team"), &[("project", info.upstream)]));
        row(ui, &tr("Adapted by"), "Chanon Sarasuk");
        row(ui, &tr("Licence"), &tr("MIT or Apache-2.0"));
    });
    ui.add_space(10.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        ui.hyperlink_to("kruboffice.app", SITE_URL);
        ui.hyperlink_to(&info.source_label, info.source_url);
        ui.hyperlink_to(fill(&tr("Original project {project}"), &[("project", info.upstream)]), info.upstream_url);
        ui.hyperlink_to(tr("Report a problem"), REPORT_URL);
    });
}

/// The Contributors tab's own rows, above the app's contributor table.
fn credit_rows(ui: &mut Ui, info: &Info, tr: &dyn Fn(&str) -> String) {
    egui::Grid::new("krub_about_credits").num_columns(2).min_col_width(130.0).spacing(vec2(12.0, 6.0)).show(ui, |ui| {
        row(ui, &tr("Adapted for Thai"), "Chanon Sarasuk");
        row(ui, &tr("Original project"), &fill(&tr("The ArtCraft team and the {project} contributors"), &[("project", info.upstream)]));
        for (role, names) in &info.credits {
            row(ui, role, names);
        }
    });
}

fn licences(ui: &mut Ui, info: &Info, tr: &dyn Fn(&str) -> String) {
    ui.label(RichText::new(format!("{} · {}", info.name, tr("MIT or Apache-2.0"))).strong());
    ui.label(
        RichText::new(format!(
            "Copyright (c) 2026 ArtCraft Team and the {} contributors. Modifications copyright (c) 2026 Chanon Sarasuk.",
            info.upstream
        ))
        .small()
        .weak(),
    );
    ui.add_space(8.0);
    crate::tabler_credit(ui, &tr("Icons: Tabler Icons by Paweł Kuna"), "MIT License");
    ui.add_space(8.0);
    ui.label(
        RichText::new(tr("Fonts and the other open-source parts are listed in the NOTICE and ATTRIBUTION files that come with the program.")).weak(),
    );
}

/// The copyright line and the Close button in the app's colour; `true` when Close was clicked.
fn footer(ui: &mut Ui, info: &Info, tr: &dyn Fn(&str) -> String) -> bool {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("© 2026 ArtCraft · {} · Chanon Sarasuk", info.upstream)).small().weak());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close = egui::Button::new(RichText::new(tr("Close")).color(Color32::WHITE)).fill(info.color).min_size(vec2(84.0, 28.0));
            let resp = ui.add(close);
            // A thin ring so the button reads on a dark theme too.
            ui.painter().rect_stroke(resp.rect, 4.0, Stroke::new(1.0, info.color.gamma_multiply(0.8)), egui::StrokeKind::Inside);
            resp.clicked()
        })
        .inner
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoCredits;
    impl Host for NoCredits {
        fn contributors(&mut self, _: &mut Ui) {}
        fn models(&mut self, _: &mut Ui) {}
        fn has_credits(&self) -> bool {
            false
        }
    }

    struct Credits(u32);
    impl Host for Credits {
        fn contributors(&mut self, ui: &mut Ui) {
            self.0 += 1;
            ui.label("table");
        }
        fn models(&mut self, ui: &mut Ui) {
            self.0 += 10;
            ui.label("models");
        }
    }

    fn info() -> Info<'static> {
        Info {
            name: "เขียนครับ".into(),
            english: "KhianKrub",
            letter: "ข",
            color: Color32::from_rgb(0x1E, 0x3A, 0x6E),
            version: "0.17.0",
            build: None,
            tagline: "A word processor".into(),
            upstream: "WordCraft",
            upstream_url: "https://github.com/storytold/wordcraft",
            source_label: "KrubOffice on GitHub".into(),
            source_url: "https://github.com/chanonss/kruboffice-desktop",
            credits: vec![("Patches".into(), "KhianKrub Plus".into())],
        }
    }

    fn frame(ctx: &egui::Context, tab: &mut usize, host: &mut dyn Host) -> bool {
        let mut closed = false;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| closed = show(ui.ctx(), &info(), tab, &|s: &str| s.to_string(), host));
        out.textures_delta.clear();
        closed
    }

    #[test]
    fn the_credit_tabs_show_only_where_the_build_has_them() {
        assert_eq!(tabs(&NoCredits), [Tab::About, Tab::Licences]);
        assert_eq!(tabs(&Credits(0)), Tab::ALL);
        for t in Tab::ALL {
            assert!(NAMES.contains(&t.name()), "{t:?}");
        }
    }

    #[test]
    fn the_open_tab_draws_its_table() {
        let ctx = egui::Context::default();
        let mut host = Credits(0);
        let mut tab = 1;
        assert!(!frame(&ctx, &mut tab, &mut host));
        assert_eq!(host.0, 1, "Contributors drew the app's table");
        tab = 2;
        let _ = frame(&ctx, &mut tab, &mut host);
        assert_eq!(host.0, 11, "AI models drew the app's model table");
        // Without the credit tabs, a request for one shows About.
        let mut tab = 1;
        let _ = frame(&ctx, &mut tab, &mut NoCredits);
        assert_eq!(tab, 1, "the app's choice is kept for a build that has the tab");
    }

    #[test]
    fn placeholders_and_the_platform_read_right() {
        assert_eq!(fill("About {app}", &[("app", "คิดครับ")]), "About คิดครับ");
        let p = platform(&|s: &str| s.to_string());
        assert!(!p.is_empty());
    }
}
