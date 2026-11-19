//! KrubOffice for the desktop: the launcher.
//!
//! Usage: `kruboffice [FILE...]`
//!
//! - With files (a double-click on Windows, `kruboffice งาน.docx`), each file opens in the program
//!   that handles it (KhianKrub, KhitKrub or PhutKrub) and the launcher exits.
//! - On macOS, Finder sends files as an open-documents event instead; the launcher waits a moment
//!   for it before deciding whether to show the Start Center.
//! - Otherwise it shows the Start Center: the three programs, Open File and recent files.
//!
//! `kruboffice --version` prints the version; `--help` this usage.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
#![forbid(unsafe_code)]
// No console window behind the Start Center on Windows.
#![cfg_attr(windows, windows_subsystem = "windows")]

use eframe::egui;

#[cfg(any(target_os = "windows", test))]
mod graphics;
use kruboffice::{dispatch, fonts, start, updates};

const USAGE: &str = "Usage: kruboffice [FILE...]\n\nOpens each file in KhianKrub (documents), KhitKrub (spreadsheets) or PhutKrub (presentations).\nWith no file, shows the KrubOffice Start Center.";

/// The app's resources folder (macOS bundle `Contents/Resources`, else the launcher's folder).
fn resources_dir() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let here = exe.parent()?;
    if here.ends_with("Contents/MacOS") { here.parent().map(|c| c.join("Resources")) } else { Some(here.to_path_buf()) }
}

/// The KrubOffice logo (three tiles) for the window and taskbar; egui's own icon otherwise. On
/// macOS inside the .app the Dock keeps the bundle's own icon: setting another picture at run
/// time made the icon change as the app opened (owner, 2026-11-16). An empty icon tells eframe to
/// leave it alone.
fn app_icon() -> egui::IconData {
    if cfg!(target_os = "macos") && in_app_bundle() {
        return egui::IconData::default();
    }
    let png: &[u8] = include_bytes!("../packaging/windows/kruboffice-512.png");
    match image::load_from_memory_with_format(png, image::ImageFormat::Png) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            egui::IconData { width: rgba.width(), height: rgba.height(), rgba: rgba.into_raw() }
        }
        Err(e) => {
            log::warn!("app icon: {e}");
            egui::IconData::default()
        }
    }
}

/// Whether this program runs from inside a macOS .app bundle (`….app/Contents/MacOS/…`).
fn in_app_bundle() -> bool {
    std::env::current_exe().is_ok_and(|p| p.to_string_lossy().contains(".app/Contents/MacOS/"))
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("KrubOffice {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    // For packaging: each program and the extensions KrubOffice registers for it.
    if args.iter().any(|a| a == "--list-types") {
        for app in dispatch::App::ALL {
            println!("{}\t{}", app.name(), app.extensions().join(","));
        }
        return;
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return;
    }
    let install = dispatch::Installation::locate();
    let files = dispatch::file_args(args);

    // Files on the command line: hand them over without a window when every one has a program.
    if !files.is_empty()
        && let Some(install) = &install
    {
        let (groups, unknown) = dispatch::group(files.clone());
        if unknown.is_empty() && groups.iter().all(|(app, _)| install.has(*app)) {
            let mut ok = true;
            for (app, list) in groups {
                if let Err(e) = install.launch(app, &list) {
                    log::error!("{e}");
                    ok = false;
                }
            }
            if ok {
                return;
            }
        }
    }

    // macOS: register for Finder's open-documents event before the event loop starts, so the
    // event that launched us is caught too.
    #[cfg(target_os = "macos")]
    let (_registration, inbox) = fmv_macos_events::Registration::install();

    // Changed only on Windows (`graphics::configure` below).
    #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("KrubOffice")
            .with_app_id("app.kruboffice.desktop")
            .with_icon(app_icon())
            .with_inner_size([820.0, 680.0])
            .with_min_inner_size([560.0, 520.0])
            .with_drag_and_drop(true)
            // Shown once it's clear the launcher wasn't started just to open a file.
            .with_visible(false),
        ..Default::default()
    };
    // Before eframe creates the wgpu instance (issue #4): on Windows, DirectX 12 only when it has a
    // usable GPU, so AMD's OpenGL driver is never loaded; OpenGL where DirectX 12 has none or on
    // 32-bit builds; DirectX 12 shaders compiled with FXC. `WGPU_BACKEND` still overrides.
    #[cfg(target_os = "windows")]
    graphics::configure(&mut options, eframe::wgpu::Backends::from_env());
    let resources = resources_dir();
    let result = eframe::run_native(
        "KrubOffice",
        options,
        Box::new(move |cc| {
            fonts::install(&cc.egui_ctx, resources.as_deref());
            let lang = kruboffice::recent::settings_root().map(|r| updates::Lang::of_programs(&r)).unwrap_or(updates::Lang::Thai);
            let app = start::StartCenter::new(install, files).with_resources(resources.clone()).with_updates(updates::Updates::load(), lang);
            #[cfg(target_os = "macos")]
            let app = {
                let ctx = cc.egui_ctx.clone();
                inbox.set_wake(move || ctx.request_repaint());
                app.with_events(inbox)
            };
            Ok(Box::new(app))
        }),
    );
    if let Err(e) = result {
        log::error!("KrubOffice couldn't open its window: {e}");
        eprintln!("KrubOffice couldn't open its window: {e}");
        std::process::exit(1);
    }
}
