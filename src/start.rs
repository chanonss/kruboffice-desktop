//! The Start Center: what KrubOffice shows when it's opened without a file. The same welcome as
//! kruboffice.app ("วันนี้ ...อยากทำอะไรดีครับ?"), the three programs, Open File and the recent
//! files of all three.

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use eframe::egui::{self, Align2, Color32, CornerRadius, FontFamily, FontId, RichText, Sense, Stroke, vec2};

use crate::dispatch::{self, App, Installation};
use crate::fonts::BOLD;
use crate::recent::{self, Recent};

const NAVY: Color32 = Color32::from_rgb(0x1e, 0x3a, 0x8a);
const TEAL: Color32 = Color32::from_rgb(0x0f, 0x6e, 0x56);
const CORAL: Color32 = Color32::from_rgb(0x9a, 0x3a, 0x1f);
const GOLD: Color32 = Color32::from_rgb(0xf2, 0xb7, 0x05);

fn colour(app: App) -> Color32 {
    match app {
        App::KhianKrub => NAVY,
        App::KhitKrub => TEAL,
        App::PhutKrub => CORAL,
    }
}

fn letter(app: App) -> &'static str {
    match app {
        App::KhianKrub => "ข",
        App::KhitKrub => "ค",
        App::PhutKrub => "พ",
    }
}

fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(BOLD.into()))
}

/// Page colours for the light and dark themes.
struct Palette {
    bg: Color32,
    text: Color32,
    sub: Color32,
    accent: Color32,
    card: Color32,
}

impl Palette {
    fn of(ctx: &egui::Context) -> Palette {
        if ctx.global_style().visuals.dark_mode {
            Palette {
                bg: Color32::from_rgb(0x14, 0x16, 0x1f),
                text: Color32::from_rgb(0xf4, 0xf1, 0xea),
                sub: Color32::from_rgb(0xaa, 0xb3, 0xcf),
                accent: GOLD,
                card: Color32::from_rgb(0x1d, 0x20, 0x2c),
            }
        } else {
            Palette {
                bg: Color32::from_rgb(0xfb, 0xf8, 0xf1),
                text: NAVY,
                sub: Color32::from_rgb(0x4a, 0x55, 0x78),
                accent: Color32::from_rgb(0xb8, 0x86, 0x0b),
                card: Color32::WHITE,
            }
        }
    }
}

/// How long a just-started launcher waits for the open-documents event that started it (a
/// double-clicked file on macOS) before it shows the Start Center.
pub const LAUNCH_GRACE: Duration = Duration::from_millis(600);

/// The three apps' icons (their own glass artwork, placed in `icons/` by the packaging).
#[derive(Default)]
struct Icons {
    loaded: bool,
    textures: Vec<(App, egui::TextureHandle)>,
}

/// The largest icon file read.
const MAX_ICON_BYTES: u64 = 4 * 1024 * 1024;

fn load_icon(ctx: &egui::Context, path: &std::path::Path, name: &str) -> Option<egui::TextureHandle> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_ICON_BYTES {
        return None;
    }
    let img = image::load_from_memory_with_format(&std::fs::read(path).ok()?, image::ImageFormat::Png).ok()?;
    if img.width() > 1024 || img.height() > 1024 {
        return None;
    }
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Some(ctx.load_texture(name, color, egui::TextureOptions::LINEAR))
}

pub struct StartCenter {
    install: Option<Installation>,
    resources: Option<PathBuf>,
    icons: Icons,
    recent: Vec<Recent>,
    status: Option<(String, Instant)>,
    started: Instant,
    shown: bool,
    /// Files waiting to be opened (from the command line, drops or Apple events).
    pending: Vec<PathBuf>,
    #[cfg(target_os = "macos")]
    events: Option<fmv_macos_events::Inbox>,
}

impl StartCenter {
    pub fn new(install: Option<Installation>, initial: Vec<PathBuf>) -> StartCenter {
        StartCenter {
            install,
            resources: None,
            icons: Icons::default(),
            recent: Vec::new(),
            status: None,
            started: Instant::now(),
            shown: false,
            pending: initial,
            #[cfg(target_os = "macos")]
            events: None,
        }
    }

    /// The app's resources folder, where the packaging puts `icons/` and `fonts/`.
    pub fn with_resources(mut self, dir: Option<PathBuf>) -> StartCenter {
        self.resources = dir;
        self
    }

