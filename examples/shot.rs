//! Headless screenshots of the Start Center (offscreen wgpu, so it works with a locked screen).
//!
//! `cargo run --example shot -- OUT_DIR [FILE…]`: writes `start-light.png`, `start-dark.png` and
//! `start-narrow.png`, `settings.png` and `about.png`; the FILEs are shown as recent files (the
//! first one pinned). With `KRUBOFFICE_SHOT_UPDATES=1` it
//! also writes the update check's states: `update-question`, `update-available`,
//! `update-available-dark`, `update-latest-en` and `update-narrow`.
use std::path::PathBuf;

use eframe::egui;
use kruboffice::dispatch::{App, app_for};
use kruboffice::recent::Recent;
use kruboffice::start::{StartCenter, Window};
use kruboffice::updates::{Lang, Platform, Settings, Updates};

/// The update check in one of its states, with no network.
fn updates(state: &str) -> Option<(Updates, Lang)> {
    let offline = || -> kruboffice::updates::Fetch { Box::new(|| std::sync::mpsc::channel().1) };
    let on = Settings { check: Some(true), checked_on: kruboffice::updates::today(), skipped: String::new() };
    let feed = |v: &str| {
        serde_json::json!({"version": v, "files": {
            "macos": {"url": format!("https://github.com/chanonss/kruboffice-desktop/releases/download/v{v}/a.dmg")},
            "windows": {"url": format!("https://github.com/chanonss/kruboffice-desktop/releases/download/v{v}/a.msi")},
            "linux": {"url": format!("https://github.com/chanonss/kruboffice-desktop/releases/download/v{v}/a.deb")}}})
    };
    match state {
        "question" => Some((Updates::new(Settings::default(), None, offline(), Platform::this()), Lang::Thai)),
        "available" => {
            let mut u = Updates::new(on, None, offline(), Platform::this());
            u.take_answer(Ok(feed("0.5.0")), false);
            Some((u, Lang::Thai))
        }
        "latest-en" => {
            let mut u = Updates::new(on, None, offline(), Platform::this());
            u.take_answer(Ok(feed(kruboffice::updates::current())), true);
            Some((u, Lang::English))
        }
        _ => None,
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(out) = args.next().map(PathBuf::from) else {
        eprintln!("usage: shot OUT_DIR [FILE…]");
        std::process::exit(2);
    };
    let recent: Vec<Recent> = args
        .map(PathBuf::from)
        .filter_map(|p| {
            let app = app_for(&p)?;
            let modified = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
            Some(Recent { path: p, app, modified })
        })
        .collect();
    let mut shots = vec![
        ("start-light", 1040.0, 650.0, false, ""),
        ("start-dark", 1040.0, 650.0, true, ""),
        ("start-narrow", 640.0, 440.0, false, ""),
        ("settings", 1040.0, 650.0, false, "available"),
        ("about", 1040.0, 650.0, false, ""),
    ];
    if std::env::var_os("KRUBOFFICE_SHOT_UPDATES").is_some() {
        shots.extend([
            ("update-question", 1040.0, 650.0, false, "question"),
            ("update-available", 1040.0, 650.0, false, "available"),
            ("update-available-dark", 1040.0, 650.0, true, "available"),
            ("update-latest-en", 1040.0, 650.0, false, "latest-en"),
            ("update-narrow", 640.0, 440.0, false, "question"),
        ]);
    }
    for (name, w, h, dark, state) in shots {
        let resources = std::env::var_os("KRUBOFFICE_RESOURCES").map(PathBuf::from);
        let mut app = StartCenter::new(None, Vec::new()).with_resources(resources.clone());
        if let Some((u, lang)) = updates(state) {
            app = app.with_updates(u, lang);
        }
        app.show_with(recent.clone());
        app.pin_for_show(recent.first().map(|r| vec![r.path.clone()]).unwrap_or_default());
        app.open_window(match name {
            "settings" => Some(Window::Settings),
            "about" => Some(Window::About),
            _ => None,
        });
        let mut fonts_done = false;
        let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(w, h)).with_pixels_per_point(2.0).wgpu().build_ui_state(
            move |ui, app: &mut StartCenter| {
                if !fonts_done {
                    kruboffice::fonts::install(ui.ctx(), resources.as_deref());
                    ui.ctx().set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
                    fonts_done = true;
                }
                app.draw(ui);
            },
            app,
        );
        for _ in 0..6 {
            harness.step();
        }
        match harness.render() {
            Ok(img) => match img.save(out.join(format!("{name}.png"))) {
                Ok(()) => println!("{}", out.join(format!("{name}.png")).display()),
                Err(e) => eprintln!("{name}: {e}"),
            },
            Err(e) => eprintln!("{name}: render failed: {e}"),
        }
    }
    let _ = App::ALL;
}
