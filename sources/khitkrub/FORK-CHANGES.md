# Changes in this fork

This is Chanon Sarasuk's fork of [storytold/gridcraft](https://github.com/storytold/gridcraft), kept for a
Thai-speaking audience and called **คิดครับ · KhitKrub**. It follows KhianKrub (the WordCraft fork in
`../khiankrub-web`) and PhutKrub (the DeckCraft fork in `../phutkrub-web`). This file lists what differs from
the original, newest first. The original's own progress notes are in `ROADMAP.md` and `docs/`; they are not
edited here. Each entry matches commits on the `my-changes` branch.

## 2026-11-06

### Upstream round: GridCraft main 632e4a1b → ff35c6b8 (after v0.16.1)
- Merged GridCraft `main` (94 commits): form controls (buttons, sliders and spin buttons with Assign Action, Format
  Control and XLSX round trip, #481), whole-column/row paste no longer aborts (#2018), Set precision as displayed in
  calculation (#1366), RefEdit follow-ups (#3180), doughnut series as rings (#1827), confirmation before a
  programmatic link opens (#2181), XLSX memory caps (#2173, #2527), About credits that fit (#3320).
- Upstream's page host mode (`craft-web-host`, `data-host="page"`, #2592) is merged; `web.rs` keeps ours (start screen,
  drafts, Drive), with upstream's `Bridge::detect` choosing the host mode. The ready message reports `APP_VERSION`.
- Form-control icons (Button, Slider, Spin) added to the shared `krub-icons` sheet set (copied to the other two
  repositories) and routed there from `icons/content.rs`; our `icons.rs` (shared re-exports) kept.
- Thai: 25 strings, so `cargo xtask i18n-report` shows Thai 0 missing (form controls, hyperlink menu, Display
  options, Highlight the active cell's row, About, doughnut options, the header menu's Insert).
- The new Assign Action and Format Control dialogs (`control_ui.rs`) drew their own English labels, buttons and
  messages, which the report can't see; they now go through `Lang::tr`, with Thai templates for the messages that
  carry numbers. Titles are translated when the dialog opens.
- The Insert tab's form-control column uses `ribbon_layout::column`, as the Function Library does, so three small
  buttons fit the 72 px ribbon (`every_tab_fits_the_card_height`).

## 2026-11-02

### Google Drive (KhianKrub's, through the shared `krub-drive` crate)
- **Shared crate** `crates/krub-drive` (vendored byte-identical in all three KrubOffice repos, like `krub-icons`): KhianKrub's
  Drive code (Google sign-in, Picker, Drive transfers, the persistent connection service, the controller), made generic over
  the file type and free of egui. `DriveServices` lives there; `ui-egui` re-exports it as `Services::google_drive`.
- **File pages** (`crates/ui-egui/src/backstage/drive.rs`): on the web, Open and Save As show place cards, *This device* and
  *Google Drive* (Connect, Choose from Google Drive…, the app's own `.xlsx` files when there is no Picker key, Save to this
  file on Google Drive, Save a new copy, Disconnect, Try signing in again). A workbook opened from Drive shows Drive first.
  The start screen has a Google Drive button. `file.save` in `SheetApp::run_command` (File › Save, Ctrl+S, the title bar's Save) of a workbook opened from Drive replaces that Drive file
  (only if nobody changed it there since; Drive keeps the old version); other workbooks download as before.
- **Web shell** (`apps/gridcraft-web/src/web.rs`): a Drive file opens as a new workbook and is linked to it (by uid); a finished save marks it
  saved only if it has not changed since (same uid and revision). Drive's messages show as toasts, in Thai.
- **Settings**: KhianKrub's Google project. `packaging/web/build-local.sh` reads `~/.config/khiankrub/web-build.env`;
  the client ID, project number and Picker key are used only when `KHITKRUB_DRIVE_BACKEND_URL=https://drive.khitkrub.app`
  is set too, so the public site shows Drive only once the owner has set up the console and the Worker route. Without
  it, Drive works on localhost only, with Google's test client (its origin is `http://localhost:8771`).
- **Connection service**: KhianKrub's Worker now also answers at `drive.khitkrub.app` (its own origin, cookie and sessions).
- Thai for every Drive string (`crates/ui-egui/src/i18n/th.rs`); tests in `crates/ui-egui/src/backstage/drive_tests.rs`.

## 2026-10-31

### Thai sample workbook and Thai font names (owner's decisions of 2026-10-31)
- **A Thai sample workbook** (`crates/engine/src/sample.rs`, `thai`, opened by `?sample=thai` or the fourth start-screen tile "ยอดขายร้านกาแฟ"): a coffee shop's month in baht, written in Thai with sums, a
  profit share, a data bar, a colour scale and a chart; original content. The tile and title are translated through the catalog ("Coffee Shop Sales" in English). Also listed in the MCP `file.new`
  tool and `docs/cli.md`. Test: `the_thai_sample_computes_and_keeps_its_thai_text`.
- **Sarabun and TH Sarabun New in the Font list** (`ribbon::FONTS`), and a cell set in either name is drawn in it (`bind_host_font_names`: role families `fnt:TH Sarabun New:…` and `fnt:Sarabun:…`
  per weight and slant, led by the matching face). Default font untouched. The upstream keytips test `font_dropdown_highlight_follows_the_filtered_rows` types "ar", which both new names contain, so its
  expectation changed (six rows instead of four; Down four times instead of twice).
- Decisions recorded: no phone-screen block (not tested on iPads and phones, and said so); placeholder icon kept for v0.1.0.

### Release preparation (version 0.1.0, scripts, drafts; nothing published)
- **KhitKrub's own version**: `ui_egui::APP_VERSION = "0.1.0"` (About, Account, the host `gridcraft:ready` message, `publish.sh`). The GridCraft base stays 0.14.2 in the crates' `Cargo.toml`.
- **Release scripts** (`packaging/web/`): `build-local.sh` (release build; compiler paths renamed so the public wasm carries no `/Users/...`), `publish.sh [--skip-build] [--push]` (checks: fonts,
  licences, icons, manifest, no source files, no home-folder paths, relative URLs, files under 100 MB; commits the site into `target/khitkrub-web`, with `CNAME khitkrub.app`, `.nojekyll`, `images/` and the
  README from `site-README.md`; **works before the public repository exists** by using a local-only clone, and refuses `--push` until `chanonss/khitkrub-web` exists), `site-README.md`.
  A release build is 22.9 MB of wasm and took about 6 minutes.
- **Bug fixed before the first release: browser drafts of two tabs overwrote each other.** A document's uid restarts at 1 in every tab, so two tabs each starting a new workbook wrote to the same
  draft id (`doc-1`) and the later silently replaced the earlier. Draft ids now carry a token made once per page load (`doc-<token>-<uid>`). **PhutKrub's and KhianKrub's recovery code uses the
  same `doc-<uid>` scheme and very likely has the same bug**; they were not touched.
- **Drafts for the owner** (`packaging/web/drafts/`, not applied, not posted): `facebook-th.md`, `facebook-en.md`, `kruboffice-changes.md` (exactly what changes in the KrubOffice README, the launcher
  tile and the two other apps' app switchers when KhitKrub goes live).

### Web start screen and browser drafts (ported from PhutKrub and KhianKrub)
- **Start screen** (`crates/ui-egui/src/backstage/web_home.rs`): with no address asking for anything, the page shows a top bar (the gold ค on teal, KhitKrub, Open file,
  Options, the KrubOffice app switcher with KhianKrub and PhutKrub as live links), a Thai greeting, the new-workbook tiles (Blank, Household Budget, Quarterly Sales
  Dashboard, Gradebook) and the drafts kept in this browser. Each tile or draft opens **in a new tab**: `./?new=blank`, `./?sample=<name>`, `./?draft=<id>`; this page stays.
  GridCraft always keeps one workbook open, so the start screen is a state laid over the blank workbook the window opened with (`SheetApp::set_web_start_screen`,
  `web_start_screen`, `leave_web_start_screen`): it ends when something is opened in this tab (a picked or dropped file, a draft), and then the untouched blank workbook is closed.
  Options opens over it as a page of its own and Back returns to it. No start screen in host mode (`?host=parent`, the parent owns the files) or without `Services::browser_recovery`.
- **Browser drafts** (`apps/gridcraft-web/src/{recovery_protocol.rs, web/recovery.rs, web/recovery_store.rs}`, IndexedDB `khitkrub-recovery`): the open workbook is kept as `.xlsx` bytes two
  seconds after the last edit or every ten seconds of continuous editing (current + previous copy, checksum on read, at most 50 drafts, 64 MiB each, 256 MiB in all). A workbook nothing
  has changed gets no draft. `?draft=<id>` opens one as an unsaved workbook (its macros and scripts ask before running) and its first new draft replaces the old, so working on leaves
  one entry. The ⋮ menu offers Keep working, Download a copy (`.xlsx`) and Discard (after asking). A draft does not keep a VBA project (macros stored in the workbook's names survive);
  it keeps no undo history. Encoding runs on the UI thread, so the 64 MiB cap is the guard against a huge sheet.
- Thai: the start screen's strings are in `i18n/th.rs`; draft times use the Thai calendar for Thai (`toLocaleString("th-TH")`), e.g. 31/10/2569. The File page's New tiles now translate the
  sample names too, and the Thai name of "Quarterly Sales Dashboard" is short enough not to break in its tile (Thai has no spaces between words).
- Tests: `tests/it/web_start.rs` (drafts listed and opened in a new tab, ⋮ actions with the discard question, loading and empty states, tile addresses, Options and Back, opening a file
  leaves the start screen and drops the blank workbook, an edited workbook stays, Thai entries for every label, no start screen without drafts), `recovery_protocol` tests (scheduling,
  quotas, an `.xlsx` snapshot round trip including Thai text).
- Checked in the built-in browser (dev build): the Thai start screen, `?new=blank`, an edit kept as a draft 2 s later (IndexedDB), the draft listed with its Thai-calendar time, `?draft=` restoring the
  unsaved workbook with its formatting, and the draft replaced rather than copied. Not checked: a click opening a new tab (the pane did not show one), Download a copy, Discard, a draft of a large sheet.

### Teal band, gold highlights, KhitKrub name and icon (KhianKrub's design in KhitKrub's colour)
- **Title bar and tab strip are one teal band** in the app icon's colour, `#0F6E56` (`#095040` in the dark theme), with white title, tab names and icons.
  The active tab is a tab in the ribbon's colour whose bottom corners curve outward (8 pt) into a **flat ribbon** (no rounded card, no outline; one hairline
  under it). A collapsed ribbon is the band alone. Contextual tabs (Table Design, Chart Design) are gold on the band.
- **Gold**: the icon's gold, lightened to `#FFDA5C` because the icon's own `#F2B705` is only 3.4:1 on this teal and the rule is 4.5:1 (the other option,
  a deeper teal, would have broken the match with the icon and the loading screen). Used for the AutoSave switch, contextual tabs and the File page's current-page marker.
- **Gold hover and selection** everywhere the ribbon and menus draw them: hover `#FDF3D2`, pressed `#FAE39C`, selected `#FCEAB0` with a `#E0A800` outline
  (dark theme: `#403A28`, `#574B24`, `#4D421C`, `#C99608`). New tokens `selected` and `selected_border`; `widgets.rs` `fill_button`.
- **Accent is teal** (`#0F6E56`, dark `#4FC8A4`; soft tints and the sheet's selection border/fill and header highlight follow) instead of Excel-like green, so the
  sheet, the File page and the band agree. Chart and icon category colours are unchanged.
- **File page**: the side menu is the teal band (both themes) with white text and a gold marker on the current page; its icons are one colour.
- **Icons on the band are one colour** (white; gold on selected): `icons::paint_flat` draws every category colour in the ink, `Tokens::flat_icons` is true inside `band_ui`.
- Code: `theme::{BAND_TEAL, BAND_TEAL_DARK, BAND_GOLD}`, `Tokens::{band, band_ink, selected, selected_border, flat_icons}`, `Tokens::of(ui)`, `Tokens::on_band()`,
  `theme::band_ui(ui)` (the same mechanism as KhianKrub and PhutKrub: a Ui marked as band gets the band's colours, menus opened from it keep the normal ones);
  `ribbon.rs` (`active_tab_shape`, strip, flat ribbon, File as a painted tab), `ribbon/title_bar.rs`, `backstage.rs`, `widgets.rs` (`Tokens::of`, `paint_icon`).
  Tests: the `contrast_tests` pairs (WCAG 4.5:1 text, 3:1 glyphs) now include the band (white and dim text, gold, the band's ink on gold and on the active tab) and the
  selected surface, in both themes.
- **Name**: message-box titles, the Account page, the About dialog and its menu entries say KhitKrub (Thai: คิดครับ, via the Thai catalog). English keys changed for
  `About KhitKrub`, `About\nKhitKrub`, `How agents drive KhitKrub (MCP / control channel)`: **the other 18 languages' translations of those three labels now show English**
  (they were keyed on "GridCraft"). The About dialog no longer links ArtCraft's community or Discord (the brand licence forbids presenting a fork as an ArtCraft product);
  it says in plain text that this is a fork of GridCraft by the ArtCraft team and links the GridCraft repository and kruboffice.app. The desktop app (`apps/gridcraft`),
  CLI, packaging and crate names keep GridCraft's names.
- **Icon**: `assets/app-icon/khitkrub.svg` (+ 192/512 PNG, made with headless Chrome): the gold ค traced from Sarabun Bold on teal, the launcher's placeholder (the
  owner has not drawn a final one). `ATTRIBUTION.md` rows and `OFL-Sarabun.txt` added.
- **Web shell**: loading screen in the icon's teal with the icon, Thai name and a progress bar, Thai loading and failure messages (`loading_status`, `loading_failed`
  in `web.rs`), `theme-color` and web manifest (`manifest.webmanifest`, icons 192/512) in the same teal, favicon is the SVG icon.
- Checked in the built-in browser (dev build): loading screen, Thai ribbon in dark and light, File page, Account page, About dialog.
- Not done: the Account and About pages show GridCraft's version number `0.14.2` (KhitKrub's own `0.1.0` comes with the release scripts); the band colour pickers and Thai motifs
  KhianKrub has; the sheet-tab strip and status bar keep the window colour.

### Thai interface and Thai fonts on the web
- **Thai is the 20th interface language** (`Language::Th`, code `th`, shown as `ไทย`; `th`, `th-TH`... resolve to it).
  `crates/ui-egui/src/i18n/th.rs` has the same shape as the other catalogs (`CORE`, `DIALOGS`, `EXTRA`) and covers all
  1,169 interface strings any catalog translates, so `cargo xtask i18n-report` lists none missing for Thai. Written from the plain meaning of the
  English (clean room; the file header says so): Excel's Thai wording was not consulted, unlike the other catalogs. Common words are the ones
  KhianKrub and PhutKrub use; worksheet function names stay English. Spreadsheet words chosen here (not in the other apps): sheet = ชีต,
  workbook = เวิร์กบุ๊ก, cell = เซลล์, range = ช่วง, formula = สูตร, filter = กรอง, PivotTable = พิวอตเทเบิล, Name Box = กล่องชื่อ.
  Tests: `i18n::th::tests` (unique keys, none blank, every key of the Japanese reference catalog has a Thai entry, mostly Thai text) plus the
  shared shape tests (placeholders kept, one-line labels stay one line, no context prefix).
- **Fonts**: a browser has no system fonts, so `theme::set_host_faces` lets the host hand over Sarabun (regular, semibold) and TH Sarabun New
  (regular, bold, italic, bold italic). In Thai, Sarabun leads the interface families; everywhere else, and in every cell family, they are fallbacks at the end of each chain, so Thai letters still draw. (First version: TH Sarabun New led
  the cell families in Thai. **Changed the same day, on the owner's decision: new workbooks keep GridCraft's default font, Calibri 11 pt, and nothing makes TH Sarabun New the cell font.** Cells are now identical in Thai and English; the style-matching
  TH Sarabun New face is tried first only for letters the cell's own face lacks.) `theme::font_key` makes a switch between Thai and another
  language rebuild the fonts (it used to rebuild only when the Han order changed). Test: `host_faces_lead_the_interface_in_thai_and_trail_everywhere_else`.
- **Web shell** (`apps/gridcraft-web`): `site_fonts.rs` fetches the six faces from `fonts/` before eframe starts (a failure shows a Thai/English
  message instead of a blank page); `packaging/web/prepare-fonts.sh` copies them (sha256-checked) from KhianKrub's `target/web-fonts` into
  `target/web-assets/fonts`, and `index.html` copies that folder to `fonts/` (never committed). First visit opens in **Thai** whatever the browser's
  language; a saved language replaces it. `localStorage` keys are now `khitkrub.ui` and `khitkrub.prefs`; the page is `lang="th"`, titled
  "คิดครับ · KhitKrub", with element ids `khitkrub_canvas` / `khitkrub_loading`. The host-mode message names (`gridcraft:ready`...) are unchanged
  so embedding pages keep working.
- The no-font test loop (`interaction::language_switch_renders_the_ribbon_and_persists`) skips Thai, like Japanese, Korean and Chinese, when the
  optional craft-fonts input is absent.
- Checked in the built-in browser (dev build of `apps/gridcraft-web`): first visit opens in Thai (ribbon tabs and buttons, formula bar, File page, Options), the
  language picker lists `ไทย` and English, switching to English applies at once and is remembered across a reload under `khitkrub.ui`, no console errors.
- Not done yet: the three template tiles on the File page (Household Budget, Quarterly Sales Dashboard, Gradebook) stay English (they bypass the catalog), and
  so does the sample workbook's own text; Thai for strings that are not in any catalog (shown in English), Thai digits, พ.ศ.
  dates, baht text, Thai collation and word breaking (see `HANDOFF-KHITKRUB.md` §3.2).

### The fork itself
- Forked GridCraft 0.14.2 (upstream `981fad14`, `main`). Remotes: `upstream` (storytold/gridcraft), `web-source`
  (private `chanonss/khitkrub-web-source`; `my-changes` pushed 2026-10-31). Branch `my-changes`. Web only; no desktop app work.
- Removed `docs/brand/` (the ArtCraft marks), as `docs/brand/LICENSE-brand.txt` requires of modified versions. The
  README says in plain text that KhitKrub is based on GridCraft by the ArtCraft team. `NOTICE` keeps the upstream
  copyright and licence lines (MIT or Apache-2.0, at your option) and drops the sentence about the brand files.
- `CLAUDE.md` now opens with what this project is and the owner's rules; `README.md` is bilingual (Thai/English);
  `HANDOFF.md` (what to read first, state, next steps) and `HANDOFF-KHITKRUB.md` (the fork playbook written from
  the KhianKrub and PhutKrub work) are new.
- Not done yet (see `HANDOFF.md`): Thai interface, Thai fonts on the web, KhitKrub name and colours in the app,
  loading screen and manifest, start screen with browser drafts, release scripts.
- Upstream baseline: see `HANDOFF.md` ("Checks last run").