    fn icon(&mut self, ctx: &egui::Context, app: App) -> Option<egui::TextureHandle> {
        if !self.icons.loaded {
            self.icons.loaded = true;
            if let Some(dir) = &self.resources {
                for a in App::ALL {
                    let path = dir.join("icons").join(format!("{}-192.png", a.name().to_ascii_lowercase()));
                    if let Some(t) = load_icon(ctx, &path, a.name()) {
                        self.icons.textures.push((a, t));
                    }
                }
            }
        }
        self.icons.textures.iter().find(|(a, _)| *a == app).map(|(_, t)| t.clone())
    }

    #[cfg(target_os = "macos")]
    pub fn with_events(mut self, inbox: fmv_macos_events::Inbox) -> StartCenter {
        self.events = Some(inbox);
        self
    }

    fn refresh_recent(&mut self) {
        self.recent = recent::settings_root().map(|root| recent::collect(&root, 12)).unwrap_or_default();
    }

    fn say(&mut self, msg: impl Into<String>) {
        self.status = Some((msg.into(), Instant::now()));
    }

    /// Open files in their programs. Returns whether every file was handed over.
    fn open(&mut self, files: Vec<PathBuf>) -> bool {
        let (groups, unknown) = dispatch::group(files);
        let mut ok = unknown.is_empty();
        let Some(install) = self.install.clone() else {
            self.say("ไม่พบโปรแกรมของ KrubOffice ในเครื่องนี้ ลองติดตั้งใหม่อีกครั้งครับ");
            return false;
        };
        for (app, files) in groups {
            if let Err(e) = install.launch(app, &files) {
                log::error!("{e}");
                self.say(format!("เปิด{}ไม่ได้: {e}", app.thai_name()));
                ok = false;
            }
        }
        if let Some(f) = unknown.first() {
            let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            self.say(format!("KrubOffice ยังเปิดไฟล์ชนิดนี้ไม่ได้: {name}"));
        }
        ok
    }

    fn launch_app(&mut self, app: App) {
        let Some(install) = self.install.clone() else {
            self.say("ไม่พบโปรแกรมของ KrubOffice ในเครื่องนี้ ลองติดตั้งใหม่อีกครั้งครับ");
            return;
        };
        if let Err(e) = install.launch(app, &[]) {
            log::error!("{e}");
            self.say(format!("เปิด{}ไม่ได้", app.thai_name()));
        }
    }

    fn pick_and_open(&mut self) {
        let mut all: Vec<&str> = Vec::new();
        for app in App::ALL {
            all.extend(app.extensions());
        }
        let mut dialog = rfd::FileDialog::new().set_title("เปิดไฟล์").add_filter("เอกสาร ตารางคำนวณ และงานนำเสนอ", &all);
        for app in App::ALL {
            dialog = dialog.add_filter(format!("{} ({})", app.thai_name(), app.thai_kind()), app.extensions());
        }
        if let Some(files) = dialog.pick_files() {
            self.open(files);
        }
    }

    /// Collect files from Apple events and drops into `pending`.
    fn gather(&mut self, ctx: &egui::Context) {
        #[cfg(target_os = "macos")]
        if let Some(inbox) = &self.events {
            for e in inbox.drain() {
                match e {
                    fmv_macos_events::Event::Open(paths) => self.pending.extend(paths.into_iter().take(256)),
                    fmv_macos_events::Event::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                }
            }
        }
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).take(256).collect());
        self.pending.extend(dropped);
    }
}

