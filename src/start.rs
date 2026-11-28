//! The Start Center: what KrubOffice shows when it's opened without a file (owner, 2026-11-24:
//! draft D "gallery" of `ui-experiments/welcome-screen-2026-11-24`). A top bar with the logo, a
//! button per program, a search box, Settings and About; a strip when a new version is out; then
//! the recent files of all three programs as cards, filtered by kind or pins, and Open File.

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use eframe::egui::{self, Align2, Color32, CornerRadius, FontFamily, FontId, RichText, Sense, Stroke, vec2};

use crate::dispatch::{self, App, Installation};
use crate::fonts::BOLD;
use crate::recent::{self, Recent};
use crate::updates::{self, Lang, Text, Updates};

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
    /// The top bar's and the cards' edges.
    line: Color32,
    /// Under the pointer.
    hover: Color32,
    /// A blank page in the file pictures, the grey around it, and its lines of text.
    paper: Color32,
    well: Color32,
    rule: Color32,
    /// The update strip.
    strip: Color32,
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
                line: Color32::from_rgb(0x34, 0x39, 0x4a),
                hover: Color32::from_rgb(0x2a, 0x2e, 0x3d),
                paper: Color32::from_rgb(0xe9, 0xe7, 0xe1),
                well: Color32::from_rgb(0x26, 0x2a, 0x37),
                rule: Color32::from_rgb(0xc6, 0xc9, 0xd2),
                strip: Color32::from_rgb(0x2a, 0x26, 0x18),
            }
        } else {
            Palette {
                bg: Color32::from_rgb(0xfb, 0xf8, 0xf1),
                text: NAVY,
                sub: Color32::from_rgb(0x4a, 0x55, 0x78),
                accent: Color32::from_rgb(0xb8, 0x86, 0x0b),
                card: Color32::WHITE,
                line: Color32::from_rgb(0xe4, 0xe1, 0xda),
                hover: Color32::from_rgb(0xf1, 0xed, 0xe4),
                paper: Color32::WHITE,
                well: Color32::from_rgb(0xf2, 0xef, 0xe8),
                rule: Color32::from_rgb(0xe0, 0xe3, 0xe9),
                strip: Color32::from_rgb(0xfd, 0xf3, 0xd2),
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
    /// The update check (`None`: none in this window, as in screenshots and tests).
    updates: Option<Updates>,
    /// The language of the update texts.
    lang: Lang,
    /// What the search box holds: recent files whose name contains it are shown.
    search: String,
    /// Which recent files are shown.
    filter: Filter,
    /// The pinned files (KrubOffice's `settings.json`), and where they are kept.
    pins: Vec<PathBuf>,
    pins_path: Option<PathBuf>,
    /// The mouse pointer's arrow (`settings.json`'s `pointer`): the system's or a KrubOffice picture.
    pointer: crate::pointer::Pointer,
    /// The Settings or About window, when open.
    window: Option<Window>,
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
            updates: None,
            lang: Lang::Thai,
            search: String::new(),
            filter: Filter::All,
            pins: Vec::new(),
            pins_path: None,
            pointer: crate::pointer::Pointer::System,
            window: None,
            #[cfg(target_os = "macos")]
            events: None,
        }
    }

    /// The app's resources folder, where the packaging puts `icons/` and `fonts/`.
    pub fn with_resources(mut self, dir: Option<PathBuf>) -> StartCenter {
        self.resources = dir;
        self
    }

    /// Check for updates in this window, with texts in `lang`.
    pub fn with_updates(mut self, updates: Updates, lang: Lang) -> StartCenter {
        self.updates = Some(updates);
        self.lang = lang;
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
        let Some(root) = recent::settings_root() else {
            self.recent.clear();
            return;
        };
        let settings = updates::settings_path(&root);
        self.pins = recent::load_pins(&settings);
        self.pointer = serde_json::from_value(recent::load_key(&settings, "pointer")).unwrap_or_default();
        self.pins_path = Some(settings);
        self.recent = recent::with_pins(recent::collect(&root, MAX_RECENT), &self.pins);
    }

    /// Pin `path`, or unpin it when it's pinned, and keep the choice.
    fn toggle_pin(&mut self, path: &std::path::Path) {
        match self.pins.iter().position(|p| p == path) {
            Some(i) => {
                self.pins.remove(i);
            }
            None if self.pins.len() < recent::MAX_PINS => self.pins.push(path.to_path_buf()),
            None => {
                self.say("ปักหมุดได้ไม่เกิน 200 ไฟล์ครับ");
                return;
            }
        }
        if let Some(settings) = &self.pins_path
            && let Err(e) = recent::save_pins(settings, &self.pins)
        {
            log::error!("{e}");
            self.say("บันทึกการปักหมุดไม่ได้ครับ");
        }
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

/// The corners of a rounded rectangle (`x`, `y`, `w`, `h`, corner radius `r`, in the logo's
/// 120-unit box), turned `deg` degrees about (60, 70), as the logo's SVG draws its cards.
fn card(x: f32, y: f32, w: f32, h: f32, r: f32, deg: f32) -> Vec<egui::Pos2> {
    const STEPS: usize = 8;
    let corners = [(x + w - r, y + r, -90.0_f32), (x + w - r, y + h - r, 0.0), (x + r, y + h - r, 90.0), (x + r, y + r, 180.0)];
    let (sin, cos) = deg.to_radians().sin_cos();
    let mut points = Vec::with_capacity(4 * (STEPS + 1));
    for (cx, cy, start) in corners {
        for i in 0..=STEPS {
            let a = (start + 90.0 * i as f32 / STEPS as f32).to_radians();
            let (px, py) = (cx + r * a.cos() - 60.0, cy + r * a.sin() - 70.0);
            points.push(egui::pos2(60.0 + px * cos - py * sin, 70.0 + px * sin + py * cos));
        }
    }
    points
}

/// The KrubOffice logo in `rect`, drawn from the same shapes as kruboffice.app's and the app
/// icon's: a coral card turned −14°, a teal one turned 10°, the navy card in front with the gold bar.
fn logo(painter: &egui::Painter, rect: egui::Rect) {
    let scale = rect.width().min(rect.height()) / 120.0;
    let place = |points: Vec<egui::Pos2>| -> Vec<egui::Pos2> { points.into_iter().map(|p| rect.min + p.to_vec2() * scale).collect() };
    painter.add(egui::Shape::convex_polygon(place(card(22.0, 26.0, 64.0, 64.0, 15.0, -14.0)), CORAL, Stroke::NONE));
    painter.add(egui::Shape::convex_polygon(place(card(36.0, 22.0, 64.0, 64.0, 15.0, 10.0)), TEAL, Stroke::NONE));
    painter.add(egui::Shape::convex_polygon(place(card(26.0, 38.0, 68.0, 68.0, 16.0, 0.0)), NAVY, Stroke::NONE));
    let bar = egui::Rect::from_min_size(rect.min + vec2(44.0, 66.0) * scale, vec2(32.0, 12.0) * scale);
    painter.rect_filled(bar, CornerRadius::same((6.0 * scale).round() as u8), GOLD);
}

/// The most recent files listed (pinned ones come on top of these).
const MAX_RECENT: usize = 40;

/// Which recent files the Start Center shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    All,
    Kind(App),
    Pinned,
}

