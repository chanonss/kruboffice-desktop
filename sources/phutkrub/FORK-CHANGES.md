# Changes in this fork

This is Chanon Sarasuk's fork of [storytold/deckcraft](https://github.com/storytold/deckcraft), kept for a
Thai-speaking audience and called **พูดครับ · PhutKrub**. It follows KhianKrub (the WordCraft fork in `../khiankrub-web`).
This file lists what differs from the original, newest first. The original's own progress notes are in
`ROADMAP.md` and `docs/`; they are not edited here. Each entry matches commits on the `my-changes` branch.

## 2026-11-06

### Upstream round: DeckCraft main 23ac4f7e → 9a95c3a6 (after v0.15.1)
- Merged DeckCraft `main` (76 commits). Brings the web Presenter View fixes (#1870: Previous and Next work in the
  pop-up; #1872: All slides opens in the pop-up, Enter and Space pick a focused thumbnail), Presenter View only with a
  second display on Linux and the BSDs (#1156; the web, macOS and Windows are unchanged), copy and paste of pictures and placeholders (#1658), Grayscale thumbnails (#1886),
  slide numbers in the PDF text layer (#1153), outlines without footer/date/number placeholders (#1879), Show media
  controls kept in PPTX (#1474), connectors glued inside groups (#1876), and bounds for fonts, Ogg and repair.
- Upstream's host modes (`craft-web-host`: `?host=parent` and page host mode, #1809, #1895) are merged but unused by
  the site. `apps/deckcraft-web/src/web.rs` runs both: the bridge connects to the same services as Drive and drafts,
  and in host mode the start screen's address parameters are ignored. `SlideApp::save` tries a Drive file first, then
  the host page, then the path.
- Conflicts kept ours where they were branding, Thai or Drive; upstream's About credits layout (#1806) taken with our
  Thai labels. Thai for the presenter's All slides heading and hint (`i18n/th.tsv`).
- Fix found while checking: the Slide Show tab's three checkboxes (Use Timings, Play Narrations, Show Media Controls)
  and the show's own toolbar (Black, White, All, Zoom, End) were English in the Thai interface; now through `i18n::t`.

## 2026-11-02

### Google Drive (KhianKrub's, through the shared `krub-drive` crate)
- **Shared crate** `crates/krub-drive` (vendored byte-identical in all three KrubOffice repos, like `krub-icons`): KhianKrub's
  Drive code (Google sign-in, Picker, Drive transfers, the persistent connection service, the controller), made generic over
  the file type and free of egui. `DriveServices` lives there; `ui-egui` re-exports it as `Services::google_drive`.
- **File pages** (`crates/ui-egui/src/backstage/drive.rs`): on the web, Open and Save As show place cards, *This device* and
  *Google Drive* (Connect, Choose from Google Drive…, the app's own `.pptx` files when there is no Picker key, Save to this
  file on Google Drive, Save a new copy, Disconnect, Try signing in again). A presentation opened from Drive shows Drive first.
  The start screen has a Google Drive button. `SlideApp::save` (File › Save, Ctrl+S, the toolbar's Save) of a presentation opened from Drive replaces that Drive file
  (only if nobody changed it there since; Drive keeps the old version); other presentations download as before.
- **Web shell** (`apps/deckcraft-web/src/web.rs`): a Drive file opens as a new presentation and is linked to it (by uid); a finished save marks it
  saved only if it has not changed since (same uid and revision). Drive's messages show as toasts, in Thai.
- **Settings**: KhianKrub's Google project. `packaging/web/build-local.sh` reads `~/.config/khiankrub/web-build.env`;
  the client ID, project number and Picker key are used only when `PHUTKRUB_DRIVE_BACKEND_URL=https://drive.phutkrub.app`
  is set too, so the public site shows Drive only once the owner has set up the console and the Worker route. Without
  it, Drive works on localhost only, with Google's test client (its origin is `http://localhost:8771`).
- **Connection service**: KhianKrub's Worker now also answers at `drive.phutkrub.app` (its own origin, cookie and sessions).
- Thai for every Drive string (`crates/ui-egui/src/i18n/th.tsv`); tests in `crates/ui-egui/src/backstage/drive_tests.rs`.

## 2026-10-31

### Start screen with browser drafts; white line fixed; READMEs
- **Web start screen** (`crates/ui-egui/src/backstage/web_home.rs`, ported in design from KhianKrub): with nothing open and a host that
  keeps drafts (`Services::browser_recovery`), the empty window shows a top bar (the coral พ mark, PhutKrub, Open file, Options, app
  switcher with KhianKrub as a live link), a Thai greeting, Blank and the built-in themes, the drafts kept in this browser, and the
  starter decks. No title bar there (`SlideApp::web_start_screen`). Each tile opens in a new tab: `./?new=blank`, `./?theme=<name>`,
  `./?template=<name>`, `./?sample`. A new deck of a plain theme is "not edited" (the Thai default font does not count as an edit).
- **Browser drafts** (`apps/deckcraft-web/src/web/recovery.rs`, `recovery_store.rs` (IndexedDB `phutkrub-recovery`), `recovery_protocol.rs`):
  the open presentation's native bytes (`deckcraft_format::save`) are kept two seconds after you stop editing, or every ten seconds of
  continuous work; the latest and the previous copy; at most 50 drafts, 64 MiB each, 256 MiB in all; a checksum is checked on read. `?draft=<id>`
  opens one as an unsaved presentation and its first new draft replaces the old; ⋮ offers Keep working, Download a copy (`.pptx`) and Discard
  (after asking). Not ported from KhianKrub: the recovery banner and manager dialog, the Options switch to turn it off, Google Drive.
- Thai faces are now fallbacks in the English interface too, so the Thai letters in the logo and app names draw.
- The white line between the title bar and the tab strip is gone (`show_separator_line(false)` on both panels, as in KhianKrub).
- READMEs: `packaging/web/site-README.md` (the public site repo's) and `README.md` show the new screenshots (`docs/images/phutkrub-*.png`);
  the KrubOffice README (apps' origins, bundle, features, screenshots) is `../khiankrub-web/packaging/web/kruboffice/README.md`.
- Not done (Presenter View became Thai and says PhutKrub on 2026-11-05); the sample deck's speaker note says "made in DeckCraft";
  theme and starter-deck names stay English.

### Coral band and gold highlights (KhianKrub's design in PhutKrub's colour); coral loading screen
- **Title bar and tab strip are one coral band** in the app icon's colour, `#9A3A1F` (`#6B2915` in the dark theme),
  as KhianKrub's is its icon's navy, with white title, tab names and icons. (Chanon first saw KhianKrub's navy in the
  reference screenshots; PhutKrub's icon is coral, so the band is coral.) The active tab is a white tab whose bottom
  corners curve outward into the ribbon (8 pt, light and dark), its name in coral (normal text colour in the dark
  theme). The ribbon is a flat white strip with no rounded card. Gold marks what stands out on the band: the
  **Present** button (coral text) and contextual tabs (Shape Format, Table Design...). The gold is `#FFCB2E`, a touch
  lighter than the icon's `#F2B705`, because the icon's gold reaches only 3.9:1 on this coral. A collapsed ribbon keeps
  a gold underline under the active tab.
- **Gold hover and selection** everywhere (buttons, menu rows, gallery tiles, panels): pale gold on hover
  (`#FDF3D2`), a gold tint with a gold outline when on or selected (`#FCEAB0`, outline `#E0A800`), stronger gold while
  pressed; muted golds in the dark theme. Alternate table rows keep a neutral grey.
- **File page**: the side menu is the same coral, current page in gold.
- **Icons on the band are one colour** (white; gold when selected): the icons' fixed accent colours (blue Save, green
  Present...) do not show on coral, so `icons::paint_ex(.., flat)` draws every colour in the ink, and `Tokens::flat_icons`
  is true inside `band_ui` and on the File page's side menu.
- Code: `theme::Tokens` gained `band`, `band_ink`, `selected_border`, `Tokens::on_band()`, `Tokens::of(ui)` and
  `theme::band_ui()` (the same mechanism as KhianKrub); `widgets.rs` reads `Tokens::of(ui)`, so anything drawn inside
  `band_ui` gets band colours while menus opened from it keep the normal ones; `ribbon.rs` draws the band and
  `active_tab_shape`; `ribbon/title_bar.rs`, `backstage.rs`. The constants are `BAND_CORAL`, `BAND_CORAL_DARK`,
  `BAND_GOLD`. Tests: `the_band_is_readable`, `text_is_readable_on_the_gold_tints` (4.5:1).
- Not ported from KhianKrub: the band colour and Thai motif pickers (File › Options), which are a larger feature. The
  accent (selected slide thumbnail outline, text caret...) stays DeckCraft's orange-coral.
- The loading screen, `theme-color` and web manifest use the same coral as the page colour, as KhianKrub uses its
  logo's navy.

## 2026-10-30

### First cut: Thai interface, Thai fonts, PhutKrub web shell
- Forked DeckCraft 0.12.0 (upstream `3a4e30e1`). Remotes: `upstream` (storytold/deckcraft), `web-source`
  (private `chanonss/phutkrub-web-source`). Web only; no desktop app work.
- **Interface languages** (`crates/ui-egui/src/i18n`, ported from KhianKrub): English plus Thai. Code strings stay
  English and are the keys (`tl!`), `th.tsv` holds the Thai text, a missing entry shows in English. The `language`
  setting (`auto`, `en`, `th`) is in the saved UI state, and `ui.language` sets it from the control channel.
  About 730 string literals in the ribbon, menus, dialogs, panes and File pages were wrapped by script; the shared
  widgets (`big_button`, `small_button`, `check_item`, `spinner`, `section`...) also translate their labels.
- **Thai catalog**: ~590 entries, written from the plain meaning of the English (clean room).
- **Thai fonts**: Sarabun is the interface font in Thai (`theme::install_fonts_for`); the web loads Sarabun and TH Sarabun
  New before the first frame (`apps/deckcraft-web/src/site_fonts.rs`) and new decks use TH Sarabun New.
- **Web shell**: Thai loading screen and messages, first visit opens in Thai, the language is remembered in the
  browser, a blank Thai deck opens by default (`?sample` for the sample), Thai web manifest.
- **Icon and name**: พ on coral from the KrubOffice launcher (`assets/app-icon/phutkrub.svg`).
- Removed `docs/brand/` (ArtCraft marks), as its licence requires of modified versions; the README says in plain
  text that PhutKrub is based on DeckCraft by the ArtCraft team.

### Checked
- `i18n` tests pass, including the coverage test for every ribbon tab and File page. The release web build runs in the
  browser pane: Thai ribbon and File page, Sarabun interface, a blank deck in TH Sarabun New. The full UI suite has one
  failure that also fails on pristine upstream (it needs the optional craft-fonts bundle for a ⌥ glyph).

### Not done yet
- Still English in the browser: the title bar ("Presentation1 — Edited"), placeholder prompts on the canvas ("Click to
  add subtitle"), the status bar ("Slide 1 of 1", the proofing language), the contextual "Shape Format" tab, theme,
  template and layout names, the "Join the ArtCraft community" link on the File page, "Font…" and "Paragraph…" on
  the ribbon, most dialog bodies and tooltips that carry a shortcut.
- "DeckCraft" in English mode (the Thai catalog maps it to PhutKrub).
- The other SIPA and TLWG Thai font families (KhianKrub loads them on demand), Thai word breaking and proofing in slide
  text, Thai sample deck and templates, Thai test deck.
- Google Drive (skipped on purpose).

## Release
`bash packaging/web/publish.sh [--push]` builds the site (`build-local.sh`, with neutral build paths), checks it and
commits it to the public site repository `chanonss/phutkrub-web` (GitHub Pages, `CNAME` `phutkrub.app`, v0.1.0). The
source stays here, private. The domain is at Cloudflare (DNS only, grey cloud, as for KhianKrub): four `A` records to
GitHub Pages and a `www` CNAME to `chanonss.github.io`. The KrubOffice launcher (`kruboffice.app`) is published from
`../khiankrub-web` (`packaging/web/publish-kruboffice.sh`) and shows the พ tile.
