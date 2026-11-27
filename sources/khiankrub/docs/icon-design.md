# KrubOffice icon design

Status: 2026-11-25 · ICON.20 accent colours (style A) on every Tabler icon in KhianKrub, except the flat white ones on the blue band (top bar, File and start-screen side).

Quiet Glass uses original 24×24 drawings, usually inside a 20×20 area, with 1.75-unit outlines and round caps. Display sizes remain controlled by each app. At 16 px the drawings omit fine details and keep at least a 1 px stroke. Foreground ink follows the caller, including its alpha; gold tints disappear on flat band icons and disabled icons. Common action mappings change painting only, never command IDs, tooltips, enabled state or hit targets.

`crates/krub-icons` is an identical vendored library in KhianKrub, KhitKrub and PhutKrub. Each copy uses the host workspace's egui version (0.36 or 0.37), so a checkout builds independently. Changes to this library must be copied to all three repositories and checked with `python3 packaging/check-icon-sync.py <sibling-repo> <sibling-repo>`; no cross-repository path dependencies. The catalog includes 574 distinct names (including aliases), spanning 318 document icon spellings, 131 spreadsheet variants and 246 presentation variants, plus document menu diagrams. Aliases route common actions to identical geometry. Specialized symbols retain their original meaning, adapted from contributor-original Rust artwork to shared proportions, stroke weight and neutral ink with translucent gold surfaces. This consolidates and restyles the existing openly licensed drawings; it does not claim every path was newly invented. Content previews (actual inserted pictures, shape/style galleries, worksheet and document colours) continue to depict the content they represent.

Logo masters are the app-named SVGs under `assets/app-icon/`. Their gold glass treatment comes from the reviewed richer-gold draft; the existing Thai contours are from Sarabun Bold, SIL OFL 1.1, with its licence beside the artwork. Web loading screens embed the master; install icons are 192/512 px PNG renderings. Start-screen marks use a clipped colour gradient over the app's existing Thai font, so they remain usable without an SVG loader. Native installer icons and the separate KrubOffice launcher are outside this first preview.

## Validation history

ICON.1 initial preview, 2026-11-01: native offscreen application previews captured and inspected; icon tests (finite tessellation at 16/20/24/32 px in normal, flat and disabled states; invalid bounds ignored), shared-crate Clippy with warnings denied, formatting, asset attribution, dependency layering and UI WASM checks passed in all three repositories. Vendored library equality checked. Full workspace CI and complete icon migration remain pending; no commit, push or publication in this preview stage. Existing unused-patch, optional craft-fonts and WASM dead-code warnings remain.

## Balanced toolbar gold · ICON.3

The owner approved Balanced after reviewing the palette comparison. Toolbar gold is `#F2B705` with primary alpha 230/255 and surface alpha 107/255. Caller alpha and drawing opacity are multiplied into those strengths, without a minimum opacity floor. The deck toolkit identifies soft surfaces through its explicit tint role, rather than classifying rendered pixel alpha. Document fills and spreadsheet primary rectangles share the same primary role; worksheet UI washes and surface rectangles use the soft role. Spreadsheet rectangles resolve their surface role once, avoiding double tinting. Flat band icons and disabled controls remain monochrome, including specialized document/spreadsheet routes. The preserved workbook content renderer and document artwork are unchanged.

## Revision history

| Date | Change |
|---|---|
| 2026-11-25 | ICON.20: style A in the dialogs' page lists, the account menu and More's tray. |
| 2026-11-25 | ICON.19: style A on the right-click menu, the mini toolbar and the status bar. |
| 2026-11-25 | ICON.18: style A on the last tabs (Draw, Mailings, Developer, Zotero, Help, Outlining, Equation); Expand drawn as a plus. |
| 2026-11-25 | ICON.17: style A on KhianKrub's tool panes and the panel menu (side panels, batch 3). |
| 2026-11-24 | ICON.16: style A on KhianKrub's object panes (side panels, batch 2). |
| 2026-11-24 | ICON.15: style A on KhianKrub's side panels, batch 1. |
| 2026-11-24 | ICON.14: style A on KhianKrub's View ribbon. |
| 2026-11-24 | ICON.13: style A on KhianKrub's Review ribbon. |
| 2026-11-24 | ICON.12: style A on KhianKrub's References ribbon. |
| 2026-11-24 | ICON.11: style A on KhianKrub's Layout ribbon; coloured solid shapes. |
| 2026-11-24 | ICON.10: style A on KhianKrub's Design ribbon. |
| 2026-11-24 | ICON.9: style A on KhianKrub's Insert ribbon; tint-only parts. |
| 2026-11-24 | ICON.8: style A accent colours on KhianKrub's Home ribbon. |
| 2026-11-01 | ICON.3: approved Balanced gold roles; preserve dimmed alpha and monochrome flat/disabled states. |
| 2026-11-01 | ICON.1: shared library and first common action mappings; richer gold glass web logos and start-screen lettering. |