/// The small windows the top bar opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Window {
    Settings,
    About,
}

/// The new-file button's word for each program's kind.
fn short_kind(app: App) -> &'static str {
    match app {
        App::KhianKrub => "เอกสาร",
        App::KhitKrub => "ตาราง",
        App::PhutKrub => "งานนำเสนอ",
    }
}

/// A program's icon in `rect`, or its letter on its colour when the icon file isn't there.
fn paint_icon(painter: &egui::Painter, rect: egui::Rect, app: App, texture: Option<&egui::TextureHandle>) {
    match texture {
        Some(t) => {
            painter.image(t.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
        }
        None => {
            let r = (rect.width() * 0.24).round().clamp(2.0, 255.0) as u8;
            painter.rect_filled(rect, CornerRadius::same(r), colour(app));
            painter.text(rect.center(), Align2::CENTER_CENTER, letter(app), bold(rect.height() * 0.6), GOLD);
        }
    }
}

/// A program's new-file button on the top bar: a pill in its colour with its icon and "+ kind".
/// `compact` (a narrow window): the icon and "+" only.
fn new_pill(ui: &mut egui::Ui, app: App, texture: Option<&egui::TextureHandle>, compact: bool) -> egui::Response {
    let font = bold(13.5);
    let text = if compact { "+".to_string() } else { format!("+ {}", short_kind(app)) };
    let galley = ui.painter().layout_no_wrap(text, font, Color32::WHITE);
    let size = vec2(6.0 + 22.0 + 6.0 + galley.size().x + 12.0, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let fill = if resp.hovered() { colour(app).gamma_multiply(0.88) } else { colour(app) };
    ui.painter().rect_filled(rect, CornerRadius::same(16), fill);
    if resp.has_focus() {
        ui.painter().rect_stroke(rect.expand(2.0), CornerRadius::same(18), Stroke::new(2.0, GOLD), egui::StrokeKind::Outside);
    }
    let icon = egui::Rect::from_min_size(rect.min + vec2(6.0, 5.0), vec2(22.0, 22.0));
    paint_icon(ui.painter(), icon, app, texture);
    ui.painter().galley(egui::pos2(icon.max.x + 6.0, rect.center().y - galley.size().y / 2.0), galley, Color32::WHITE);
    named(
        resp.on_hover_text(format!("เปิด{} เพื่อสร้าง{}ใหม่", app.thai_name(), app.thai_kind())).on_hover_cursor(egui::CursorIcon::PointingHand),
        &format!("{}ใหม่", app.thai_kind()),
    )
}

/// `resp` named `label` for assistive technology, as a button.
fn named(resp: egui::Response, label: &str) -> egui::Response {
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    resp
}

/// A square icon button on the top bar (Settings, About), drawn by `paint`.
fn icon_button(ui: &mut egui::Ui, p: &Palette, tip: &str, paint: impl FnOnce(&egui::Painter, egui::Pos2, Color32)) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
    if resp.hovered() || resp.has_focus() {
        ui.painter().rect_filled(rect, CornerRadius::same(8), p.hover);
    }
    paint(ui.painter(), rect.center(), p.sub);
    named(resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand), tip)
}

