use super::printing;

pub use krub_drive::DriveServices;

type PickSave = Box<dyn Fn(&str) -> Option<String>>;
type OpenUrl = Box<dyn Fn(&str)>;
type CopyHtml = Box<dyn FnMut(&str, &str) -> Result<(), String>>;
type Download = Box<dyn Fn(&str, &[u8])>;
type DraftList = Box<dyn Fn(&str) -> Option<Vec<BrowserDraft>>>;
type DraftAct = Box<dyn Fn(DraftAction, &str)>;
type PrintPdf = Box<dyn Fn(&[u8])>;

/// Browser-local drafts, supplied by the web host. No workbook bytes leave the device.
pub struct BrowserRecoveryServices {
    /// The drafts the start screen lists, newest first, with the time written for the language
    /// given (its code, `th`, `en`...); `None` while the browser's storage is still being read.
    pub drafts: DraftList,
    /// Download or discard a draft (by id); the host finishes it later. Opening one is the start
    /// screen's own business: it opens `./?draft=<id>` in a new tab.
    pub act: DraftAct,
    /// The browser may remove drafts by itself (it hasn't agreed to keep this site's storage).
    pub fragile: Box<dyn Fn() -> bool>,
    /// The active workbook is kept only as a draft in this browser (never saved as a file or to
    /// Google Drive): `Some(true)` once its latest edit is in the draft, `Some(false)` while saving.
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

/// Host services the platform shell provides (file pickers, opening URLs).
#[derive(Default)]
pub struct Services {
    /// Web: Google Drive, when the site is configured for it.
    pub google_drive: Option<DriveServices>,
    /// Web: the drafts kept in this browser. Its presence lets the window show a web start screen
    /// (see `SheetApp::set_web_start_screen`).
    pub browser_recovery: Option<BrowserRecoveryServices>,
    /// Desktop: discover configured printers on a worker.
    pub printers: Option<printing::Discover>,
    /// Desktop: submit the PDF with the chosen settings on a worker.
    pub print_native: Option<printing::Submit>,
    pub pick_open: Option<Box<dyn Fn() -> Option<String>>>,
    pub pick_save: Option<PickSave>,
    /// Native picture picker (PNG/JPEG), separate from workbook opening.
    pub pick_picture: Option<Box<dyn Fn() -> Option<String>>>,
    /// Browser picture picker. Each dialog owns its inbox, so canceled picks cannot
    /// insert into a subsequent dialog or open the image as a workbook.
    pub pick_picture_async: Option<Box<dyn Fn(Inbox)>>,
    pub open_url: Option<OpenUrl>,
    /// Publish HTML and its plain-text alternative together. `Ok` means the host owns the
    /// write (including any asynchronous fallback); an immediate error uses egui's text path.
    pub copy_html: Option<CopyHtml>,
    /// Read the HTML flavour of the system clipboard when the user pastes text, so cells the
    /// source spreadsheet recorded as numbers stay numbers ("Rp10,000.00" → 10000). `None` from
    /// the host pastes the plain text alone.
    pub paste_html: Option<Box<dyn FnMut() -> Option<String>>>,
    /// Read the system clipboard's plain text for the ribbon and menu Paste commands (Cmd/Ctrl+V
    /// gets it from egui's `Event::Paste`). `None` from the host, or no service (web), pastes
    /// GridCraft's own clipboard.
    pub paste_text: Option<Box<dyn FnMut() -> Option<String>>>,
    /// Read the picture on the system clipboard for the ribbon and menu Paste commands when it
    /// holds no text (Cmd/Ctrl+V gets it from egui's `Event::PasteImage`).
    pub paste_image: Option<Box<dyn FnMut() -> Option<image::RgbaImage>>>,
    /// Web: deliver a file to the user (name, bytes).
    pub download: Option<Download>,
    /// Web: start an asynchronous file pick; the bytes arrive later in `inbox`.
    pub open_async: Option<Box<dyn Fn()>>,
    /// Files delivered asynchronously (name, bytes), opened on the next frame.
    pub inbox: Option<Inbox>,
    /// Web: hand PDF bytes to the browser's print flow directly (no download,
    /// no intermediate file) — opens the system print dialog on that PDF.
    pub print: Option<PrintPdf>,
    /// Web: read the computer's interface font through the browser's Local Font Access and use it
    /// for the menus and ribbon. The browser asks the person once; `None` when it can't.
    pub load_system_ui_fonts: Option<Box<dyn Fn()>>,
}

/// Shared queue of files read asynchronously (browser file picker, drag and drop).
pub type Inbox = std::sync::Arc<std::sync::Mutex<Vec<(String, Vec<u8>)>>>;

/// Which parts of the app's own window chrome are drawn. Everything is on by default; a host
/// that embeds the sheet in its own window (and owns files itself) can turn off the title bar
/// and the File tab, whose Backstage view then never opens. Not persisted: it describes the
/// host, not a user preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chrome {
    /// The title bar: AutoSave, Home, Save, Undo/Redo, the file name, Comments, Share, Search and
    /// the theme switch.
    pub title_bar: bool,
    /// The ribbon's File tab and the Backstage view it opens (also reached by keyboard and
    /// commands such as `ui.backstage`).
    pub file_tab: bool,
}

impl Default for Chrome {
    fn default() -> Self {
        Chrome { title_bar: true, file_tab: true }
    }
}