| 2026-11-01 | ICON.2: full shared catalog, common action aliases, document-menu migration, complete inventory/mesh checks and gallery tooling. |

## ICON.2 final verification · 2026-11-01

Full `cargo xtask ci` passed. After the final round-cap adjustment, all 9 shared-catalog tests, Clippy and the UI WASM check passed again. Native screenshots and the full 573-name gallery were rendered and inspected.

The three vendored library copies are byte-identical. Commands, shortcuts and document format handlers retain their IDs and behaviour. Changes remain local and uncommitted; no GitHub push, deployment or social publication was performed.

## ICON.3 verification · 2026-11-01

All 11 shared-icon tests pass against egui 0.36.2 and the exact egui 0.37 fork revision used by KhitKrub/PhutKrub. Library Clippy with warnings denied and library WASM checks pass for both. Source formatting and vendored-library equality checks pass. The equality helper ignores Finder's `.DS_Store` metadata while comparing every source/configuration file.

Actual tessellated source icons were rendered without a GPU and visually inspected in light/dark themes, including 32/16 px, dimmed caller opacity, disabled and flat states. The complete 573-name catalog was rendered and inspected at 28/16 px. These are source-icon previews, not complete application ribbon screenshots.

Validation used isolated manifests and a source snapshot verified byte-identical to all three vendored copies. Full workspace CI, whole-app WASM checks and complete app screenshots were not rerun during ICON.3; the broader gate limitations in ICON.2 remain to be resolved before committing/releasing. No commit, push, deployment or social publication was performed.

## Full local web previews · 2026-11-01

Rebuilt all three complete web apps from the updated working trees into separate temporary preview folders. The sample document (KhianKrub), sales workbook and charts (KhitKrub), and Northern Lights presentation (PhutKrub) loaded successfully in the in-app browser with Thai UI labels and the Balanced icons. No runtime errors appeared in the captured browser logs; the existing unsupported web viewport title/theme warnings are non-fatal. Home toolbars were visually inspected in all apps; KhianKrub's Insert ribbon was also checked. This is a browser startup/visual smoke check, not full workspace CI.

Local links: KhianKrub `http://127.0.0.1:8771/?sample`, KhitKrub `http://127.0.0.1:8772/?sample=sales`, PhutKrub `http://127.0.0.1:8773/?sample`. The servers bind only to loopback. Fresh WASM hashes differ from the old pre-redesign `dist/web` bundles. HTML assets and manifest icon paths were checked. No public release or deployment.

## Approved Thai Distribution icon · ICON.6 · 2026-11-01

The owner reviewed `thai-distribution-square-draft.png`, approved it, and requested Thai Distribution in PhutKrub too. The icon uses an 18×18 square on the 24-unit master with a 1.75-unit neutral outline, Balanced soft gold fill, and a centered ท derived from the existing Sarabun SemiBold face. The outline is preflattened Rust geometry with a true counter/hole, so the glyph is accurate and does not depend on installed Thai fonts. Its SIL OFL licence is included in `crates/krub-icons/OFL-Sarabun.txt` and attributed. Flat and disabled states keep the established monochrome/alpha rules. Named document `thaiDistribute` and presentation `Icon::ThaiDistribute` / `thai-distribution` routes use identical painting. No font binaries were added.