/// A gear: a ring with eight teeth.
fn paint_gear(painter: &egui::Painter, c: egui::Pos2, color: Color32) {
    let stroke = Stroke::new(1.7, color);
    painter.circle_stroke(c, 5.5, stroke);
    painter.circle_stroke(c, 2.2, stroke);
    for i in 0..8 {
        let a = (i as f32 * 45.0).to_radians();
        let d = vec2(a.cos(), a.sin());
        painter.line_segment([c + d * 5.5, c + d * 8.3], Stroke::new(2.6, color));
    }
}

/// An "i" in a circle.
fn paint_info(painter: &egui::Painter, c: egui::Pos2, color: Color32) {
    painter.circle_stroke(c, 8.0, Stroke::new(1.7, color));
    painter.circle_filled(c + vec2(0.0, -3.6), 1.2, color);
    painter.line_segment([c + vec2(0.0, -0.8), c + vec2(0.0, 4.4)], Stroke::new(1.8, color));
}

/// A filter chip under the heading.
fn chip(ui: &mut egui::Ui, on: bool, text: &str, p: &Palette) -> bool {
    let (fill, color, stroke) = if on { (p.text, p.bg, p.text) } else { (p.card, p.text, p.line) };
    let b = egui::Button::new(RichText::new(text).size(13.0).color(color))
        .fill(fill)
        .stroke(Stroke::new(1.0, stroke))
        .corner_radius(CornerRadius::same(14))
        .min_size(vec2(0.0, 28.0));
    ui.add(b).clicked()
}

