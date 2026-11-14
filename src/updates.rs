//! Update check: is a newer KrubOffice released?
//!
//! The Start Center asks, at most once a day when it opens (and only after the person said yes
//! once), or whenever the person clicks "Check for updates". It reads the same `latest.json` that
//! kruboffice.app's download card reads, compares its version with this build's, and when a newer
//! release has a file for this platform it shows a bar offering the download page. Nothing is
//! downloaded or installed here: Download opens kruboffice.app/desktop.html, which picks the right
//! file and shows the install steps.
//!
//! The answer to "check automatically?", the day of the last check and a skipped version are kept
//! in KrubOffice's own settings file (`settings.json` in its settings folder, next to the
//! programs' folders); the programs' own update checks stay off.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, SystemTime};

use serde_json::{Value, json};

/// The release feed: written by the release workflow, read by kruboffice.app too.
pub const FEED: &str = "https://raw.githubusercontent.com/chanonss/kruboffice-desktop/main/latest.json";
/// Where Download leads: the install page picks this computer's file and shows the steps.
pub const DOWNLOAD_PAGE: &str = "https://kruboffice.app/desktop.html";
/// Download links in the feed must point at the project's releases.
const DOWNLOADS: &str = "https://github.com/chanonss/kruboffice-desktop/releases/download/";
/// The most of the feed read: it is well under a kilobyte.
const MAX_BYTES: u64 = 64 * 1024;
/// The most of the settings file read.
const MAX_SETTINGS_BYTES: u64 = 64 * 1024;

/// The desktop platforms KrubOffice is released for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Windows,
    Linux,
}

impl Platform {
    /// The platform this build runs on.
    pub fn this() -> Platform {
        if cfg!(target_os = "macos") {
            Platform::MacOs
        } else if cfg!(windows) {
            Platform::Windows
        } else {
            Platform::Linux
        }
    }

    /// The `files` entries in `latest.json` that serve this platform (any one is enough).
    fn keys(self) -> &'static [&'static str] {
        match self {
            Platform::MacOs => &["macos"],
            Platform::Windows => &["windows"],
            Platform::Linux => &["linux", "linux_appimage"],
        }
    }
}

/// A newer release: its version (`0.5.0`) and its release notes page, if the feed gives one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub notes: Option<String>,
}

/// The release in `latest.json`, if it is a final version (no `-test1`) with a download for
/// `platform` on the project's releases page. A release that left this platform out is no news
/// here.
pub fn release(v: &Value, platform: Platform) -> Option<Release> {
    let version = v.get("version")?.as_str()?;
    if version.contains('-') || parts(version).is_none() {
        return None;
    }
    let files = v.get("files")?;
    let has_file =
        platform.keys().iter().any(|k| files.get(*k).and_then(|f| f.get("url")).and_then(Value::as_str).is_some_and(|u| u.starts_with(DOWNLOADS)));
    if !has_file {
        return None;
    }
    let notes = v
        .get("notes")
        .and_then(Value::as_str)
        .filter(|u| u.starts_with("https://github.com/chanonss/kruboffice-desktop/releases/"))
        .map(str::to_string);
    Some(Release { version: version.to_string(), notes })
}

/// `1.2.3` as numbers; `None` for anything else.
fn parts(v: &str) -> Option<Vec<u64>> {
    let p: Option<Vec<u64>> = v.split('.').map(|n| n.parse().ok()).collect();
    p.filter(|p| !p.is_empty() && p.len() <= 4)
}

/// Is `latest` a later version than `current`? (`0.4.10` > `0.4.9`; `0.5` > `0.4.3`.) A
/// development build (`0.4.0-dev`) compares by its number; anything unreadable is not newer.
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parts(latest), parts(current.split('-').next().unwrap_or(current))) {
        (Some(a), Some(b)) => {
            let n = a.len().max(b.len());
            let pad = |v: Vec<u64>| v.into_iter().chain(std::iter::repeat(0)).take(n).collect::<Vec<_>>();
            pad(a) > pad(b)
        }
        _ => false,
    }
}

/// This build's version.
pub fn current() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The date (UTC) `secs` seconds after 1970, as `2026-11-14`.
pub fn date_of(secs: u64) -> String {
    // Days to a civil date (Howard Hinnant's algorithm), valid for every u64 day count we can get.
    let days = (secs / 86_400) as i64 + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Today's date (UTC). Once a day means once per UTC day: in Thailand the day turns at 07:00.
pub fn today() -> String {
    date_of(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0))
}

