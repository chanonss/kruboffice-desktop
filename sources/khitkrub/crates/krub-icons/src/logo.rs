//! The KrubOffice apps' logos (owner, 2026-11-21: the start screens' letter tiles were "not our
//! logo"): the Quiet Glass app icons, the same drawings as each app's favicon, window and install
//! icon (`assets/app-icon/<app>-512.png` in its repository), scaled to 128 px and kept here so the
//! start screen's top bar, the app switcher and the About dialog draw the real logo in every app.

use egui::{Color32, Painter, Rect, TextureHandle, TextureOptions, pos2};

/// One of the KrubOffice apps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum App {
    /// เขียนครับ, documents.
    KhianKrub,
    /// คิดครับ, spreadsheets.
    KhitKrub,
    /// พูดครับ, presentations.
    PhutKrub,
}

impl App {
    pub const ALL: [App; 3] = [App::KhianKrub, App::KhitKrub, App::PhutKrub];

    /// The logo, a 128 px PNG.
    fn png(self) -> &'static [u8] {
        match self {
            App::KhianKrub => include_bytes!("../assets/khiankrub-128.png"),
            App::KhitKrub => include_bytes!("../assets/khitkrub-128.png"),
            App::PhutKrub => include_bytes!("../assets/phutkrub-128.png"),
        }
    }

    fn name(self) -> &'static str {
        match self {
            App::KhianKrub => "khiankrub",
            App::KhitKrub => "khitkrub",
            App::PhutKrub => "phutkrub",
        }
    }

    /// The logo's tile colour and letter, drawn only if its picture can't be read.
    fn fallback(self) -> (Color32, &'static str) {
        match self {
            App::KhianKrub => (Color32::from_rgb(0x1E, 0x3A, 0x8A), "ข"),
            App::KhitKrub => (Color32::from_rgb(0x0F, 0x6E, 0x56), "ค"),
            App::PhutKrub => (Color32::from_rgb(0x9A, 0x3A, 0x1F), "พ"),
        }
    }
}

/// The logo's texture, decoded once per context (None if the picture can't be read).
fn texture(ctx: &egui::Context, app: App) -> Option<TextureHandle> {
    let id = crate::pane_ui::id(("krub_icons_logo", app));
    if let Some(cached) = ctx.data_mut(|d| d.get_temp::<Option<TextureHandle>>(id)) {
        return cached;
    }
    let decoded = image::load_from_memory_with_format(app.png(), image::ImageFormat::Png).ok().map(|img| {
        let rgba = img.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw())
    });
    // Mipmaps keep the 128 px picture smooth at the 30-point size of the app switcher.
    let options = TextureOptions { mipmap_mode: Some(egui::TextureFilter::Linear), ..TextureOptions::LINEAR };
    let handle = decoded.map(|image| ctx.load_texture(format!("krub-logo-{}", app.name()), image, options));
    ctx.data_mut(|d| d.insert_temp(id, handle.clone()));
    handle
}

/// Draw `app`'s logo filling `rect` (a square), `opacity` 0–1 (1 is as drawn; less fades it, as for
/// an app that isn't out yet).
pub fn paint(painter: &Painter, rect: Rect, app: App, opacity: f32) {
    let tint = Color32::WHITE.gamma_multiply(opacity.clamp(0.0, 1.0));
    if let Some(tex) = texture(painter.ctx(), app) {
        painter.image(tex.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tint);
    } else {
        let (back, letter) = app.fallback();
        painter.rect_filled(rect, rect.width() * 0.21, back.gamma_multiply(opacity));
        let font = egui::FontId::proportional(rect.height() * 0.6);
        crate::glass_letter(painter, rect.center(), letter, font, tint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_logo_decodes_as_a_square_picture() {
        for app in App::ALL {
            let img = image::load_from_memory_with_format(app.png(), image::ImageFormat::Png);
            assert!(img.as_ref().is_ok_and(|i| i.width() == 128 && i.height() == 128), "{app:?}");
        }
    }

    #[test]
    fn painting_a_logo_uploads_its_picture_once() {
        let ctx = egui::Context::default();
        let mut shapes = 0;
        for _ in 0..2 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let r = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(32.0, 32.0));
                for app in App::ALL {
                    paint(ui.painter(), r, app, 1.0);
                }
            });
            shapes = out.shapes.len();
            // A headless frame hands its texture uploads to no renderer.
            out.textures_delta.clear();
        }
        assert!(shapes >= 3, "each logo is drawn");
        for app in App::ALL {
            let id = crate::pane_ui::id(("krub_icons_logo", app));
            assert!(ctx.data_mut(|d| d.get_temp::<Option<TextureHandle>>(id)).flatten().is_some(), "{app:?} decoded");
        }
    }
}