/// What a blank file of `app`'s kind looks like, in `rect`: a page of lines, a grid, a slide.
fn paint_kind(painter: &egui::Painter, rect: egui::Rect, app: App, p: &Palette) {
    let paper = p.paper;
    let rule = p.rule;
    match app {
        App::KhianKrub => {
            painter.rect_filled(rect, CornerRadius::same(10), p.well);
            let h = rect.height() - 14.0;
            let page = egui::Rect::from_min_size(egui::pos2(rect.center().x - h * 0.36, rect.min.y + 14.0), vec2(h * 0.72, h + 10.0));
            let page = page.intersect(rect.shrink2(vec2(0.0, 0.0)));
            painter.rect_filled(page, CornerRadius::same(2), paper);
            painter.rect_stroke(page, CornerRadius::same(2), Stroke::new(1.0, p.line), egui::StrokeKind::Inside);
            let inner = page.shrink2(vec2(page.width() * 0.16, 0.0));
            painter.rect_filled(
                egui::Rect::from_min_size(egui::pos2(inner.min.x, page.min.y + 14.0), vec2(inner.width() * 0.6, 4.0)),
                1.0,
                colour(app).gamma_multiply(0.5),
            );
            for i in 0..8 {
                let y = page.min.y + 26.0 + i as f32 * 8.0;
                if y + 3.0 > page.max.y - 4.0 {
                    break;
                }
                let w = if i % 4 == 3 { 0.55 } else { 1.0 };
                painter.rect_filled(egui::Rect::from_min_size(egui::pos2(inner.min.x, y), vec2(inner.width() * w, 3.0)), 1.0, rule);
            }
        }
        App::KhitKrub => {
            painter.rect_filled(rect, CornerRadius::same(10), paper);
            // Inside the rounded corners.
            let clip = painter.with_clip_rect(rect.shrink2(vec2(3.0, 3.0)));
            let head = egui::Rect::from_min_size(rect.min, vec2(rect.width(), 16.0));
            clip.rect_filled(head, CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 }, colour(app).gamma_multiply(0.16));
            let mut y = rect.min.y + 16.0;
            while y < rect.max.y {
                clip.hline(rect.x_range(), y, Stroke::new(1.0, rule));
                y += 15.0;
            }
            let col = (rect.width() / 4.0).max(24.0);
            let mut x = rect.min.x + col * 0.8;
            while x < rect.max.x {
                clip.vline(x, rect.y_range(), Stroke::new(1.0, rule));
                x += col;
            }
        }
        App::PhutKrub => {
            painter.rect_filled(rect, CornerRadius::same(10), p.well);
            let w = rect.width() * 0.84;
            let slide = egui::Rect::from_center_size(rect.center() - vec2(0.0, 4.0), vec2(w, w * 9.0 / 16.0));
            painter.rect_filled(slide, CornerRadius::same(3), paper);
            painter.rect_stroke(slide, CornerRadius::same(3), Stroke::new(1.0, p.line), egui::StrokeKind::Inside);
            painter.rect_filled(
                egui::Rect::from_center_size(slide.center() - vec2(0.0, 6.0), vec2(slide.width() * 0.56, 6.0)),
                2.0,
                colour(app).gamma_multiply(0.45),
            );
            painter.rect_filled(egui::Rect::from_center_size(slide.center() + vec2(0.0, 8.0), vec2(slide.width() * 0.36, 4.0)), 2.0, rule);
        }
    }
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
        if let Some(u) = &mut self.updates
            && u.tick(&updates::today())
        {
            ctx.request_repaint_after(Duration::from_millis(200));
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

    /// Pin these files (screenshots and tests); nothing is saved.
    pub fn pin_for_show(&mut self, pins: Vec<PathBuf>) {
        self.pins = pins;
    }

    /// Open the Settings or About window (screenshots and tests).
    pub fn open_window(&mut self, window: Option<Window>) {
        self.window = window;
    }

    /// The recent files the filter and the search box let through.
    fn shown_files(&self) -> Vec<Recent> {
        let needle = self.search.trim().to_lowercase();
        self.recent
            .iter()
            .filter(|r| match self.filter {
                Filter::All => true,
                Filter::Kind(app) => r.app == app,
                Filter::Pinned => self.pins.contains(&r.path),
            })
            .filter(|r| needle.is_empty() || r.path.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase().contains(&needle)))
            .cloned()
            .collect()
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
        for app in App::ALL {
            let _ = self.icon(&ctx, app);
        }
        let p = Palette::of(&ctx);
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(p.bg)).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            self.top_bar(ui, &p);
            self.update_strip(ui, &p);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                egui::Frame::NONE.inner_margin(egui::Margin { left: 28, right: 28, top: 24, bottom: 18 }).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    self.gallery(ui, &p);
                });
            });
        });
        self.windows(&ctx, &p);
        // Last, once every widget has asked for its cursor: the chosen arrow, if any.
        crate::pointer::paint(&ctx, self.pointer);
    }

    /// Use this pointer without saving it (screenshots and tests).
    pub fn set_pointer(&mut self, pointer: crate::pointer::Pointer) {
        self.pointer = pointer;
    }

    /// The bar across the top: logo, the three new-file buttons, search, Settings and About.
    fn top_bar(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let icons = self.icons.textures.clone();
        let frame = egui::Frame::NONE.fill(p.card).inner_margin(egui::Margin::symmetric(20, 10));
        let bar = frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            // A narrow window: the logo without the word, the new-file buttons with their icons only.
            let compact = ui.available_width() < 700.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
                logo(&ui.painter_at(r), r);
                if !compact {
                    ui.add_space(2.0);
                    ui.label(RichText::new("Krub").font(bold(19.0)).color(p.text));
                    ui.add_space(-ui.spacing().item_spacing.x);
                    ui.label(RichText::new("Office").font(bold(19.0)).color(p.accent));
                }
                ui.add_space(if compact { 6.0 } else { 14.0 });
                for app in App::ALL {
                    let texture = icons.iter().find(|(a, _)| *a == app).map(|(_, t)| t);
                    if new_pill(ui, app, texture, compact).clicked() {
                        self.launch_app(app);
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_button(ui, p, "เกี่ยวกับ KrubOffice", paint_info).clicked() {
                        self.window = Some(Window::About);
                    }
                    if icon_button(ui, p, "ตั้งค่า", paint_gear).clicked() {
                        self.window = Some(Window::Settings);
                    }
                    ui.add_space(4.0);
                    // As wide as there's room for, between 90 and 240 points: a magnifier and
                    // the field in one rounded box.
                    let width = (ui.available_width() - 12.0).clamp(90.0, 240.0);
                    let boxed = egui::Frame::NONE
                        .fill(p.bg)
                        .stroke(Stroke::new(1.0, p.line))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin { left: 8, right: 6, top: 3, bottom: 3 });
                    boxed.show(ui, |ui| {
                        ui.set_width(width);
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                            let c = r.center() - vec2(1.0, 1.0);
                            ui.painter().circle_stroke(c, 5.0, Stroke::new(1.6, p.sub));
                            ui.painter().line_segment([c + vec2(3.6, 3.6), c + vec2(7.0, 7.0)], Stroke::new(1.8, p.sub));
                            let field = egui::TextEdit::singleline(&mut self.search)
                                .hint_text("ค้นหาไฟล์")
                                .frame(egui::Frame::NONE)
                                .desired_width(ui.available_width())
                                .margin(egui::Margin::symmetric(2, 4));
                            ui.add(field).on_hover_text("พิมพ์ชื่อไฟล์ ไฟล์ล่าสุดที่ชื่อตรงกันจะแสดงด้านล่าง");
                        });
                    });
                });
            });
        });
        let r = bar.response.rect;
        ui.painter().hline(r.x_range(), r.max.y, Stroke::new(1.0, p.line));
    }

    /// The recent files as cards, with the heading, the filters and Open File.
    fn gallery(&mut self, ui: &mut egui::Ui, p: &Palette) {
        ui.label(RichText::new("ทำต่อจากที่ค้างไว้").font(bold(22.0)).color(p.text));
        ui.label(RichText::new("ไฟล์ที่เปิดล่าสุดจากทั้งสามโปรแกรม คลิกขวาที่ไฟล์เพื่อปักหมุด").size(13.0).color(p.sub));
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            let choices = [
                (Filter::All, "ทั้งหมด"),
                (Filter::Kind(App::KhianKrub), "เอกสาร"),
                (Filter::Kind(App::KhitKrub), "ตาราง"),
                (Filter::Kind(App::PhutKrub), "งานนำเสนอ"),
                (Filter::Pinned, "ปักหมุดไว้"),
            ];
            for (f, text) in choices {
                if chip(ui, self.filter == f, text, p) {
                    self.filter = f;
                }
            }
        });
        if let Some((msg, at)) = &self.status
            && at.elapsed() < Duration::from_secs(8)
        {
            ui.add_space(8.0);
            ui.label(RichText::new(msg).size(13.0).color(Color32::from_rgb(0xb4, 0x3c, 0x1e)));
            ui.ctx().request_repaint_after(Duration::from_secs(1));
        }
        ui.add_space(16.0);
        let files = self.shown_files();
        if files.is_empty() {
            let why = if !self.search.trim().is_empty() {
                format!("ไม่พบไฟล์ที่ชื่อมีคำว่า \u{201c}{}\u{201d}", self.search.trim())
            } else if self.filter == Filter::Pinned {
                "ยังไม่ได้ปักหมุดไฟล์ไหน คลิกขวาที่ไฟล์แล้วเลือก \u{201c}ปักหมุด\u{201d}".to_string()
            } else if self.recent.is_empty() {
                "ไฟล์ที่เปิดล่าสุดจากทั้งสามโปรแกรมจะแสดงที่นี่".to_string()
            } else {
                "ยังไม่มีไฟล์ประเภทนี้ในรายการล่าสุด".to_string()
            };
            ui.label(RichText::new(why).size(13.5).color(p.sub));
            ui.add_space(12.0);
        }
        // As many columns as fit cards at least 150 points wide.
        const GAP: f32 = 16.0;
        let width = ui.available_width();
        let columns = ((width + GAP) / (150.0 + GAP)).floor().clamp(1.0, 6.0) as usize;
        let card_w = ((width - GAP * (columns as f32 - 1.0)) / columns as f32).floor();
        let icons = self.icons.textures.clone();
        let mut open = None;
        let mut pin = None;
        let mut pick = false;
        let mut items: Vec<Option<&Recent>> = files.iter().map(Some).collect();
        items.push(None);
        for row in items.chunks(columns) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                for item in row {
                    match item {
                        Some(r) => {
                            let texture = icons.iter().find(|(a, _)| *a == r.app).map(|(_, t)| t);
                            let pinned = self.pins.contains(&r.path);
                            let resp = file_card(ui, r, card_w, pinned, texture, p);
                            if resp.clicked() {
                                open = Some(r.path.clone());
                            }
                            resp.context_menu(|ui| {
                                if ui.button("เปิด").clicked() {
                                    open = Some(r.path.clone());
                                }
                                if ui.button(if pinned { "เลิกปักหมุด" } else { "ปักหมุด" }).clicked() {
                                    pin = Some(r.path.clone());
                                }
                            });
                        }
                        None => {
                            if open_card(ui, card_w, p).clicked() {
                                pick = true;
                            }
                        }
                    }
                }
            });
            ui.add_space(GAP);
        }
        if let Some(path) = open {
            self.open(vec![path]);
        }
        if let Some(path) = pin {
            self.toggle_pin(&path);
        }
        if pick {
            self.pick_and_open();
        }
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let line = format!("ไฟล์ของท่านอยู่ในเครื่องของท่าน ไม่ได้อยู่ที่ผมครับ · KrubOffice {} (รุ่นทดลอง) · ใช้ฟรีครับ · ", env!("CARGO_PKG_VERSION"));
            ui.label(RichText::new(line).size(11.5).color(p.sub));
            if ui.link(RichText::new("เกี่ยวกับ").size(11.5).color(p.sub)).clicked() {
                self.window = Some(Window::About);
            }
        });
    }

    /// Under the top bar: the one-time question about checking for updates, or the news of a
    /// newer version.
    fn update_strip(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let lang = self.lang;
        let Some(u) = &mut self.updates else { return };
        let question = u.needs_question();
        let found = if question { None } else { u.found.clone() };
        if !question && found.is_none() {
            return;
        }
        let frame = egui::Frame::NONE.fill(p.strip).inner_margin(egui::Margin::symmetric(28, 10));
        let shown = frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
            if question {
                ui.label(RichText::new(Text::QuestionTitle.get(lang)).font(bold(14.5)).color(p.text));
                ui.label(RichText::new(Text::QuestionBody.get(lang)).size(13.0).color(p.text));
                ui.label(RichText::new(Text::QuestionPrivacy.get(lang)).size(12.0).color(p.sub));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.add(banner_button(Text::CheckAutomatically.get(lang), true, p)).clicked() {
                        u.answer_question(true, &updates::today());
                    }
                    if ui.add(banner_button(Text::DontCheck.get(lang), false, p)).clicked() {
                        u.answer_question(false, &updates::today());
                    }
                });
            } else if let Some(found) = found {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(Text::Available.with_version(lang, &found.version)).font(bold(14.0)).color(p.text));
                    ui.add_space(6.0);
                    if ui.add(banner_button(Text::Download.get(lang), true, p)).on_hover_text(updates::DOWNLOAD_PAGE).clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(updates::DOWNLOAD_PAGE));
                        u.close();
                    }
                    if ui.add(banner_button(Text::Skip.get(lang), false, p)).clicked() {
                        u.skip();
                    }
                    if ui.add(banner_button(Text::Close.get(lang), false, p)).clicked() {
                        u.close();
                    }
                });
            }
        });
        let r = shown.response.rect;
        ui.painter().hline(r.x_range(), r.max.y, Stroke::new(1.0, p.accent.gamma_multiply(0.45)));
    }

    /// The Settings or About window, when one is open.
    fn windows(&mut self, ctx: &egui::Context, p: &Palette) {
        let Some(which) = self.window else { return };
        let modal = egui::Modal::new(egui::Id::new(("kruboffice-window", which as u8)))
            .frame(egui::Frame::popup(&ctx.global_style()).fill(p.card).corner_radius(CornerRadius::same(12)).inner_margin(egui::Margin::same(22)));
        let shown = modal.show(ctx, |ui| {
            ui.set_width(380.0);
            ui.spacing_mut().item_spacing.y = 6.0;
            let close = match which {
                Window::Settings => self.settings_body(ui, p),
                Window::About => about_body(ui, p),
            };
            ui.add_space(10.0);
            close || ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.add(banner_button("ปิด", true, p)).clicked()).inner
        });
        if shown.inner || shown.should_close() {
            self.window = None;
        }
    }

    /// Settings: the mouse pointer, kept in `settings.json` with the other keys.
    fn pointer_setting(&mut self, ui: &mut egui::Ui, p: &Palette) {
        use crate::pointer::Pointer;
        ui.label(RichText::new("ตัวชี้เมาส์").font(bold(14.0)).color(p.text));
        let current = self.pointer;
        let mut chosen = None;
        ui.horizontal(|ui| {
            current.preview(ui, 20.0);
            let width = crate::pointer::picker_width(ui, 20.0, Pointer::ALL.map(pointer_name));
            egui::ComboBox::from_id_salt("kruboffice-pointer").width(width).selected_text(pointer_name(current)).show_ui(ui, |ui| {
                ui.set_min_width(ui.available_width());
                for ptr in Pointer::ALL {
                    let row = ui.horizontal(|ui| {
                        ptr.preview(ui, 20.0);
                        ui.selectable_label(current == ptr, pointer_name(ptr))
                    });
                    if row.inner.clicked() {
                        chosen = Some(ptr);
                    }
                }
            });
        });
        ui.label(RichText::new("เปลี่ยนเฉพาะลูกศร ตัวชี้สำหรับพิมพ์ข้อความ ลิงก์ และปรับขนาด ยังเป็นแบบของระบบ").size(12.5).color(p.sub));
        if let Some(ptr) = chosen.filter(|ptr| *ptr != current) {
            self.pointer = ptr;
            if let Some(settings) = &self.pins_path
                && let Err(e) = recent::save_key(settings, "pointer", serde_json::Value::String(ptr.code().into()))
            {
                self.say(format!("บันทึกการตั้งค่าไม่ได้: {e}"));
            }
        }
    }

    /// Settings: the update check. Returns true to close.
    fn settings_body(&mut self, ui: &mut egui::Ui, p: &Palette) -> bool {
        ui.label(RichText::new("ตั้งค่า").font(bold(18.0)).color(p.text));
        ui.add_space(6.0);
        self.pointer_setting(ui, p);
        ui.add_space(10.0);
        ui.label(RichText::new("รุ่นใหม่ของ KrubOffice").font(bold(14.0)).color(p.text));
        let lang = self.lang;
        let Some(u) = &mut self.updates else {
            ui.label(RichText::new("หน้าต่างนี้ไม่ได้ตรวจหารุ่นใหม่").size(13.0).color(p.sub));
            return false;
        };
        let mut on = u.settings.check == Some(true);
        if ui.checkbox(&mut on, RichText::new(Text::AutomaticToggle.get(lang)).size(13.0).color(p.text)).changed() {
            u.set_automatic(on);
        }
        ui.horizontal(|ui| {
            if ui.add_enabled(!u.checking(), banner_button(Text::CheckNow.get(lang), false, p)).clicked() {
                u.check_now(true, &updates::today());
            }
            if u.checking() {
                ui.add(egui::Spinner::new().size(14.0));
            }
        });
        let line = match &u.status {
            None => None,
            Some(updates::Status::Checking) => Some(Text::Checking.get(lang).to_string()),
            Some(updates::Status::Latest) => Some(Text::Latest.with_version(lang, updates::current())),
            Some(updates::Status::Failed(why)) => Some(format!("{} ({why})", Text::Failed.get(lang))),
        };
        if let Some(line) = line {
            ui.label(RichText::new(line).size(12.5).color(p.sub));
        }
        ui.label(RichText::new(format!("รุ่นที่ใช้อยู่: {}", updates::current())).size(12.5).color(p.sub));
        false
    }
}

