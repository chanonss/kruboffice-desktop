# Source code of the three programs · ซอร์สโค้ดของสามโปรแกรม

Written by `packaging/export-sources.sh` at each KrubOffice release: the desktop source of the programs
bundled in KrubOffice 0.4.0, the files only (no history). The web-only
parts (the web app, the website and its publishing scripts) are not included.

ซอร์สโค้ดฉบับเดสก์ท็อปของสามโปรแกรมใน KrubOffice เขียนโดยอัตโนมัติทุกครั้งที่ออกรุ่นใหม่ ไม่รวมส่วนที่ใช้เฉพาะบนเว็บ

| Folder | Program | Version | Commit | Based on |
|---|---|---|---|---|
| [`khiankrub/`](khiankrub/) | KhianKrub (เขียนครับ) | 0.16.0 | `680b0b90e0f6` | [storytold/wordcraft](https://github.com/storytold/wordcraft) |
| [`khitkrub/`](khitkrub/) | KhitKrub (คิดครับ) | 0.5.0 | `f0812c7fa437` | [storytold/gridcraft](https://github.com/storytold/gridcraft) |
| [`phutkrub/`](phutkrub/) | PhutKrub (พูดครับ) | 0.7.0 | `2e9b99b7b73b` | [storytold/deckcraft](https://github.com/storytold/deckcraft) |

Each folder is a Cargo workspace (MIT OR Apache-2.0, see its `LICENSE-*`, `NOTICE` and `ATTRIBUTION.md`).
Fonts: release builds embed [storytold/craft-fonts](https://github.com/storytold/craft-fonts) at
`aced4137c022` (optional: set `CRAFT_FONTS_DIR` to an absolute path of a checkout). To build one program:

```bash
cd sources/khiankrub && cargo build --release -p wordcraft
```

(`-p gridcraft` in `khitkrub/`, `-p deckcraft` in `phutkrub/`.) To build all of KrubOffice, see the main README.
