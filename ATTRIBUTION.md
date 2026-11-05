# Attribution

This repository holds no image, font or icon files. The app icons in a built KrubOffice are made
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
