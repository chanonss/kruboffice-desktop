# KrubOffice สำหรับเดสก์ท็อป · KrubOffice for the desktop

**สถานะ: กำลังพัฒนา ยังไม่เผยแพร่ / Status: in development, not published.**

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

## How it is put together

- **This repository** holds the launcher (`src/`: Start Center, file-type dispatch), the packaging
  (`packaging/macos`, `packaging/windows`) and, later, the release workflow and the Releases page.
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
 "includes": {"KhianKrub": "0.12.0", "KhitKrub": "0.3.0", "PhutKrub": "0.5.0"},
 "files": {
  "macos": {"name": "KrubOffice-0.1.0-macos-universal.dmg", "url": "https://github.com/chanonss/kruboffice-desktop/releases/download/v0.1.0/KrubOffice-0.1.0-macos-universal.dmg", "size": 101711872},
  "windows": {"name": "KrubOffice-0.1.0-windows-x64.msi", "url": "https://github.com/chanonss/kruboffice-desktop/releases/download/v0.1.0/KrubOffice-0.1.0-windows-x64.msi", "size": 67108864}}}
```

## Building (macOS)

```bash
packaging/macos/build-app.sh
```

Builds the launcher and the three programs (sibling folders by default; `KHIANKRUB_SRC`,
`KHITKRUB_SRC`, `PHUTKRUB_SRC` override) and writes `dist/KrubOffice.app`, ad-hoc signed.

## Licence

MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). KhianKrub, KhitKrub and PhutKrub are
adaptations, by Chanon Sarasuk, of WordCraft, GridCraft and DeckCraft by the ArtCraft team. Not
products of ArtCraft and not affiliated with Microsoft.