fn app_tile(ui: &mut egui::Ui, app: App, p: &Palette, texture: Option<&egui::TextureHandle>) -> bool {
    let size = vec2(150.0, 170.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter_at(rect);
    let icon = egui::Rect::from_center_size(rect.center_top() + vec2(0.0, 58.0), vec2(96.0, 96.0));
    if resp.hovered() || resp.has_focus() {
        painter.rect_stroke(icon.expand(5.0), CornerRadius::same(26), Stroke::new(3.0, colour(app)), egui::StrokeKind::Outside);
    }
    match texture {
        Some(t) => {
            painter.image(t.id(), icon, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
        }
        None => {
            painter.rect_filled(icon, CornerRadius::same(22), colour(app));
            painter.text(icon.center() + vec2(0.0, 2.0), Align2::CENTER_CENTER, letter(app), bold(58.0), GOLD);
        }
    }
    painter.text(rect.center_top() + vec2(0.0, 126.0), Align2::CENTER_CENTER, app.thai_name(), bold(17.0), p.text);
    painter.text(
        rect.center_top() + vec2(0.0, 150.0),
        Align2::CENTER_CENTER,
        format!("{} · {}", app.name(), app.thai_kind()),
        FontId::proportional(12.5),
        p.sub,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// Text on one line, cut with "…" when it's wider than `width`.
fn one_line(painter: &egui::Painter, text: String, font: FontId, color: Color32, width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(text, font, color, width);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    painter.layout_job(job)
}

/// A folder with the home folder written as `~`.
fn short_folder(folder: &str) -> String {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
    match folder.strip_prefix(home.as_str()) {
        Some(rest) if !home.is_empty() => format!("~{rest}"),
        _ => folder.to_string(),
    }
}

fn when(t: Option<SystemTime>) -> String {
    let Some(t) = t else { return String::new() };
    let Ok(age) = SystemTime::now().duration_since(t) else { return "เมื่อสักครู่".into() };
    let mins = age.as_secs() / 60;
    match mins {
        0 => "เมื่อสักครู่".into(),
        1..=59 => format!("{mins} นาทีที่แล้ว"),
        60..=1439 => format!("{} ชั่วโมงที่แล้ว", mins / 60),
        _ => format!("{} วันที่แล้ว", mins / 1440),
    }
}

impl eframe::App for StartCenter {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.gather(ctx);
        if !self.pending.is_empty() {
            let files = std::mem::take(&mut self.pending);
            let all_ok = self.open(files);
            // Started only to open a file: hand it over and go, without showing the Start Center.
            if !self.shown && all_ok {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }
        if !self.shown {
            if self.started.elapsed() < LAUNCH_GRACE && self.status.is_none() {
                ctx.request_repaint_after(Duration::from_millis(50));
                return;
            }
            self.shown = true;
            self.refresh_recent();
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        if ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::WindowFocused(true)))) {
            self.refresh_recent();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.shown {
            self.draw(ui);
        }
    }
}

impl StartCenter {
    /// Show the Start Center now with these recent files (screenshots and tests).
    pub fn show_with(&mut self, recent: Vec<Recent>) {
        self.shown = true;
        self.recent = recent;
    }

    /// Draw the Start Center.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        // Fonts set with `set_fonts` take effect from the next frame; drawing with the bold family
        // before then would panic inside egui, so wait a frame instead.
        if !ctx.fonts(|f| f.families().contains(&FontFamily::Name(BOLD.into()))) {
            ctx.request_repaint();
            return;
        }
        let p = Palette::of(&ctx);
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(p.bg)).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(36.0);
                    ui.horizontal(|ui| {
                        let width = 220.0;
                        ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
                        let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                        let painter = ui.painter_at(r.expand(4.0));
                        let tile = |dx: f32, dy: f32, c: Color32| {
                            painter.rect_filled(egui::Rect::from_min_size(r.min + vec2(dx, dy), vec2(26.0, 26.0)), CornerRadius::same(6), c)
                        };
                        tile(2.0, 9.0, CORAL);
                        tile(12.0, 2.0, TEAL);
                        tile(6.0, 12.0, NAVY);
                        painter.rect_filled(egui::Rect::from_min_size(r.min + vec2(12.0, 23.0), vec2(14.0, 5.0)), CornerRadius::same(2), GOLD);
                        ui.label(RichText::new("Krub").font(bold(26.0)).color(p.text));
                        ui.add_space(-ui.spacing().item_spacing.x);
                        ui.label(RichText::new("Office").font(bold(26.0)).color(p.accent));
                    });
                    ui.add_space(10.0);
                    ui.label(RichText::new("วันนี้ ...อยากทำอะไรดีครับ?").font(bold(24.0)).color(p.text));
                    ui.label(RichText::new("ไฟล์ของท่านอยู่ในเครื่องของท่าน ไม่ได้อยู่ที่ผมครับ").size(13.0).color(p.sub));
                    ui.add_space(26.0);
                    ui.horizontal(|ui| {
                        // Up to 36 pt between the tiles, less in a narrow window.
                        let gap = ((ui.available_width() - 3.0 * 150.0) / 2.0).clamp(4.0, 36.0);
                        let width = 3.0 * 150.0 + 2.0 * gap;
                        ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
                        ui.spacing_mut().item_spacing.x = gap;
                        for app in [App::KhianKrub, App::KhitKrub, App::PhutKrub] {
                            let texture = self.icon(&ctx, app);
                            if app_tile(ui, app, &p, texture.as_ref()) {
                                self.launch_app(app);
                            }
                        }
                    });
                    ui.add_space(18.0);
                    let open = egui::Button::new(RichText::new("เปิดไฟล์…").size(15.0).color(p.text))
                        .fill(p.card)
                        .stroke(Stroke::new(1.0, p.sub.gamma_multiply(0.4)))
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(160.0, 34.0));
                    if ui.add(open).clicked() {
                        self.pick_and_open();
                    }
                    ui.add_space(4.0);
                    ui.label(RichText::new("หรือลากไฟล์มาวางที่หน้าต่างนี้").size(12.0).color(p.sub));
                    if let Some((msg, at)) = &self.status
                        && at.elapsed() < Duration::from_secs(8)
                    {
                        ui.add_space(8.0);
                        ui.label(RichText::new(msg).size(13.0).color(Color32::from_rgb(0xb4, 0x3c, 0x1e)));
                        ctx.request_repaint_after(Duration::from_secs(1));
                    }
                    ui.add_space(26.0);
                });
                let column = 560.0_f32.min(ui.available_width() - 32.0);
                ui.horizontal(|ui| {
                    ui.add_space(((ui.available_width() - column) / 2.0).max(0.0));
                    ui.vertical(|ui| {
                        ui.set_width(column);
                        ui.label(RichText::new("ไฟล์ล่าสุด").font(bold(16.0)).color(p.text));
                        ui.add_space(4.0);
                        if self.recent.is_empty() {
                            ui.label(RichText::new("ไฟล์ที่เปิดล่าสุดจากทั้งสามโปรแกรมจะแสดงที่นี่").size(13.0).color(p.sub));
                        }
                        let mut clicked = None;
                        let icons = self.icons.textures.clone();
                        for r in &self.recent {
                            let (rect, resp) = ui.allocate_exact_size(vec2(column, 44.0), Sense::click());
                            let painter = ui.painter_at(rect);
                            if resp.hovered() {
                                painter.rect_filled(rect, CornerRadius::same(8), p.card);
                            }
                            let badge = egui::Rect::from_min_size(rect.min + vec2(8.0, 8.0), vec2(28.0, 28.0));
                            match icons.iter().find(|(a, _)| *a == r.app) {
                                Some((_, t)) => {
                                    painter.image(
                                        t.id(),
                                        badge,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        Color32::WHITE,
                                    );
                                }
                                None => {
                                    painter.rect_filled(badge, CornerRadius::same(7), colour(r.app));
                                    painter.text(badge.center(), Align2::CENTER_CENTER, letter(r.app), bold(17.0), GOLD);
                                }
                            }
                            let name = r.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                            let folder = r.path.parent().map(|d| d.display().to_string()).unwrap_or_default();
                            let age = painter.layout_no_wrap(when(r.modified), FontId::proportional(11.5), p.sub);
                            let room = (column - 46.0 - age.size().x - 20.0).max(40.0);
                            painter.galley(rect.min + vec2(46.0, 5.0), one_line(&painter, name, FontId::proportional(14.5), p.text, room), p.text);
                            painter.galley(
                                rect.min + vec2(46.0, 25.0),
                                one_line(&painter, short_folder(&folder), FontId::proportional(11.5), p.sub, room),
                                p.sub,
                            );
                            painter.galley(rect.right_center() - vec2(8.0 + age.size().x, age.size().y / 2.0), age, p.sub);
                            if resp.on_hover_text(r.path.display().to_string()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                clicked = Some(r.path.clone());
                            }
                        }
                        if let Some(path) = clicked {
                            self.open(vec![path]);
                        }
                        ui.add_space(20.0);
                        ui.label(RichText::new(format!("KrubOffice {} (รุ่นทดลอง) · ใช้ฟรีครับ", env!("CARGO_PKG_VERSION"))).size(11.5).color(p.sub));
                        ui.add_space(16.0);
                    });
                });
            });
        });
    }
}