/// The Thai name of a pointer, as in the apps' Options.
fn pointer_name(ptr: crate::pointer::Pointer) -> &'static str {
    use crate::pointer::Pointer;
    match ptr {
        Pointer::System => "ตามระบบ",
        Pointer::White => "ลูกศรขาว",
        Pointer::Black => "ลูกศรดำ",
        Pointer::Gold => "ลูกศรทอง",
        Pointer::Large => "ลูกศรใหญ่",
        Pointer::Cat => "แมว",
        Pointer::Rabbit => "กระต่าย",
        Pointer::Chick => "ลูกเจี๊ยบ",
        Pointer::Elephant => "ช้าง",
    }
}

/// About KrubOffice. Returns true to close.
fn about_body(ui: &mut egui::Ui, p: &Palette) -> bool {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::hover());
        logo(&ui.painter_at(r), r);
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label(RichText::new("Krub").font(bold(22.0)).color(p.text));
                ui.label(RichText::new("Office").font(bold(22.0)).color(p.accent));
            });
            ui.label(RichText::new(format!("รุ่น {} (รุ่นทดลอง) · ใช้ฟรีครับ", env!("CARGO_PKG_VERSION"))).size(13.0).color(p.sub));
        });
    });
    ui.add_space(8.0);
    ui.label(RichText::new("เขียนครับ คิดครับ และพูดครับ รวมไว้ในแอปเดียว เปิดไฟล์ไหน KrubOffice ก็เลือกโปรแกรมให้เอง").size(13.5).color(p.text));
    ui.label(RichText::new("ไฟล์ของท่านอยู่ในเครื่องของท่าน ไม่ได้อยู่ที่ผมครับ").size(13.0).color(p.sub));
    ui.label(RichText::new("โอเพนซอร์ส ใช้สัญญาอนุญาต MIT หรือ Apache-2.0").size(13.0).color(p.sub));
    ui.add_space(6.0);
    for (text, url) in [
        ("kruboffice.app", "https://kruboffice.app/"),
        ("เพจ Facebook ของ KrubOffice", "https://www.facebook.com/kruboffice/"),
        ("ซอร์สโค้ดบน GitHub", "https://github.com/chanonss/kruboffice-desktop"),
    ] {
        ui.hyperlink_to(RichText::new(text).size(13.0), url);
    }
    false
}

