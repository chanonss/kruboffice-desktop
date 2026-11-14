# PhutKrub (พูดครับ) — working notes for Claude

The Thai interface works like KhianKrub's (`crates/ui-egui/src/i18n`): code strings stay English and are the
lookup keys (`tl!("Paste")`), `i18n/th.tsv` maps them to Thai, a missing entry shows in English, and
`i18n::tests::the_most_seen_parts_are_in_thai` fails until every string on the ribbon tabs and File pages has an
entry. Write Thai from the plain meaning of the English (clean room; never copy PowerPoint's Thai strings).


Read [AGENTS.md](AGENTS.md) first: its rules (asset policy, clean room, never crash) bind you and
win over anything here.

New workspace crates must include `[lints] workspace = true` in `Cargo.toml` to inherit
`[workspace.lints]`. Production crate roots must declare the required crate-level lint attributes
before modules or other crate items; crate-level `//!` documentation may precede them.

## Start every session
1. Read `plan/STATUS.md`, then the current milestone in `plan/execution-plan.md`.

## Habits
- Every user-visible action is a command with tests (`crates/engine/src/cmd/*`).
- Look at UI changes: `deckcraft --sample --control 7990` + `ui.screenshot`, or
  `deckcraft-cli render`.
- Commit after each landed arc (`M<n>.<k>: what`) and push to `origin main`.
- Keep `ROADMAP.md`, the progress docs in `docs/` (AGENTS.md §7) and `plan/STATUS.md` current.
- Don't ask the user questions; record genuinely-theirs decisions in `plan/STATUS.md`.

## CI workflows
- Workflows in `.github/workflows/` call the shared workflows in [`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions) at `@main`, with their inputs and secrets. No other jobs: if no shared workflow does what's needed, it doesn't go here.
- Don't add or change CI in a feature or fix PR. If the existing workflows can't build or test a change, link an existing craft-common-actions issue or PR, or open a new issue, or a PR only for a small fix that is safe for every app and does not increase CI time.
- Some workflow files here still run their own jobs. Don't add to them. Moving a job into craft-common-actions is its own task, never part of another PR, and the job stays here until a shared workflow runs it.
