# KrubOffice icon design

Status: 2026-11-01 · ICON.6 approved square ท Thai Distribution icon; PhutKrub paragraph alignment added locally.

Quiet Glass uses original 24×24 drawings, usually inside a 20×20 area, with 1.75-unit outlines and round caps. Display sizes remain controlled by each app. At 16 px the drawings omit fine details and keep at least a 1 px stroke. Foreground ink follows the caller, including its alpha; gold tints disappear on flat band icons and disabled icons. Common action mappings change painting only, never command IDs, tooltips, enabled state or hit targets.

`crates/krub-icons` is an identical vendored library in KhianKrub, KhitKrub and PhutKrub. Each copy uses the host workspace's egui version (0.36 or 0.37), so a checkout builds independently. Changes to this library must be copied to all three repositories and checked with `python3 packaging/check-icon-sync.py <sibling-repo> <sibling-repo>`; no cross-repository path dependencies. The catalog includes 574 distinct names (including aliases), spanning 318 document icon spellings, 131 spreadsheet variants and 246 presentation variants, plus document menu diagrams. Aliases route common actions to identical geometry. Specialized symbols retain their original meaning, adapted from contributor-original Rust artwork to shared proportions, stroke weight and neutral ink with translucent gold surfaces. This consolidates and restyles the existing openly licensed drawings; it does not claim every path was newly invented. Content previews (actual inserted pictures, shape/style galleries, worksheet and document colours) continue to depict the content they represent. In KhitKrub, `icons/content.rs` retains the original stored-artwork painter for chart/cell icon objects; on-sheet UI controls use the shared painter instead.

Logo masters are the app-named SVGs under `assets/app-icon/`. Their gold glass treatment comes from the reviewed richer-gold draft; the existing Thai contours are from Sarabun Bold, SIL OFL 1.1, with its licence beside the artwork. Web loading screens embed the master; install icons are 192/512 px PNG renderings. Start-screen marks use a clipped colour gradient over the app's existing Thai font, so they remain usable without an SVG loader. Native installer icons and the separate KrubOffice launcher are outside this first preview.

## Validation history

ICON.1 initial preview, 2026-11-01: native offscreen application previews captured and inspected; icon tests (finite tessellation at 16/20/24/32 px in normal, flat and disabled states; invalid bounds ignored), shared-crate Clippy with warnings denied, formatting, asset attribution, dependency layering and UI WASM checks passed in all three repositories. Vendored library equality checked. Full workspace CI and complete icon migration remain pending; no commit, push or publication in this preview stage. Existing unused-patch, optional craft-fonts and WASM dead-code warnings remain.

## Balanced toolbar gold · ICON.3

The owner approved Balanced after reviewing the palette comparison. Toolbar gold is `#F2B705` with primary alpha 230/255 and surface alpha 107/255. Caller alpha and drawing opacity are multiplied into those strengths, without a minimum opacity floor. The deck toolkit identifies soft surfaces through its explicit tint role, rather than classifying rendered pixel alpha. Document fills and spreadsheet primary rectangles share the same primary role; worksheet UI washes and surface rectangles use the soft role. Spreadsheet rectangles resolve their surface role once, avoiding double tinting. Flat band icons and disabled controls remain monochrome, including specialized document/spreadsheet routes. The preserved workbook content renderer and document artwork are unchanged.

## Revision history

| Date | Change |
|---|---|
| 2026-11-01 | ICON.3: approved Balanced gold roles; preserve dimmed alpha and monochrome flat/disabled states. |
| 2026-11-01 | ICON.1: shared library and first common action mappings; richer gold glass web logos and start-screen lettering. |

| 2026-11-01 | ICON.2: full shared catalog, common action aliases, document-menu migration, complete inventory/mesh checks and gallery tooling. |

## ICON.2 final verification · 2026-11-01

