//! Headless screenshots of the Start Center (offscreen wgpu, so it works with a locked screen).
//!
//! `cargo run --example shot -- OUT_DIR [FILE…]`: writes `start-light.png`, `start-dark.png` and
//! `start-narrow.png`; the FILEs are shown as recent files.
use std::path::PathBuf;

use eframe::egui;
use kruboffice::dispatch::{App, app_for};
use kruboffice::recent::Recent;
use kruboffice::start::StartCenter;

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
    for (name, w, h, dark) in [("start-light", 820.0, 680.0, false), ("start-dark", 820.0, 680.0, true), ("start-narrow", 560.0, 660.0, false)] {
        let resources = std::env::var_os("KRUBOFFICE_RESOURCES").map(PathBuf::from);
        let mut app = StartCenter::new(None, Vec::new()).with_resources(resources.clone());
        app.show_with(recent.clone());
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