/// What KrubOffice remembers about update checks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    /// The answer to "check automatically?": `None` until the person has answered.
    pub check: Option<bool>,
    /// The day of the last check (`2026-11-14`), automatic or not.
    pub checked_on: String,
    /// A version the person chose to skip: no bar for it at start-up.
    pub skipped: String,
}

impl Settings {
    /// Settings from the file's JSON; anything missing or of the wrong type keeps its default.
    pub fn from_json(v: &Value) -> Settings {
        let text = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().chars().take(32).collect::<String>();
        Settings { check: v.get("checkUpdates").and_then(Value::as_bool), checked_on: text("updateCheckedOn"), skipped: text("updateSkipped") }
    }

    /// The settings as JSON, kept with any other keys `old` had.
    pub fn to_json(&self, old: Value) -> Value {
        let mut v = if old.is_object() { old } else { json!({}) };
        if let Some(map) = v.as_object_mut() {
            match self.check {
                Some(b) => map.insert("checkUpdates".into(), Value::Bool(b)),
                None => map.remove("checkUpdates"),
            };
            map.insert("updateCheckedOn".into(), Value::String(self.checked_on.clone()));
            map.insert("updateSkipped".into(), Value::String(self.skipped.clone()));
        }
        v
    }

    /// Read the settings file; a missing, oversized or broken one gives the defaults.
    pub fn load(path: &Path) -> Settings {
        Settings::from_json(&read_json(path))
    }

    /// Write the settings file (through a temporary file, so a crash never leaves half a file).
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let v = self.to_json(read_json(path));
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
        std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
    }
}

fn read_json(path: &Path) -> Value {
    let Ok(meta) = std::fs::metadata(path) else { return Value::Null };
    if meta.len() > MAX_SETTINGS_BYTES {
        return Value::Null;
    }
    std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}

/// KrubOffice's settings file under the settings root (`KrubOffice/settings.json`; lower-case
/// folder on Linux, as the programs do).
pub fn settings_path(root: &Path) -> PathBuf {
    let folder = if cfg!(any(target_os = "macos", windows)) { "KrubOffice" } else { "kruboffice" };
    root.join(folder).join("settings.json")
}

/// Should the Start Center check by itself now? Only when the person said yes, and not if it
/// already checked today.
pub fn due_at_start(s: &Settings, today: &str) -> bool {
    s.check == Some(true) && s.checked_on != today
}

/// The feed's JSON, or why it couldn't be read.
pub type Answer = Result<Value, String>;

/// Something that asks for the feed and answers on a channel (the network, or a test).
pub type Fetch = Box<dyn Fn() -> mpsc::Receiver<Answer>>;

/// The feed address. Debug builds may point it elsewhere with `KRUBOFFICE_UPDATE_FEED` (to try
/// the bar against a local file); release builds always read the real one.
fn feed_url() -> String {
    if cfg!(debug_assertions)
        && let Ok(url) = std::env::var("KRUBOFFICE_UPDATE_FEED")
        && (url.starts_with("https://") || url.starts_with("http://127.0.0.1:"))
    {
        return url;
    }
    FEED.to_string()
}

/// Ask the network on a background thread, so the window never waits.
pub fn ask_network() -> mpsc::Receiver<Answer> {
    let (tx, rx) = mpsc::channel();
    let url = feed_url();
    let spawned = std::thread::Builder::new().name("update-check".into()).spawn({
        let tx = tx.clone();
        move || {
            let _ = tx.send(fetch(&url));
        }
    });
    if let Err(e) = spawned {
        let _ = tx.send(Err(e.to_string()));
    }
    rx
}

fn fetch(url: &str) -> Answer {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(15)).build();
    let resp = agent.get(url).set("User-Agent", concat!("KrubOffice/", env!("CARGO_PKG_VERSION"))).call().map_err(|e| match e {
        ureq::Error::Status(code, _) => format!("HTTP {code}"),
        ureq::Error::Transport(t) => t.to_string(),
    })?;
    let mut body = String::new();
    resp.into_reader().take(MAX_BYTES).read_to_string(&mut body).map_err(|e| e.to_string())?;
    serde_json::from_str(&body).map_err(|e| e.to_string())
}

/// What a check asked from the Start Center's link says, under the link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Asking.
    Checking,
    /// This version is the latest.
    Latest,
    /// Couldn't ask, and why.
    Failed(String),
}