All 9 shared-catalog tests, UI Clippy, native screenshots, doctests, assets, layers and WASM checks passed. Full workspace CI remains non-green: `gridcraft-calc::tests::loan_schedule_residue_keeps_excels_sign` fails independently with no UI dependency or changed calculation code; `interaction::column_autocomplete_folds_case_beyond_ascii` and `interaction::language_switch_renders_the_ribbon_and_persists` lack the ẞ/ẻ glyphs in the minimal test font set. The remaining integration run reported 908 passing tests alongside those two font failures. These failures were not hidden or assertions weakened. Stored workbook icon objects retain the original renderer.

The three vendored library copies are byte-identical. Commands, shortcuts and document format handlers retain their IDs and behaviour. Changes remain local and uncommitted; no GitHub push, deployment or social publication was performed.

## ICON.3 verification · 2026-11-01

All 11 shared-icon tests pass against egui 0.36.2 and the exact egui 0.37 fork revision used by KhitKrub/PhutKrub. Library Clippy with warnings denied and library WASM checks pass for both. Source formatting and vendored-library equality checks pass. The equality helper ignores Finder's `.DS_Store` metadata while comparing every source/configuration file.

Actual tessellated source icons were rendered without a GPU and visually inspected in light/dark themes, including 32/16 px, dimmed caller opacity, disabled and flat states. The complete 573-name catalog was rendered and inspected at 28/16 px. These are source-icon previews, not complete application ribbon screenshots.

Validation used isolated manifests and a source snapshot verified byte-identical to all three vendored copies. Full workspace CI, whole-app WASM checks and complete app screenshots were not rerun during ICON.3; the broader gate limitations in ICON.2 remain to be resolved before committing/releasing. No commit, push, deployment or social publication was performed.

## Full local web previews · 2026-11-01

Rebuilt all three complete web apps from the updated working trees into separate temporary preview folders. The sample document (KhianKrub), sales workbook and charts (KhitKrub), and Northern Lights presentation (PhutKrub) loaded successfully in the in-app browser with Thai UI labels and the Balanced icons. No runtime errors appeared in the captured browser logs; the existing unsupported web viewport title/theme warnings are non-fatal. Home toolbars were visually inspected in all apps; KhianKrub's Insert ribbon was also checked. This is a browser startup/visual smoke check, not full workspace CI.

Local links: KhianKrub `http://127.0.0.1:8771/?sample`, KhitKrub `http://127.0.0.1:8772/?sample=sales`, PhutKrub `http://127.0.0.1:8773/?sample`. The servers bind only to loopback. Fresh WASM hashes differ from the old pre-redesign `dist/web` bundles. HTML assets and manifest icon paths were checked. No public release or deployment.

## KhianKrub ribbon sizing · ICON.4 · 2026-11-01

Ported the owner's [KhianKrub layout change](https://github.com/chanonss/khiankrub-web-source/commit/b2f991252f53ea07fea94574edd2be41a426a08a): small icons are 20 logical px in 26×24 controls, large icons are 32 px with their center 21 px below the ribbon body top, labels start at 40 px with 13 px line pitch, and the body is 72 px high. Measured columns center within that body; Home two-row formatting blocks use an 8 px gap and three small-control rows fit exactly. A height change requests a discarded layout pass before the new position is displayed. Compact nested rows keep their ordinary placement. All ribbon tabs use the shared button sizing and column helper; no command ids, action routing, enabled states, theme colours or split-menu semantics were changed. This ports the layout rules, without adding group captions to these apps.

The identical `ribbon_layout.rs` modules keep these dimensions and centering rules explicit in both independent checkouts. KhitKrub's title-bar/gallery buttons keep their existing sizing through a separate ribbon-only icon-button wrapper. PhutKrub keeps its separate main-command label and options-arrow hit targets, including the two-line label regression test's updated text coordinates.

Validation: three actual-source layout tests pass per app against the exact pinned egui 0.37 revision (two/three-row bounds and spacing, changed height, nested short row); layout-module Clippy passes with warnings denied. Both complete web apps rebuilt with the locked dependencies. Home and Insert ribbons were inspected at the normal browser viewport with no visible clipping; sample workbook/deck loaded and browser error logs were empty. These are targeted layout checks and complete web smoke checks; existing full widget interaction tests and full workspace CI were not rerun for ICON.4. Existing optional-font, unused-patch and WASM dead-code warnings remain. No commit, push or deployment.