PhutKrub gains Home › Paragraph › Thai Distribution immediately after Justify, plus the corresponding paragraph menu/dialog option and Thai tooltip. `format.thaiDistribute` is a command-registry entry with normal enabled state and undo/redo. The model preserves a distinct Thai alignment (`thaiDist` in DrawingML) instead of importing it as ordinary Justify. Wrapped lines share space at Thai grapheme/cluster boundaries and spaces; combining marks remain attached, Latin letters within words retain their spacing, and final paragraph lines remain unstretched. Generic alignment/paragraph command parameters accept `thaiDistribute` too. KhianKrub keeps its existing command/behaviour and receives the new icon only. KhitKrub's vendored library is synchronized without adding a spreadsheet command.

Targeted validation details and final build identifiers are recorded in the local handoff. Full workspace CI remains a separate gate before commits or release.

| Date | Change |
|---|---|
| 2026-11-01 | ICON.6: approved font-independent square ท icon and PhutKrub Thai Distribution. |

### ICON.6 final verification

All 12 shared-icon tests passed on both egui versions; library Clippy with warnings denied passed for both; vendored sources are identical. All three asset-attribution checks passed. PhutKrub's two command/PPTX/undo tests, two Thai/mixed-language layout tests, and the native ribbon-click regression passed. The latter verifies the button follows Justify and applies `Align::ThaiDistribute`. Complete KhianKrub and PhutKrub web builds passed and their new buttons were visually inspected and clicked without browser runtime errors. Formatting and whitespace checks passed. Full workspace CI was not rerun; no commit or publication.

## Clipboard icon pilot · 2026-11-08

Chanon requested a small set before the larger icon redraw. KhianKrub has original vector drawings for Paste, Cut, Copy and Format Painter in `crates/ui-egui/src/icons.rs`, on a shared 24-unit grid with consistent stroke and restrained gold. The pilot is intentionally local to KhianKrub pending owner review, so the shared vendored `krub-icons` crate remains unchanged. It uses the existing ribbon commands, hit targets and tooltips. The set was inspected at the actual 20 px and 32 px ribbon sizes in light and dark themes. The 27 ribbon tests, UI Clippy, WebAssembly check, formatting and diff checks passed; full CI and broader rollout remain.

## Home ribbon icon set · 2026-11-08

After the Clipboard pilot, Chanon requested the rest of KhianKrub's Home ribbon first. Original vector artwork on the same 24-unit grid now covers the Home font, paragraph, editing, voice and editor actions as well as Clipboard. The previously approved Thai Distribution icon remains unchanged, and style previews and navigation chevrons retain their existing purpose. The artwork is currently local to KhianKrub; commands, tooltips and button behavior are unchanged. Home icons reused elsewhere in KhianKrub also use the new artwork. Further ribbons and the other apps have not been redesigned in this pass.

### Home submenu icon follow-up · 2026-11-08

The Home submenu pass covers the existing Paste, Find and Select menu icons, plus original border-menu diagrams for each edge, inside and diagonal option, horizontal line, table drawing, gridlines, and borders/shading. Active edges use dark ink and inactive edges use continuous light grey guides; disabled entries retain the disabled ink. The border menu is shared with Table Design, so that menu receives the same diagrams. Text-only choices and content previews remain text/content, and existing menu commands and enabled states are unchanged.

## Insert ribbon icons · 2026-11-08

KhianKrub's Insert ribbon and its table and page-number submenu diagrams use original vector artwork on the Home set's 24-unit grid. Shared actions reuse the Home set. Content galleries keep showing the actual shapes, equations, symbols and WordArt they insert. The artwork is local to KhianKrub for review; command behavior remains unchanged.

## Draw ribbon icons · 2026-11-08

KhianKrub's Draw ribbon uses the same original vector style, including pen tools, colour and thickness, conversion, canvas and replay. Add Pen has matching menu icons; the pen gallery's colour/thickness swatches remain functional content indicators. Shared icon names can update other contexts too.

## View ribbon icons · 2026-11-08

KhianKrub's View ribbon uses matching original vectors for view modes, page movement, Show, Zoom, theme, window, macro and template commands. The previous View layout and commands remain unchanged. Text-only submenu choices remain text.