/// The update check's state in the Start Center.
pub struct Updates {
    pub settings: Settings,
    /// Where the settings are saved; `None` keeps them in memory only.
    path: Option<PathBuf>,
    fetch: Fetch,
    platform: Platform,
    pending: Option<mpsc::Receiver<Answer>>,
    /// The check in progress was asked for by the person.
    manual: bool,
    started: bool,
    /// A newer release, while its bar shows.
    pub found: Option<Release>,
    /// What the last asked-for check found (no newer version), shown under the link.
    pub status: Option<Status>,
}

impl Updates {
    pub fn new(settings: Settings, path: Option<PathBuf>, fetch: Fetch, platform: Platform) -> Updates {
        Updates { settings, path, fetch, platform, pending: None, manual: false, started: false, found: None, status: None }
    }

    /// The real thing: settings from KrubOffice's settings file, answers from the network.
    pub fn load() -> Updates {
        let path = crate::recent::settings_root().map(|root| settings_path(&root));
        let settings = path.as_deref().map(Settings::load).unwrap_or_default();
        Updates::new(settings, path, Box::new(ask_network), Platform::this())
    }

    /// Has the person yet to answer "check automatically?"
    pub fn needs_question(&self) -> bool {
        self.settings.check.is_none()
    }

    /// A check is under way.
    pub fn checking(&self) -> bool {
        self.pending.is_some()
    }

    fn save(&self) {
        if let Some(path) = &self.path
            && let Err(e) = self.settings.save(path)
        {
            log::warn!("update settings not saved: {e}");
        }
    }

    /// Ask now (`manual`: from the link, so "up to date" and errors are reported and a skipped
    /// version shows again).
    pub fn check_now(&mut self, manual: bool, today: &str) {
        if self.pending.is_some() {
            self.manual |= manual;
            return;
        }
        self.pending = Some((self.fetch)());
        self.manual = manual;
        if manual {
            self.found = None;
            self.status = Some(Status::Checking);
        }
        self.settings.checked_on = today.to_string();
        self.save();
    }

    /// Every frame while the Start Center shows: the start-up check (once a day, when allowed)
    /// and the answer when it comes. True while waiting, so the caller keeps repainting.
    pub fn tick(&mut self, today: &str) -> bool {
        if !self.started {
            self.started = true;
            if due_at_start(&self.settings, today) {
                self.check_now(false, today);
            }
        }
        let Some(rx) = &self.pending else { return false };
        let answer = match rx.try_recv() {
            Ok(a) => a,
            Err(mpsc::TryRecvError::Empty) => return true,
            Err(mpsc::TryRecvError::Disconnected) => Err("no answer".into()),
        };
        self.pending = None;
        let manual = std::mem::take(&mut self.manual);
        self.take_answer(answer, manual);
        false
    }

    /// What an answer means: a newer release for this platform shows the bar (unless it was
    /// skipped and the check wasn't asked for); an asked-for check also says "up to date" or why
    /// it failed.
    pub fn take_answer(&mut self, answer: Answer, manual: bool) {
        let newer = match &answer {
            Ok(v) => release(v, self.platform).filter(|r| is_newer(&r.version, current())),
            Err(_) => None,
        };
        match &answer {
            Ok(v) => log::info!("update check: latest {} (this is {})", v.get("version").and_then(Value::as_str).unwrap_or("?"), current()),
            Err(e) => log::info!("update check: {e}"),
        }
        match newer {
            Some(r) => {
                if manual || self.settings.skipped != r.version {
                    self.found = Some(r);
                }
                if manual {
                    self.status = None;
                }
            }
            None if manual => {
                self.status = Some(match answer {
                    Ok(_) => Status::Latest,
                    Err(e) => Status::Failed(e),
                });
            }
            None => {}
        }
    }

    /// The answer to "check automatically?": yes also checks now (the start-up moment has passed).
    pub fn answer_question(&mut self, yes: bool, today: &str) {
        self.settings.check = Some(yes);
        self.save();
        if yes {
            self.check_now(false, today);
        }
    }

    /// Turn the start-up check on or off later.
    pub fn set_automatic(&mut self, on: bool) {
        self.settings.check = Some(on);
        self.save();
    }

    /// Skip this version: no bar for it at start-up from now on.
    pub fn skip(&mut self) {
        if let Some(r) = self.found.take() {
            self.settings.skipped = r.version;
            self.save();
        }
    }

    /// Hide the bar until the next time KrubOffice opens.
    pub fn close(&mut self) {
        self.found = None;
    }
}

