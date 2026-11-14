pub use krub_drive::DriveServices;

pub type ReadFn = Box<dyn Fn(&str) -> Result<Vec<u8>, String>>;

pub type WriteFn = Box<dyn FnMut(&str, &[u8]) -> Result<(), String>>;

pub type PickFn = Box<dyn FnMut(&str) -> Option<String>>;

/// `(purpose, request id)`. The host shows a picker and later pushes an [`InboxEvent`]
/// carrying that same id. Completing a cancelled or unknown id does nothing: the file is
/// not applied to whichever document happens to be active.
pub type OpenAsyncFn = Box<dyn FnMut(&str, u64)>;

pub type DownloadFn = Box<dyn FnMut(&str, &[u8])>;

/// [`Services::host_save`]: `(file name, bytes, save_as)`. An error means the page didn't get
/// the file, and the save failed.
pub type HostSaveFn = Box<dyn FnMut(&str, &[u8], bool) -> Result<(), String>>;

/// A file the host finished reading for the request `id` from [`SlideApp::begin_external_file`].
#[derive(Clone, Debug)]
pub struct InboxFile {
    /// Cancellation identity. A file whose id was cancelled, or never started, is ignored.
    pub id: u64,
    pub name: String,
    pub bytes: Vec<u8>,
}

/// What the host delivers through [`Inbox`] after an asynchronous read.
#[derive(Clone, Debug)]
pub enum InboxEvent {
    File(InboxFile),
    /// The picker was dismissed or the file could not be read. `id` will not be applied.
    Cancelled {
        id: u64,
    },
}

/// Files delivered asynchronously by the host (web file picker, dropped files).
pub type Inbox = std::sync::Arc<std::sync::Mutex<Vec<InboxEvent>>>;

/// Browser-local drafts, supplied by the web host. No presentation bytes leave the device.
pub struct BrowserRecoveryServices {
    /// The drafts the start screen lists, newest first, with the time written for the language
    /// given; `None` while the browser's storage is still being read.
    pub drafts: Box<dyn Fn(&str) -> Option<Vec<BrowserDraft>>>,
    /// Open, download or discard a draft (by id); the host finishes it later.
    pub act: Box<dyn Fn(DraftAction, &str)>,
    /// The browser may remove drafts by itself (it hasn't agreed to keep this site's storage).
    pub fragile: Box<dyn Fn() -> bool>,
    /// The active presentation is kept only as a draft in this browser (never saved as a file or
    /// to Google Drive): `Some(true)` once its latest edit is in the draft, `Some(false)` while saving.
    pub open_draft: Box<dyn Fn() -> Option<bool>>,
}

/// A draft kept in the browser, as the start screen shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserDraft {
    pub id: String,
    pub name: String,
    pub saved: String,
}

/// What the start screen asks the web host to do with a browser draft.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftAction {
    Open,
    Download,
    Discard,
}

/// Platform services injected by the host (desktop or web).
#[derive(Default)]
pub struct Services {
    /// Web: Google Drive, when the site is configured for it.
    pub google_drive: Option<DriveServices>,
    /// Web: the drafts kept in this browser. Its presence makes the empty window a web start screen.
    pub browser_recovery: Option<BrowserRecoveryServices>,
    /// Web: read the computer's interface font through the browser's Local Font Access and use it
    /// for the menus and ribbon. The browser asks the person once; `None` when it can't.
    pub load_system_ui_fonts: Option<Box<dyn Fn()>>,
    /// Browser-owned presenter popup; absent on native hosts.
    pub presenter: Option<Box<dyn crate::presenter::Host>>,
    /// Open-file dialog for a purpose (`open`, `picture`, `audio`, `video`) → path.
    pub pick_open: Option<PickFn>,
    /// Save dialog with a suggested name → path.
    pub pick_save: Option<PickFn>,
    pub read: Option<ReadFn>,
    pub write: Option<WriteFn>,
    /// Asynchronous open dialog (web). The callback receives the purpose and the request id
    /// from [`SlideApp::begin_external_file`]; the chosen file arrives through `inbox`.
    pub open_async: Option<OpenAsyncFn>,
    /// Hand bytes to the user as a named file (browser download).
    pub download: Option<DownloadFn>,
    /// Web, framed by a page that keeps the files (`?host=parent`): Save and Save As hand the
    /// presentation to that page instead of downloading it, with whether it is a new file
    /// ([`HostSaveFn`]). Save writes a presentation the page sent back under its own name and
    /// format, as the desktop app writes back to the file it opened; a presentation the page
    /// never sent (an untitled one, a repaired copy) goes as a new `.deckcraft` file, as Save
    /// suggests on the desktop. Save As (.pptx or .deckcraft) is always a new file. Either way the
    /// presentation is then that file, and the next Save goes to it. Exports and Print still go
    /// through `download`, which the host also points at the page: they are copies.
    pub host_save: Option<HostSaveFn>,
    pub inbox: Option<Inbox>,
    /// Image on the system clipboard (PNG bytes), when the host can read one.
    pub clipboard_image: Option<Box<dyn FnMut() -> Option<Vec<u8>>>>,
    /// HTML on the system clipboard (Pages, Keynote, and other apps that write `public.html`).
    pub clipboard_html: Option<Box<dyn FnMut() -> Option<String>>>,
    /// Put HTML and its plain-text alternative on the system clipboard. `true` when it landed.
    pub clipboard_set_html: Option<Box<dyn FnMut(&str, &str) -> bool>>,
    /// Audio output for media playback (cpal on desktop); without one media plays silently.
    pub audio_out: Option<Box<dyn deckcraft_media::AudioOut>>,
    /// How many displays are connected, when the host can tell: Presenter View needs a second.
    pub displays: Option<Box<dyn FnMut() -> Option<usize>>>,
}