| Date | Change |
|---|---|
| 2026-11-01 | ICON.4: port KhianKrub's enlarged ribbon icons and centered columns; verify complete local web previews. |

## Consistent leading ribbon spacing · ICON.5 · 2026-11-01

The owner’s screenshots exposed different whitespace before Home’s Paste icon. Both apps now anchor ribbon content 8 logical px from the window edge, matching KhianKrub. KhitKrub no longer stacks the panel and inner-card horizontal padding. PhutKrub removes the card’s extra horizontal inset and reserves a left overflow-chevron gutter only after scrolling away from the start; its right gutter remains, and the left cue retains a separate hit target. Large buttons use the same 44 px minimum width as KhianKrub. The shared layout modules remain identical; other sizing and command behaviour are preserved.

Both complete local web builds passed. The six actual-source layout tests passed again. Home was inspected in both browsers; PhutKrub’s right/left overflow arrows were exercised and returning to the start restored the margin without a leading gutter. Captured browser error logs were empty. No full workspace CI or full native widget tests were rerun. No commit, push or deployment.

| Date | Change |
|---|---|
| 2026-11-01 | ICON.5: consistent leading ribbon margin and conditional left overflow gutter. |

## Approved Thai Distribution icon · ICON.6 · 2026-11-01

The owner reviewed `thai-distribution-square-draft.png`, approved it, and requested Thai Distribution in PhutKrub too. The icon uses an 18×18 square on the 24-unit master with a 1.75-unit neutral outline, Balanced soft gold fill, and a centered ท derived from the existing Sarabun SemiBold face. The outline is preflattened Rust geometry with a true counter/hole, so the glyph is accurate and does not depend on installed Thai fonts. Its SIL OFL licence is included in `crates/krub-icons/OFL-Sarabun.txt` and attributed. Flat and disabled states keep the established monochrome/alpha rules. Named document `thaiDistribute` and presentation `Icon::ThaiDistribute` / `thai-distribution` routes use identical painting. No font binaries were added.

PhutKrub gains Home › Paragraph › Thai Distribution immediately after Justify, plus the corresponding paragraph menu/dialog option and Thai tooltip. `format.thaiDistribute` is a command-registry entry with normal enabled state and undo/redo. The model preserves a distinct Thai alignment (`thaiDist` in DrawingML) instead of importing it as ordinary Justify. Wrapped lines share space at Thai grapheme/cluster boundaries and spaces; combining marks remain attached, Latin letters within words retain their spacing, and final paragraph lines remain unstretched. Generic alignment/paragraph command parameters accept `thaiDistribute` too. KhianKrub keeps its existing command/behaviour and receives the new icon only. KhitKrub's vendored library is synchronized without adding a spreadsheet command.

Targeted validation details and final build identifiers are recorded in the local handoff. Full workspace CI remains a separate gate before commits or release.

| Date | Change |
|---|---|
| 2026-11-01 | ICON.6: approved font-independent square ท icon and PhutKrub Thai Distribution. |

### ICON.6 final verification

All 12 shared-icon tests passed on both egui versions; library Clippy with warnings denied passed for both; vendored sources are identical. All three asset-attribution checks passed. PhutKrub's two command/PPTX/undo tests, two Thai/mixed-language layout tests, and the native ribbon-click regression passed. The latter verifies the button follows Justify and applies `Align::ThaiDistribute`. Complete KhianKrub and PhutKrub web builds passed and their new buttons were visually inspected and clicked without browser runtime errors. Formatting and whitespace checks passed. Full workspace CI was not rerun; no commit or publication.

### macOS application icon sizing (2026-11-07)

The runtime icon uses a transparent margin on macOS: the existing artwork occupies 412 of 512 pixels on each side, matching KhianKrub's 824/1024 icon grid. The original artwork remains the source for the other platforms.