/// The interface language for the update texts: the programs' own setting (KhianKrub first),
/// Thai when none is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Thai,
    English,
}

impl Lang {
    /// The language the programs use, read from their `ui.json` (`"language": "en"`).
    pub fn of_programs(root: &Path) -> Lang {
        for app in crate::dispatch::App::ALL {
            let folder =
                if cfg!(any(target_os = "macos", windows)) { app.settings_folder().to_string() } else { app.settings_folder().to_ascii_lowercase() };
            match read_json(&root.join(folder).join("ui.json")).get("language").and_then(Value::as_str) {
                Some("en") => return Lang::English,
                Some("th") => return Lang::Thai,
                _ => {}
            }
        }
        Lang::Thai
    }
}

/// The update texts in Thai and English.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Text {
    QuestionTitle,
    QuestionBody,
    QuestionPrivacy,
    CheckAutomatically,
    DontCheck,
    Available,
    Download,
    Skip,
    Close,
    CheckNow,
    Checking,
    Latest,
    Failed,
    AutomaticToggle,
}

impl Text {
    pub const ALL: [Text; 14] = [
        Text::QuestionTitle,
        Text::QuestionBody,
        Text::QuestionPrivacy,
        Text::CheckAutomatically,
        Text::DontCheck,
        Text::Available,
        Text::Download,
        Text::Skip,
        Text::Close,
        Text::CheckNow,
        Text::Checking,
        Text::Latest,
        Text::Failed,
        Text::AutomaticToggle,
    ];

