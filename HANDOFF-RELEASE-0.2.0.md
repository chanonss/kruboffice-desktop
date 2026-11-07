# Handoff: releasing KrubOffice 0.2.0 (web + Windows + macOS) · 2026-11-07

Everything below is **prepared and committed locally; nothing is pushed or published**. Do the steps in order.
Every step publishes something: the owner (Chanon) runs or approves each one.

## State

| Repo (folder in `~/dev`) | Branch → remote | Release | Local commits ready |
|---|---|---|---|
| `phutkrub-web` | `my-changes` → `web-source/my-changes` | PhutKrub web **0.6.0** | DeckCraft merge, Thai labels, krub-icons sync, release prep (`dfed3d70`) |
| `khitkrub-web` | `my-changes` → `web-source/my-changes` | KhitKrub web **0.4.0** | GridCraft merge, form controls Thai, release prep (`83dc4d31`) |
| `khiankrub-web` | `web` → `web-source/main` | KhianKrub web **0.14.0** | UP16 batches 1–5, release prep (`ef3499f3`), drafts, `desktop.html` Mac steps (`28b82ea4`) |
| `kruboffice-desktop` | `main` → `origin/main` | KrubOffice **0.2.0** (Windows + macOS) | source pins, notes, Mac signing + notarization (`f89c0cc`), this file |

`kruboffice-desktop/packaging/sources.json` pins the **exact local commits** `ef3499f3…` (KhianKrub),
`83dc4d31…` (KhitKrub), `dfed3d70…` (PhutKrub). They must be on the `web-source` repos before the tag is pushed,
or the release workflow can't fetch them.

Owner's uncommitted edit, leave it alone: `khiankrub-web/packaging/web/kruboffice/facebook-th.md`.

## macOS signing (done, tested)

- Developer ID Application: **Chanon Sarasuk (29J2H8D2LA)** in the login keychain; notary profile `krub-notary`
  (App Store Connect API key `HJ56995LM8`).
- GitHub secrets on `chanonss/kruboffice-desktop` (all 7 set): `SOURCES_TOKEN`, `MACOS_CERTIFICATE_P12`,
  `MACOS_CERTIFICATE_PASSWORD`, `MACOS_SIGN_IDENTITY`, `APPLE_API_KEY_P8`, `APPLE_API_KEY_ID`, `APPLE_API_ISSUER_ID`.
- Tested locally: app and dmg signed (hardened runtime), notarized **Accepted**, stapled; Gatekeeper
  `accepted · Notarized Developer ID`. Workflow: `.github/workflows/release.yml` (macOS job on; release needs both
  Windows and macOS to succeed). Scripts: `packaging/macos/{build-app.sh,make-dmg.sh,notarize.sh}`.

## Steps

1. **Push the three app sources** (so the pins resolve):
   ```
   cd ~/dev/phutkrub-web   && git push web-source my-changes
   cd ~/dev/khitkrub-web   && git push web-source my-changes
   cd ~/dev/khiankrub-web  && git push web-source web:main
   ```
2. **Publish the three web apps and their GitHub releases**, as for the previous releases (each repo's `HANDOFF.md`
   describes it): `packaging/web/publish.sh` in each repo, then a GitHub release on the public site repo with the
   Thai notes file:
   - PhutKrub `v0.6.0`: `phutkrub-web/packaging/web/release-notes-v0.6.0.md` → `chanonss/phutkrub-web`
   - KhitKrub `v0.4.0`: `khitkrub-web/packaging/web/release-notes-v0.4.0.md` → `chanonss/khitkrub-web`
   - KhianKrub `v0.14.0`: `khiankrub-web/packaging/web/release-notes-v0.14.0.md` → `chanonss/khiankrub-web`
   Check each live site shows the new version (File › About, and `version.json`).
3. **Publish kruboffice.app**: `khiankrub-web/packaging/web/publish-kruboffice.sh` (README "What's new" draft block —
   change its "ร่าง · ยังไม่เผยแพร่" heading to the release date — and the new `desktop.html` Mac steps).
4. **Release KrubOffice 0.2.0**:
   ```
   cd ~/dev/kruboffice-desktop && git push origin main && git tag v0.2.0 && git push origin v0.2.0
   ```
   Watch the "Release" workflow: Windows builds, installs and uninstalls; macOS builds universal, signs, notarizes
   the app then the dmg (two notary submissions, a few minutes each). On success it publishes the GitHub release with
   `packaging/release-notes/v0.2.0.md` and commits `latest.json` (kruboffice.app's download card reads it).
   If the macOS job fails at notarization, its log prints Apple's reason (`notarytool log`).
5. **Facebook post**: draft in `khiankrub-web/packaging/web/kruboffice/facebook-draft-2026-11-07.md`
   (image `packaging/web/kruboffice/images/khiankrub-missing-font.png`).

## Not done

- README screenshots for PhutKrub's presenter "All slides" grid and KhitKrub's form controls (owner's README rule).
- The RTF clipboard code for Windows was never compiled locally (no Windows target); the release workflow's Windows
  build is its first compile. If it fails there, the code is in `khiankrub-web/apps/wordcraft/src/clipboard.rs`
  (`platform::add_rtf`, Windows module).
- Upstream batches 6–10 paused by the owner (see `khiankrub-web/docs/upstream-v0.16.md`).
