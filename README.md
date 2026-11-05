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