    /// The text; `{version}` stands for a version number.
    pub fn get(self, lang: Lang) -> &'static str {
        let (th, en) = match self {
            Text::QuestionTitle => ("ให้ KrubOffice คอยดูรุ่นใหม่ให้ไหมครับ?", "Check for updates automatically?"),
            Text::QuestionBody => (
                "เมื่อเปิด KrubOffice วันละครั้ง ผมจะดูว่ามีรุ่นใหม่ออกมาหรือยัง ถ้ามีจะบอกและพาไปหน้าดาวน์โหลด",
                "Once a day, when it opens, KrubOffice can look whether a newer version has been released, and offer the download.",
            ),
            Text::QuestionPrivacy => (
                "ส่งไปแค่คำขอดูเลขรุ่นเท่านั้น ไม่ส่งไฟล์ ชื่อ หรือข้อมูลอื่นของท่าน เปลี่ยนใจได้ทุกเมื่อที่ด้านล่างของหน้านี้",
                "Only that request is sent: no files, names or other information. You can change this at the bottom of this page.",
            ),
            Text::CheckAutomatically => ("ดูให้อัตโนมัติ", "Check Automatically"),
            Text::DontCheck => ("ไม่ต้อง", "Don't Check"),
            Text::Available => ("KrubOffice รุ่น {version} ออกแล้วครับ", "KrubOffice {version} is available."),
            Text::Download => ("ไปหน้าดาวน์โหลด", "Download"),
            Text::Skip => ("ข้ามรุ่นนี้", "Skip This Version"),
            Text::Close => ("ปิด", "Close"),
            Text::CheckNow => ("ดูว่ามีรุ่นใหม่ไหม", "Check for Updates"),
            Text::Checking => ("กำลังดูว่ามีรุ่นใหม่ไหม…", "Checking for updates…"),
            Text::Latest => ("รุ่น {version} ที่ใช้อยู่เป็นรุ่นล่าสุดแล้วครับ", "KrubOffice {version} is the latest version."),
            Text::Failed => ("ดูรุ่นใหม่ไม่ได้ ลองเช็กอินเทอร์เน็ตแล้วลองอีกครั้งครับ", "Couldn't check for updates. Check the internet connection and try again."),
            Text::AutomaticToggle => ("ดูรุ่นใหม่ให้อัตโนมัติ", "Check for updates automatically"),
        };
        match lang {
            Lang::Thai => th,
            Lang::English => en,
        }
    }

    /// The text with `{version}` filled in.
    pub fn with_version(self, lang: Lang, version: &str) -> String {
        self.get(lang).replace("{version}", version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(version: &str) -> Value {
        json!({
            "version": version,
            "date": "2026-11-20",
            "notes": format!("https://github.com/chanonss/kruboffice-desktop/releases/tag/v{version}"),
            "files": {
                "macos": {"name": "a.dmg", "url": format!("{DOWNLOADS}v{version}/a.dmg")},
                "windows": {"name": "a.msi", "url": format!("{DOWNLOADS}v{version}/a.msi")},
                "linux_appimage": {"name": "a.AppImage", "url": format!("{DOWNLOADS}v{version}/a.AppImage")},
            }
        })
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(is_newer("0.4.1", "0.4.0"));
        assert!(is_newer("0.4.10", "0.4.9"));
        assert!(is_newer("0.5", "0.4.3"));
        assert!(is_newer("1.0.0", "0.4.0"));
        assert!(!is_newer("0.4.0", "0.4.0"));
        assert!(!is_newer("0.4", "0.4.0"), "missing parts count as zero");
        assert!(!is_newer("0.3.9", "0.4.0"));
        assert!(!is_newer("junk", "0.4.0"));
        assert!(!is_newer("", "0.4.0"));
        assert!(!is_newer("0.5.0", "junk"));
        assert!(is_newer("0.4.1", "0.4.0-dev"), "a development build compares by its number");
    }

    #[test]
    fn only_final_releases_with_a_file_for_this_platform_count() {
        let r = release(&feed("0.5.0"), Platform::MacOs);
        assert_eq!(r.as_ref().map(|r| r.version.as_str()), Some("0.5.0"));
        assert!(r.and_then(|r| r.notes).is_some());
        assert!(release(&feed("0.5.0"), Platform::Linux).is_some(), "the AppImage alone serves Linux");
        assert!(release(&feed("0.5.0-test1"), Platform::MacOs).is_none(), "test versions");
        let mut no_windows = feed("0.5.0");
        if let Some(files) = no_windows.get_mut("files").and_then(Value::as_object_mut) {
            files.remove("windows");
        }
        assert!(release(&no_windows, Platform::Windows).is_none(), "nothing to download here");
        let elsewhere = json!({"version": "0.5.0", "files": {"macos": {"url": "https://example.com/a.dmg"}}});
        assert!(release(&elsewhere, Platform::MacOs).is_none(), "downloads only from the project's releases");
        assert!(release(&json!({}), Platform::MacOs).is_none());
        assert!(release(&json!("junk"), Platform::MacOs).is_none());
    }

    #[test]
    fn dates_are_civil_utc_days() {
        assert_eq!(date_of(0), "1970-01-01");
        assert_eq!(date_of(951_782_400), "2000-02-29");
        assert_eq!(date_of(1_794_614_400), "2026-11-14");
        assert_eq!(date_of(1_794_614_400 + 86_399), "2026-11-14");
        assert_eq!(date_of(1_794_614_400 + 86_400), "2026-11-15");
        assert_eq!(today().len(), 10);
    }

    #[test]
    fn checks_at_start_at_most_once_a_day_and_only_when_allowed() {
        let mut s = Settings::default();
        assert!(!due_at_start(&s, "2026-11-14"), "not asked yet: no check");
        s.check = Some(false);
        assert!(!due_at_start(&s, "2026-11-14"), "said no");
        s.check = Some(true);
        assert!(due_at_start(&s, "2026-11-14"), "never checked");
        s.checked_on = "2026-11-14".into();
        assert!(!due_at_start(&s, "2026-11-14"), "already checked today");
        assert!(due_at_start(&s, "2026-11-15"), "a new day");
    }

    /// An `Updates` whose answers come from `answers`, counting how often it asks.
    fn fake(settings: Settings, answer: Answer) -> (Updates, std::rc::Rc<std::cell::Cell<u32>>) {
        let asked = std::rc::Rc::new(std::cell::Cell::new(0));
        let count = asked.clone();
        let fetch: Fetch = Box::new(move || {
            count.set(count.get() + 1);
            let (tx, rx) = mpsc::channel();
            let _ = tx.send(answer.clone());
            rx
        });
        (Updates::new(settings, None, fetch, Platform::MacOs), asked)
    }

    #[test]
    fn the_start_center_asks_once_a_day() {
        let on = Settings { check: Some(true), ..Settings::default() };
        let (mut u, asked) = fake(on, Ok(feed("99.0.0")));
        u.tick("2026-11-14");
        u.tick("2026-11-14");
        assert_eq!(asked.get(), 1, "one check at start-up, not one per frame");
        assert_eq!(u.settings.checked_on, "2026-11-14");
        assert_eq!(u.found.as_ref().map(|r| r.version.as_str()), Some("99.0.0"));
        assert_eq!(u.status, None, "a start-up check says nothing when there's no news");

        // Opened again the same day: no check.
        let (mut again, asked) = fake(u.settings.clone(), Ok(feed("99.0.0")));
        again.tick("2026-11-14");
        assert_eq!(asked.get(), 0);
        // The next day: one.
        let (mut next, asked) = fake(u.settings.clone(), Ok(feed("99.0.0")));
        next.tick("2026-11-15");
        assert_eq!(asked.get(), 1);

        // Never answered, or said no: no start-up check.
        for s in [Settings::default(), Settings { check: Some(false), ..Settings::default() }] {
            let (mut u, asked) = fake(s, Ok(feed("99.0.0")));
            u.tick("2026-11-14");
            assert_eq!(asked.get(), 0);
        }
    }

    #[test]
    fn answering_yes_checks_now_and_no_never_asks() {
        let (mut u, asked) = fake(Settings::default(), Ok(feed("99.0.0")));
        assert!(u.needs_question());
        u.tick("2026-11-14");
        u.answer_question(true, "2026-11-14");
        u.tick("2026-11-14");
        assert!(!u.needs_question());
        assert_eq!(asked.get(), 1);
        assert!(u.found.is_some());

        let (mut u, asked) = fake(Settings::default(), Ok(feed("99.0.0")));
        u.tick("2026-11-14");
        u.answer_question(false, "2026-11-14");
        u.tick("2026-11-14");
        assert_eq!(asked.get(), 0);
        assert_eq!(u.settings.check, Some(false));
    }

    #[test]
    fn a_skipped_version_shows_only_when_asked_for() {
        let skipped = Settings { check: Some(true), skipped: "99.0.0".into(), ..Settings::default() };
        let (mut u, _) = fake(skipped, Ok(feed("99.0.0")));
        u.tick("2026-11-14");
        assert_eq!(u.found, None, "skipped at start-up");
        u.check_now(true, "2026-11-14");
        u.tick("2026-11-14");
        assert!(u.found.is_some(), "asked for: shown");
        u.skip();
        assert_eq!(u.found, None);
        assert_eq!(u.settings.skipped, "99.0.0");
    }

    #[test]
    fn an_asked_for_check_says_up_to_date_or_why_it_failed() {
        let (mut u, _) = fake(Settings::default(), Ok(feed(current())));
        u.check_now(true, "2026-11-14");
        assert_eq!(u.status, Some(Status::Checking));
        u.tick("2026-11-14");
        assert_eq!(u.status, Some(Status::Latest));
        assert_eq!(u.found, None);

        let (mut u, _) = fake(Settings::default(), Err("offline".into()));
        u.check_now(true, "2026-11-14");
        u.tick("2026-11-14");
        assert_eq!(u.status, Some(Status::Failed("offline".into())));

        let on = Settings { check: Some(true), ..Settings::default() };
        let (mut u, _) = fake(on, Err("offline".into()));
        u.tick("2026-11-14");
        assert_eq!(u.status, None, "a failed start-up check stays quiet");
    }

    #[test]
    fn settings_survive_a_round_trip_and_keep_other_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let path = settings_path(tmp.path());
        assert_eq!(Settings::load(&path), Settings::default(), "no file yet");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"other": 1}"#).unwrap();
        let s = Settings { check: Some(true), checked_on: "2026-11-14".into(), skipped: "0.5.0".into() };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        let raw: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(raw.get("other"), Some(&json!(1)));
        for junk in ["not json", "[]", r#"{"checkUpdates": "yes", "updateSkipped": 5}"#] {
            std::fs::write(&path, junk).unwrap();
            assert_eq!(Settings::load(&path), Settings::default());
        }
    }

    #[test]
    fn the_language_follows_the_programs() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(Lang::of_programs(tmp.path()), Lang::Thai, "nothing set");
        let folder = if cfg!(any(target_os = "macos", windows)) { "KhitKrub" } else { "khitkrub" };
        std::fs::create_dir_all(tmp.path().join(folder)).unwrap();
        std::fs::write(tmp.path().join(folder).join("ui.json"), r#"{"language": "en"}"#).unwrap();
        assert_eq!(Lang::of_programs(tmp.path()), Lang::English);
    }

    #[test]
    fn every_text_has_thai_and_english() {
        for t in Text::ALL {
            assert!(!t.get(Lang::Thai).is_empty() && !t.get(Lang::English).is_empty(), "{t:?}");
            assert_ne!(t.get(Lang::Thai), t.get(Lang::English), "{t:?}");
        }
        assert_eq!(Text::Available.with_version(Lang::English, "0.5.0"), "KrubOffice 0.5.0 is available.");
    }
}
