# KrubOffice สำหรับเดสก์ท็อป · KrubOffice for the desktop

**รุ่นทดลอง / Beta.** [ดาวน์โหลดและวิธีติดตั้ง / Download and install](https://kruboffice.app/desktop.html)

KrubOffice เป็นแอปเดียวสำหรับ macOS และ Windows ที่รวมโปรแกรมสามตัวไว้ข้างใน เปิดไฟล์ใดก็ได้ แล้ว KrubOffice จะเลือกโปรแกรมให้เอง

KrubOffice is one app for macOS and Windows with three programs inside. Open any file and KrubOffice picks the program.

| โปรแกรม / Program | ใช้กับ / For | ไฟล์ / Files |
|---|---|---|
| เขียนครับ · KhianKrub | เอกสาร / documents | `.docx` `.doc` `.dotx` `.rtf` `.odt` `.txt` `.md` `.html` … |
| คิดครับ · KhitKrub | ตารางคำนวณ / spreadsheets | `.xlsx` `.xls` `.xlsb` `.ods` `.csv` `.tsv` … |
| พูดครับ · PhutKrub | งานนำเสนอ / presentations | `.pptx` `.potx` `.ppsx` |

เปิด KrubOffice โดยไม่เลือกไฟล์ จะเห็นหน้าเริ่มต้น "วันนี้ ...อยากทำอะไรดีครับ?" แบบเดียวกับ [kruboffice.app](https://kruboffice.app/) มีสามโปรแกรมให้เลือก ปุ่มเปิดไฟล์ และไฟล์ล่าสุดของทั้งสามโปรแกรม

Opened without a file, KrubOffice shows the same welcome as [kruboffice.app](https://kruboffice.app/): the three programs, Open File, and the recent files of all three.

<p align="center">
  <img src="docs/images/start-light.png" alt="The KrubOffice Start Center in Thai: the KrubOffice logo, the welcome วันนี้ ...อยากทำอะไรดีครับ?, the three app icons เขียนครับ, คิดครับ and พูดครับ, the เปิดไฟล์… button and three recent files" width="49%">
  <img src="docs/images/start-dark.png" alt="The same Start Center in the dark theme" width="49%">
</p>

ติดตามข่าวและรุ่นใหม่ได้ที่ **[เพจ Facebook ของ KrubOffice](https://www.facebook.com/kruboffice/)** · Follow **[KrubOffice on Facebook](https://www.facebook.com/kruboffice/)** for news and new versions.

## Source code · ซอร์สโค้ด

KrubOffice is open source (MIT or Apache-2.0). This repository holds the launcher and the packaging, and in
[`sources/`](sources/) the source of the three programs exactly as they are built into each release: KhianKrub,
KhitKrub and PhutKrub, each a fork of an ArtCraft app ([WordCraft](https://github.com/storytold/wordcraft),
[GridCraft](https://github.com/storytold/gridcraft), [DeckCraft](https://github.com/storytold/deckcraft)).
`sources/` is updated automatically at every release; [`sources/README.md`](sources/README.md) says which version
and commit each comes from. The code of the web versions (khiankrub.app and the others) is not included.

KrubOffice เป็นโอเพนซอร์ส (MIT หรือ Apache-2.0) ที่เก็บนี้มีโค้ดของตัวเปิดโปรแกรมและการสร้างตัวติดตั้ง และในโฟลเดอร์
[`sources/`](sources/) มีซอร์สโค้ดของเขียนครับ คิดครับ และพูดครับ ตรงกับที่อยู่ใน KrubOffice แต่ละรุ่น
ซึ่งพัฒนาต่อยอดจาก WordCraft, GridCraft และ DeckCraft ของทีม ArtCraft โฟลเดอร์นี้อัปเดตอัตโนมัติทุกครั้งที่ออกรุ่นใหม่
ส่วนโค้ดของเวอร์ชันบนเว็บไม่ได้รวมไว้ที่นี่

## How it is put together

- **This repository** holds the launcher (`src/`: Start Center, file-type dispatch), the packaging
  (`packaging/macos`, `packaging/windows`), the release workflow and the Releases page.
- **The programs come from the web apps' repositories** (`khiankrub-web`, `khitkrub-web`,
  `phutkrub-web`): one code line for the web and the desktop. Work lands on the web first; a
  desktop release bundles the latest web releases.
- **The file types live in one table**, `App::extensions` in `src/dispatch.rs`. The macOS
  `Info.plist` and the Windows installer are generated from it (`kruboffice --list-types`).

| | macOS | Windows |
|---|---|---|
| Layout | `KrubOffice.app` with `Contents/Helpers/{KhianKrub,KhitKrub,PhutKrub}.app` | `Program Files\KrubOffice\` with `kruboffice.exe` and the three programs' `.exe` |
| Opening a file | Finder sends it to KrubOffice, which passes it on through LaunchServices (a running program gets it in its window) | Explorer starts `kruboffice.exe "file"`, which starts the program with the file |
| File types | `CFBundleDocumentTypes`, rank Alternate | `OpenWithProgids` and Default apps capabilities; never takes over a default |

## `latest.json`: what kruboffice.app shows

kruboffice.app (the download card and `desktop.html`) reads `latest.json` from this repository's `main` branch
(`https://raw.githubusercontent.com/chanonss/kruboffice-desktop/main/latest.json`). The release workflow writes it
after each release's files are uploaded; until it exists, the site shows no download. Download URLs must start
with `https://github.com/chanonss/kruboffice-desktop/releases/download/` or the site ignores them.

```json
{"version": "0.1.0", "date": "2026-11-05",
 "notes": "https://github.com/chanonss/kruboffice-desktop/releases/tag/v0.1.0",
 "includes": {"KhianKrub": "0.16.0", "KhitKrub": "0.5.0", "PhutKrub": "0.7.0"},
 "files": {
  "macos": {"name": "KrubOffice-0.1.0-macos-universal.dmg", "url": "https://github.com/chanonss/kruboffice-desktop/releases/download/v0.1.0/KrubOffice-0.1.0-macos-universal.dmg", "size": 101711872},
  "windows": {"name": "KrubOffice-0.1.0-windows-x64.msi", "url": "https://github.com/chanonss/kruboffice-desktop/releases/download/v0.1.0/KrubOffice-0.1.0-windows-x64.msi", "size": 67108864}}}
```

## Releasing

1. Each program's web release comes first; pin its source commit and version in `packaging/sources.json`.
2. Write the Thai release notes in `packaging/release-notes/vX.Y.Z.md` (drafts go to the owner first) and set
   `version` in `Cargo.toml`.
3. Push the tag `vX.Y.Z`. `.github/workflows/release.yml` builds the Mac app (Apple silicon and Intel) and the
   Windows installer, checks both (the installer is installed and removed on the runner), publishes the release
   with `SHA256SUMS`, and commits `latest.json`. It reads the private sources with the `SOURCES_TOKEN` secret, then
   commits their desktop-only files to `sources/` (`packaging/export-sources.sh`, which also checks that nothing
   web-only or secret is left).

## Building (macOS)

```bash
packaging/macos/build-app.sh
```

Builds the launcher and the three programs (sibling folders by default; `KHIANKRUB_SRC`,
`KHITKRUB_SRC`, `PHUTKRUB_SRC` override) and writes `dist/KrubOffice.app`, ad-hoc signed.

## Building (Debian, Ubuntu)

```bash
packaging/linux/build-deb.sh
```

Builds the launcher and the three programs (same sibling-folder defaults and overrides as macOS) and writes
`dist/kruboffice_<version>_<arch>.deb`. Install with `sudo apt install ./dist/kruboffice_*.deb`. Everything goes in
`/usr/lib/kruboffice` (plain Rust executables, no Python), with `/usr/bin/kruboffice` and a menu entry that
registers the file types. The release workflow builds it for amd64 on Ubuntu 22.04.

## Licence

MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). KhianKrub, KhitKrub and PhutKrub are adaptations, by Chanon
Sarasuk, of WordCraft, GridCraft and DeckCraft by the ArtCraft team. Every build ships `THIRD-PARTY-NOTICES.txt`
(made by `packaging/notices.py`) with the licences of the libraries and fonts it contains. Not products of ArtCraft
and not affiliated with Microsoft.