/// A recent file as a card: the picture of its kind with the program's icon, its name, when it
/// was last changed. A pinned file has a gold pin.
fn file_card(ui: &mut egui::Ui, r: &Recent, width: f32, pinned: bool, texture: Option<&egui::TextureHandle>, p: &Palette) -> egui::Response {
    let pic_h = (width * 0.72).round();
    let (rect, resp) = ui.allocate_exact_size(vec2(width, pic_h + 46.0), Sense::click());
    let painter = ui.painter_at(rect.expand(4.0));
    let pic = egui::Rect::from_min_size(rect.min, vec2(width, pic_h));
    paint_kind(&painter, pic, r.app, p);
    let edge = if resp.hovered() || resp.has_focus() { Stroke::new(2.0, colour(r.app)) } else { Stroke::new(1.0, p.line) };
    painter.rect_stroke(pic, CornerRadius::same(10), edge, egui::StrokeKind::Inside);
    let badge = egui::Rect::from_min_size(pic.max - vec2(34.0, 34.0), vec2(26.0, 26.0));
    paint_icon(&painter, badge, r.app, texture);
    if pinned {
        let c = pic.right_top() + vec2(-16.0, 16.0);
        painter.circle_filled(c, 9.0, GOLD);
        painter.circle_filled(c - vec2(0.0, 1.5), 3.0, Color32::WHITE);
        painter.line_segment([c + vec2(0.0, 1.5), c + vec2(0.0, 5.5)], Stroke::new(1.6, Color32::WHITE));
    }
    let name = r.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    painter.galley(egui::pos2(rect.min.x + 2.0, pic.max.y + 8.0), one_line(&painter, name, bold(13.5), p.text, width - 4.0), p.text);
    let folder = r.path.parent().map(|d| short_folder(&d.display().to_string())).unwrap_or_default();
    let meta = match when(r.modified) {
        w if w.is_empty() => folder,
        w => format!("{w} · {folder}"),
    };
    painter.galley(egui::pos2(rect.min.x + 2.0, pic.max.y + 27.0), one_line(&painter, meta, FontId::proportional(11.5), p.sub, width - 4.0), p.sub);
    let label = format!("{}, {}", r.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), r.app.thai_kind());
    named(resp.on_hover_text(r.path.display().to_string()).on_hover_cursor(egui::CursorIcon::PointingHand), &label)
}

