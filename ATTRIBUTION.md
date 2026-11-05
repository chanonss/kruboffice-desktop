# Attribution

This repository's only image files are screenshots of KrubOffice itself (below). It holds no font or icon files. The app icons in a built KrubOffice are made
at build time (`packaging/macos/build-app.sh`) from the apps' own icons, each recorded in its
repository's `ATTRIBUTION.md`:

| Used for | Source file | Repository | Licence |
|---|---|---|---|
| KrubOffice | `packaging/web/public/icons/krubOffice-maskable-512.png` | khiankrub-web | MIT OR Apache-2.0 |
| KhianKrub | `assets/app-icon/khiankrub-512.png` | khiankrub-web | MIT OR Apache-2.0 (Thai outline: Sarabun, SIL OFL 1.1) |
| KhitKrub | `assets/app-icon/khitkrub-512.png` | khitkrub-web | MIT OR Apache-2.0 (Thai outline: Sarabun, SIL OFL 1.1) |
| PhutKrub | `assets/app-icon/phutkrub-512.png` | phutkrub-web | MIT OR Apache-2.0 (Thai outline: Sarabun, SIL OFL 1.1) |

The Start Center draws its logo and app tiles in code (`src/start.rs`) and uses the platform's
Thai font (or Sarabun when the packaging places it in the app's `fonts` folder).

Built apps also carry, in their resources:

| File | Author | Source | Licence |
|---|---|---|---|
| `fonts/Sarabun-Regular.ttf`, `fonts/Sarabun-SemiBold.ttf`, `fonts/OFL.txt` | Cadson Demak | google/fonts `ofl/sarabun` (as used by the web apps) | SIL OFL 1.1 |
| `icons/khiankrub-192.png`, `khitkrub-192.png`, `phutkrub-192.png` | Chanon Sarasuk | the apps' repositories (`assets/app-icon`) | MIT OR Apache-2.0 (Thai outline: Sarabun, SIL OFL 1.1) |

Screenshots in this repository:

| File | Author | Source | Licence |
|---|---|---|---|
| `docs/images/start-light.png`, `docs/images/start-dark.png` | Chanon Sarasuk | rendered offscreen by `examples/shot.rs` (our own interface, icons and Sarabun) | MIT OR Apache-2.0 (Sarabun: SIL OFL 1.1) |