## Layout ribbon icons · 2026-11-08

KhianKrub's Layout ribbon uses matching original vectors. Existing text-direction, margin, orientation, column, break and line-number submenu diagrams were redrawn to distinguish each option while retaining their commands and selected states. The paper-size and arrangement menus that use text-only options remain text.

## Design ribbon icons · 2026-11-08

KhianKrub's Design ribbon and its existing Paragraph Spacing submenu icons use the original vector style. Style/theme previews and color palettes continue to show the actual document choices; text-only entries remain text.

## References ribbon icons · 2026-11-08

KhianKrub's References ribbon uses matching original vector icons, with distinct images for table of figures and table of tables. Existing text-only menus remain text and reference commands are unchanged.

## Review ribbon icons · 2026-11-08

KhianKrub's Review ribbon uses matching original vectors for its proofing, speech, accessibility, language, commenting, change tracking and protection controls. Existing text-only submenu entries are preserved.

## Picture Format and Table Layout icons · 2026-11-08

KhianKrub's Picture Format distinguishes the picture adjustment actions and has matching crop and alt-text icons. Table Layout includes matching table actions, nine distinct cell-alignment diagrams, and Select/Delete/AutoFit submenu diagrams. Existing commands and selected states remain unchanged; common icons may be reused in other tabs.

## Tabler Icons · ICON.7 · 2026-11-14