/// The last card: Open File, also where files can be dropped.
fn open_card(ui: &mut egui::Ui, width: f32, p: &Palette) -> egui::Response {
    let pic_h = (width * 0.72).round();
    let (rect, resp) = ui.allocate_exact_size(vec2(width, pic_h + 46.0), Sense::click());
    let painter = ui.painter_at(rect.expand(2.0));
    let pic = egui::Rect::from_min_size(rect.min, vec2(width, pic_h));
    if resp.hovered() || resp.has_focus() {
        painter.rect_filled(pic, CornerRadius::same(10), p.hover);
    }
    let color = p.sub.gamma_multiply(0.7);
    let corners = [pic.left_top(), pic.right_top(), pic.right_bottom(), pic.left_bottom(), pic.left_top()];
    painter.extend(egui::Shape::dashed_line(&corners, Stroke::new(1.4, color), 6.0, 4.0));
    // A folder: a tab and a body.
    let c = pic.center() - vec2(0.0, 12.0);
    let body = egui::Rect::from_center_size(c + vec2(0.0, 2.0), vec2(30.0, 20.0));
    painter.rect_stroke(body, CornerRadius::same(3), Stroke::new(1.8, p.sub), egui::StrokeKind::Middle);
    painter.line_segment([body.left_top() + vec2(2.0, 0.0), body.left_top() + vec2(9.0, -5.0)], Stroke::new(1.8, p.sub));
    painter.line_segment([body.left_top() + vec2(9.0, -5.0), body.left_top() + vec2(15.0, 0.0)], Stroke::new(1.8, p.sub));
    painter.text(c + vec2(0.0, 26.0), Align2::CENTER_CENTER, "เปิดไฟล์อื่น…", bold(13.5), p.text);
    painter.text(egui::pos2(rect.min.x + 2.0, pic.max.y + 18.0), Align2::LEFT_CENTER, "หรือลากไฟล์มาวางที่หน้าต่างนี้", FontId::proportional(11.5), p.sub);
    named(resp.on_hover_cursor(egui::CursorIcon::PointingHand), "เปิดไฟล์อื่น")
}