The owner compared every icon with Tabler Icons candidates and approved the default picks. Most ribbon, toolbar and menu icons are now drawn from [Tabler Icons](https://tabler.io/icons) 3.50.0 (MIT, Paweł Kuna). Flaticon UIcons were rejected first: their licence forbids sublicensing and redistributing the icons as a collection, which an MIT/Apache repository does.

- `crates/krub-icons/src/tabler.rs` is generated by `tools/tabler_rs.py` from Tabler's outline SVGs and `tools/tabler-map.tsv` (614 KrubOffice names → 341 Tabler drawings). Arcs and quadratic curves become cubic Béziers; the three filled shapes are pre-triangulated. Regenerate instead of hand-editing.
- `tabler::paint` runs first in `paint_named`, `line::paint_ex`, `deck::paint_ex`, the spreadsheet palette painter and the document painter, so every route draws the same Tabler icon. Names not in the map keep their KrubOffice drawing: the square ท Thai Distribution, Thai digits, the eight function-library books, the document-menu diagrams and 52 line-set preset diagrams (margins, paragraph spacing, page-number positions, columns, breaks, cell alignment, table selection, line numbering, text direction). A second review the same day (batch 2) moved the Options page, picture tools, table delete and autofit, portrait/landscape, ink colour and similar icons to Tabler.
- Strokes are 1.75 units on the 24-unit grid (never under 0.9 px) with round ends drawn as half-discs, so faded ink does not overlap itself. Tabler icons are monochrome: no gold surface. Disabled icons use 35% of the caller's ink, as before.
- Licence: `LICENSE-tabler.txt` beside the crate, `NOTICE`, `ATTRIBUTION.md`, the README and Help › About (credit line, link, and the full licence text in a collapsible section).
- Side-by-side duplicates fixed after the owner's preview check: 100% zoom (`zoom-reset`), Select / Split Cells / Split Table / Draw Table, Endnote (`file-text`), Ungroup (`arrows-split-2`), Save As (`file-export`), Immersive Reader (`eyeglass`). Increase / Decrease Decimal keep their KrubOffice drawings, since Tabler has a single `decimal` icon.
- KhitKrub's Automate tab had one `</>` icon on every button. New spreadsheet icons `Agent` (`robot`), `Journal` (`history`) and `Updates` (`refresh`) are drawn only from Tabler; Run Macro uses Play, Command Palette uses Search, and Macros uses Tabler `script`. `line::sheet::paint` now tries Tabler first, like the other painters.
- KhitKrub View › Zoom showed three magnifiers: new spreadsheet icons `ZoomReset` (`zoom-reset`, the 100% button, as in KhianKrub) and `ZoomSelection` (`zoom-in-area`) are drawn only from Tabler.

## Accent colours, style A · ICON.8 · 2026-11-24

Chanon compared monochrome, A (accent) and B (full colour) drafts of KhianKrub's Home ribbon and chose **A** for Home first; other ribbons follow one at a time. Tabler outlines are unchanged; the part of an icon that carries its meaning is drawn in colour, sometimes with a soft tint inside (18% on light themes, 26% on dark), and the rest keeps the caller's ink. Bold, Italic and the four alignment icons stay ink. Any colour may be used, not only the gold: blue `#2563EB`, red `#DC2626`, green `#16A34A`, orange `#EA580C`, purple `#7C3AED`, teal `#0D9488`, gold `#D99A00`, each with a lighter dark-theme shade chosen when the ink is light.

- `crates/krub-icons/src/tabler_colour.rs` (hand-written) holds the palette and the table of coloured outlines per Tabler drawing (outline index: every `M` or circle starts one). It is separate from the generated `tabler.rs`, so regenerating Tabler keeps the colours. The generator template (`tools/tabler_rs.py`) and `tabler.rs` gained the same small hook: `paint` looks up the table while colour is on.
- Colour is off by default. A caller turns it on for a stretch of drawing with `tabler_colour::with_colour(ctx, on, ..)`. KhianKrub does so in `ribbon.rs` `draw_tab` while the Home tab draws (its line, its More tray and its menus). Disabled icons and flat white icons stay monochrome; the hue takes the ink's opacity.
- Tests: coloured outlines exist in their drawings, colour only when asked and never when disabled, concave tints triangulate fully, theme shade and alpha.
- The colour hook changes the shared crate in KhianKrub only; KhitKrub and PhutKrub keep their copies until colour reaches them (their copies already differed in `chart_pane.rs` / `shape_pane.rs`).

### Insert ribbon · ICON.9 · 2026-11-24

Chanon approved the Insert draft (`~/dev/KrubOffice/ui-experiments/khiankrub-insert-icon-colours-draft.html`) and `draw_tab` now colours Home and Insert. Same colour language: green for add (Comment, Insert Table plus), red for marks (Bookmark, Date & Time, the X in Icons), blue as the main hue, purple for text tools (Quick Parts, Drop Cap, Signature, Symbol); Pictures, Chart, Icons and Diagram are livelier. Pages and Table keep their ink outline with only a blue tint inside: a new tint-only part (`f`) in the table, because a fully blue outline read as style B. Colours belong to the drawing, so Link and Cross-reference match and WordArt shares Font Colour's red bar.

### Design ribbon · ICON.10 · 2026-11-24

Chanon approved the Design draft (`~/dev/KrubOffice/ui-experiments/khiankrub-design-icon-colours-draft.html`); `draw_tab` now colours Home, Insert and Design. New: red/green/blue paint dots on the palette (Themes and Theme Colors), purple on the Fonts letter, gold star (Set as Default), gold folder (Themes menu), blue drop (Watermark), blue frame (Page Borders). Paragraph Spacing, Effects and Page Color keep the colours of the Home drawings they share. Menu diagrams and galleries stay content previews.

### Layout ribbon · ICON.11 · 2026-11-24

Chanon approved the Layout draft (`~/dev/KrubOffice/ui-experiments/khiankrub-layout-icon-colours-draft.html`); `draw_tab` now colours Home, Insert, Design and Layout. Blue marks what changes on the page (text-direction arrow, Margins content box, Size arrows, the moving layer, Selection Pane divider, Align object); Columns is blue and teal; Breaks and Hyphenation are orange like Page Break; Orientation is tint-only (an orange stroke would have coloured only half its arrowhead). Line Numbers shares Home's Numbering colours. New `tabler_colour::SOLID` part index colours a drawing's pre-filled shapes (Bring Forward / Send Backward); the generator template and `tabler.rs` share that lookup.

### References ribbon · ICON.12 · 2026-11-24

Chanon approved the References draft (`~/dev/KrubOffice/ui-experiments/khiankrub-references-icon-colours-draft.html`); `draw_tab` now colours Home, Insert, Design, Layout and References. Purple is the notes and citations family: the footnote number (Home's Superscript), the endnote's last line, Show Notes' text, the quote marks, Manage Sources' person, Bibliography's leaning book. Blue marks what changes or moves: the Update arrows (`refresh`), Next Footnote's arrow, the caption lines, the index letters (as Numbering's digits), the options "i", and Researcher's lens (as Find) with an orange question mark. Mark Entry's tag and Mark Citation's bookmark are red like Bookmark, with green plus signs; the gavel head (Table of Authorities) is orange. Table of Contents, Footnote, Style and Cross-reference reuse the Numbering, Superscript, brush and Link colours. Mark Citation's bookmark outline is open at the plus, so its tint ends in a short diagonal there.

### Review ribbon · ICON.13 · 2026-11-24

Chanon approved the Review draft (`~/dev/KrubOffice/ui-experiments/khiankrub-review-icon-colours-draft.html`); `draw_tab` now colours Home, Insert, Design, Layout, References and Review. Purple for the word tools (Spelling's "Abc", Thesaurus' page curves). Blue for what moves or speaks: the Previous/Next arrows (comments: the arrow inside the bubble; changes: `arrow-up`/`arrow-down`, the same as Next Footnote), Read Aloud's waves, Word Count's digits, the person in Check Accessibility (including its solid head), the "A" in Translate and Language, the dots in Display for Review and Reviewing Pane, and the arrows of Track Changes and Compare (one drawing). Green for Accept's check and New Comment's plus; red for Reject's and Delete Comment's X and Hide Ink's slash. Gold for Show Comments' back bubble (like the comment markers) and the Protect shields (like the object lock badge).

### View ribbon · ICON.14 · 2026-11-24

Chanon approved the View draft (`~/dev/KrubOffice/ui-experiments/khiankrub-view-icon-colours-draft.html`); `draw_tab` now colours Home, Insert, Design, Layout, References, Review and View. Six drawings keep their colours from earlier tabs: Read Mode (Thesaurus' book), Print Layout (Endnote's page, so its last line is purple), Outline (Multilevel List), One Page (Blank Page's tint), Navigation Pane (Selection Pane), Switch Modes and Interface Theme (Design's palette). Blue for what changes the view: the lenses of Zoom, Zoom In, Zoom Out and Print Preview (as Find; + and − stay ink), Page Width's and Hide White Space's arrows, Full Screen's corners, Focus' centre dot, Web Layout's globe, the Immersive Reader glasses, tinted pages/windows (Multiple Pages, New Window), Templates' top bar plus a tint, and one window in Arrange All (a tint on all four small tiles barely showed). Gridlines colours only the middle cross. Teal for Text Boundaries' corners (so they differ from Full Screen) and Split's second arrow. Orange for Hide White Space's dashed page gap (as Breaks). Gold for the ruler and Draft's pencil (as Draw Table).

### Side panels, batch 1 · ICON.15 · 2026-11-24

Chanon approved the draft (`~/dev/KrubOffice/ui-experiments/khiankrub-side-panels-icon-colours-draft.html`). Colour is on in the left Navigation panel and in the right panel's Text and Page panes with their tabs (Styles, Text, Paragraph; Page, Design), and on the title row's icon while one of those panes shows (`panes.rs`: `left_panel`, the `"text" | "page"` arm of `right_dock`, `title_row`). These panes reuse the ribbons' drawings, so they take the ribbons' colours. New drawings: Character Border and Enclose Characters (blue frame), Character Spacing (teal arrows), the Styles tab (`layout-sidebar-right`, blue divider as Navigation Pane), Navigation's Headings (`heading`, purple H). Manage Styles' gear stays ink. Not yet: the object panes (Table, Picture, Shape, Chart, Diagram, Header & Footer), the tools (Comments, Accessibility, Style Inspector, Table of Contents, Restrict Editing, Alt Text) and the title row's menu.

### Side panels, batch 2 (object panes) · ICON.16 · 2026-11-24

Chanon approved the draft (`~/dev/KrubOffice/ui-experiments/khiankrub-object-panels-icon-colours-draft.html`). Colour is on in the Table, Picture, Shape, Chart, Diagram and Header & Footer panes and on the title row's icon while one of them shows (`panes.rs`). Drawings already coloured on a ribbon keep their colours. 34 new drawings: green plus signs (Insert Above/Below/Left/Right, Add Shape); red for Delete Rows/Columns/Cells/Table and the Eraser; blue for what moves or changes (Merge, the Split arrows, Rotate, Reset, Change Chart Type, Flip's mirrored half, Crop, the Select outline, Repeat Header Rows' header, Corrections' knobs, Shadow's lines, Text Effects' handles, Convert to Text's arrow); orange for the Split line; teal for Cell Margins' dots; gold for Change Picture's pencil and Picture Styles' star; Color as a red, green and blue filter; Transparency a tinted droplet; Group a tinted front box; Promote orange and Demote green (as Decrease/Increase Indent); Text Pane a purple T. Rotate and Group also show on Layout › Arrange, which now colours them the same way. Align Left/Center/Right and Borders stay ink; KrubOffice's own drawings (diagram layouts, border and cell-alignment buttons, header and footer navigation) stay as they are.

### Side panels, batch 3 (tool panes) · ICON.17 · 2026-11-25

Chanon approved the draft (`~/dev/KrubOffice/ui-experiments/khiankrub-tool-panels-icon-colours-draft.html`). Colour is now on for everything the right panel draws (`panes.rs` `right_dock`): the tools (Comments, Accessibility, Style Inspector, Table of Contents, Alt Text, Restrict Editing), the title row and its menu, as well as the panes of batches 1 and 2 (the per-pane switches are gone). Their icons keep the colours they have elsewhere. New: List of Figures (`list-details`, gold picture boxes) and Remove (`trash`, red lid). The × that closes a tool stays ink.

### The last tabs · ICON.18 · 2026-11-25

Chanon approved the draft (`~/dev/KrubOffice/ui-experiments/khiankrub-more-tabs-icon-colours-draft.html`). `draw_tab` now colours every built-in tab, so Draw, Mailings, Developer, Zotero and Help (hidden until turned on) and Outlining and Equation (contextual) are in colour too; custom groups follow the tab. 29 new drawings: Draw (lasso, pen, Add Pen, ink colour, Ink to Math, replay), Mailings (envelope flap, labels, the fields' braces, preview, previous/next record, check for errors, finish), Developer (design mode, plain text, picture, check box, combo box, drop-down, date picker, repeating section), Zotero (note, document preferences, unlink), Help (help, commands), Outlining (Promote to Heading 1). Expand now uses Tabler's `plus` (green) instead of `minus`, so it no longer looks like Collapse (`tabler-map.tsv` and the generated `tabler.rs` changed the same way). Equation's structure buttons are equation pictures and stay as they are; alignment icons and the close buttons stay ink.

### Right-click menu, mini toolbar and status bar · ICON.19 · 2026-11-25

Chanon approved the draft (`~/dev/KrubOffice/ui-experiments/khiankrub-menus-statusbar-icon-colours-draft.html`) and asked for the status bar in colour too (the draft recommended keeping it monochrome). Colour is on in the text's right-click menu (`canvas.rs`), the mini toolbar (`mini_toolbar.rs`) and the status bar (`chrome.rs` `status_bar`). Their icons keep their ribbon colours, so Zoom Out and Zoom In in the status bar are the orange minus and the green plus. New: Arabic digits (`number-123`, blue), the spelling suggestion's tick (`check`, green), Save (`device-floppy`, blue shutter) and Repeat (`arrow-forward-up`, blue). The top bar and the left side of the File page and start screen stay flat white on the blue band. Dialogs are not coloured.

### Dialogs, account menu and More's tray · ICON.20 · 2026-11-25

Chanon approved the draft (`~/dev/KrubOffice/ui-experiments/khiankrub-dialogs-icon-colours-draft.html`). No new drawings. Colour is on for the page lists of the three dialogs that show icons (Character `dialogs_character.rs`, Modify Style `dialogs_style.rs`, Table Properties `dialogs.rs`), the account menu (`chrome.rs`, the round button's popup) and the + / − on More's tray tiles (`ribbon.rs` `tray_tile`). Options' gear stays ink; Alignment and Borders stay ink. With this, every Tabler icon in KhianKrub is in colour except the flat white ones on the blue band.