/// A button in the update strip and the small windows: the main one navy, the others plain.
fn banner_button<'a>(text: &str, primary: bool, p: &Palette) -> egui::Button<'a> {
    let (fill, color) = if primary { (NAVY, Color32::WHITE) } else { (p.card, p.text) };
    egui::Button::new(RichText::new(text).size(13.5).color(color))
        .fill(fill)
        .stroke(Stroke::new(1.0, if primary { NAVY } else { p.sub.gamma_multiply(0.4) }))
        .corner_radius(CornerRadius::same(6))
        .min_size(vec2(0.0, 28.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str) -> Recent {
        let path = PathBuf::from(format!("/files/{name}"));
        let app = dispatch::app_for(&path).unwrap_or(App::KhianKrub);
        Recent { path, app, modified: None }
    }

    #[test]
    fn filters_and_search_pick_the_files_shown() {
        let mut s = StartCenter::new(None, Vec::new());
        s.show_with(vec![file("รายงาน.docx"), file("งบประมาณ.xlsx"), file("Talk.pptx"), file("งบปีหน้า.docx")]);
        s.pin_for_show(vec![PathBuf::from("/files/Talk.pptx")]);
        let names = |s: &StartCenter| -> Vec<String> {
            s.shown_files().iter().filter_map(|r| r.path.file_name().map(|n| n.to_string_lossy().into_owned())).collect()
        };
        assert_eq!(names(&s).len(), 4);
        s.filter = Filter::Kind(App::KhianKrub);
        assert_eq!(names(&s), ["รายงาน.docx", "งบปีหน้า.docx"]);
        s.filter = Filter::Pinned;
        assert_eq!(names(&s), ["Talk.pptx"]);
        s.filter = Filter::All;
        s.search = " งบ ".into();
        assert_eq!(names(&s), ["งบประมาณ.xlsx", "งบปีหน้า.docx"], "the search is trimmed and matches part of a name");
        s.search = "talk".into();
        assert_eq!(names(&s), ["Talk.pptx"], "the search ignores case");
        s.search = "ไม่มีไฟล์นี้".into();
        assert!(names(&s).is_empty());
    }
}
