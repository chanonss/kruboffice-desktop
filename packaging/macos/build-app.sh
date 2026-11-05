#!/usr/bin/env bash
# Build KrubOffice.app for macOS: the launcher with KhianKrub, KhitKrub and PhutKrub inside.
#
#   KrubOffice.app/Contents/MacOS/kruboffice            the launcher (Start Center, file dispatch)
#   KrubOffice.app/Contents/Helpers/KhianKrub.app       documents      (from ../khiankrub-web)
#   KrubOffice.app/Contents/Helpers/KhitKrub.app        spreadsheets   (from ../khitkrub-web)
#   KrubOffice.app/Contents/Helpers/PhutKrub.app        presentations  (from ../phutkrub-web)
#
# KrubOffice.app declares every file type the three programs open, so Finder sends those files to
# KrubOffice, which passes each to its program. The helpers declare no file types of their own,
# so Finder doesn't offer them separately.
#
# Usage: packaging/macos/build-app.sh [--skip-build] [--universal] [--out DIR]
#   --universal                                 one app for Apple silicon and Intel (needs both Rust targets)
#   KHIANKRUB_SRC, KHITKRUB_SRC, PHUTKRUB_SRC   source folders (default: siblings of this repo)
#   MACOS_SIGN_IDENTITY                         codesign identity; default "-" (ad-hoc)
# Without --universal it builds for this Mac's architecture only.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEV="$(cd "$ROOT/.." && pwd)"
KHIANKRUB_SRC="${KHIANKRUB_SRC:-$DEV/khiankrub-web}"
KHITKRUB_SRC="${KHITKRUB_SRC:-$DEV/khitkrub-web}"
PHUTKRUB_SRC="${PHUTKRUB_SRC:-$DEV/phutkrub-web}"
OUT="$ROOT/dist"
BUILD=1
UNIVERSAL=0
while [ $# -gt 0 ]; do
    case "$1" in
        --skip-build) BUILD=0 ;;
        --universal) UNIVERSAL=1 ;;
        --out) shift; OUT="$1" ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
    shift
done
SIGN="${MACOS_SIGN_IDENTITY:--}"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"

# The three programs' own version numbers (the web releases they come from).
khian_version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$KHIANKRUB_SRC/Cargo.toml" | head -1)"
khit_version="$(sed -n 's/^pub const APP_VERSION: &str = "\(.*\)";$/\1/p' "$KHITKRUB_SRC/crates/ui-egui/src/lib.rs")"
phut_version="$(sed -n 's/^pub const APP_VERSION: &str = "\(.*\)";$/\1/p' "$PHUTKRUB_SRC/crates/ui-egui/src/lib.rs")"
for v in "$VERSION" "$khian_version" "$khit_version" "$phut_version"; do
    [ -n "$v" ] || { echo "error: a version number is missing" >&2; exit 1; }
done

# Build one program; prints the path of its binary (a universal one with --universal).
#   build_bin SRC_DIR PACKAGE BINARY [TARGET_DIR]
build_bin() {
    local src="$1" pkg="$2" bin="$3" tdir="${4:-$1/target}"
    if [ "$UNIVERSAL" = 1 ]; then
        if [ "$BUILD" = 1 ]; then
            (cd "$src" && CARGO_TARGET_DIR="$tdir" cargo build --release -p "$pkg" --target aarch64-apple-darwin --target x86_64-apple-darwin >&2)
        fi
        mkdir -p "$tdir/universal"
        lipo -create "$tdir/aarch64-apple-darwin/release/$bin" "$tdir/x86_64-apple-darwin/release/$bin" -output "$tdir/universal/$bin"
        echo "$tdir/universal/$bin"
    else
        if [ "$BUILD" = 1 ]; then
            (cd "$src" && CARGO_TARGET_DIR="$tdir" cargo build --release -p "$pkg" >&2)
        fi
        echo "$tdir/release/$bin"
    fi
}
LAUNCHER="$(build_bin "$ROOT" kruboffice kruboffice "${CARGO_TARGET_DIR:-$ROOT/target}")"
KHIAN_BIN="$(build_bin "$KHIANKRUB_SRC" wordcraft wordcraft)"
KHIT_BIN="$(build_bin "$KHITKRUB_SRC" gridcraft gridcraft)"
PHUT_BIN="$(build_bin "$PHUTKRUB_SRC" deckcraft deckcraft)"
for f in "$LAUNCHER" "$KHIAN_BIN" "$KHIT_BIN" "$PHUT_BIN"; do
    test -x "$f" || { echo "error: $f is missing; build first" >&2; exit 1; }
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# An .icns from one of our own square PNG icons (512 px), drawn for KrubOffice; no upstream art.
make_icns() {
    local png="$1" out="$2"
    local set
    set="$WORK/$(basename "$out" .icns).iconset"
    mkdir -p "$set"
    for s in 16 32 128 256 512; do
        sips -z "$s" "$s" "$png" --out "$set/icon_${s}x${s}.png" >/dev/null
        local d=$((s * 2))
        if [ "$d" -le 512 ]; then sips -z "$d" "$d" "$png" --out "$set/icon_${s}x${s}@2x.png" >/dev/null; fi
    done
    iconutil -c icns "$set" -o "$out"
}

# Info.plist for a helper program (no document types: KrubOffice receives files for it).
helper_plist() {
    local name="$1" exe="$2" id="$3" version="$4"
    cat <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$name</string>
  <key>CFBundleDisplayName</key><string>$name</string>
  <key>CFBundleExecutable</key><string>$exe</string>
  <key>CFBundleIdentifier</key><string>$id</string>
  <key>CFBundleIconFile</key><string>$name</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>CFBundleDevelopmentRegion</key><string>th</string>
  <key>CFBundleLocalizations</key><array><string>th</string><string>en</string></array>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
</dict>
</plist>
EOF
}

APP="$OUT/KrubOffice.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$APP/Contents/Helpers"
cp "$LAUNCHER" "$APP/Contents/MacOS/kruboffice"
make_icns "$KHIANKRUB_SRC/packaging/web/public/icons/krubOffice-maskable-512.png" "$APP/Contents/Resources/KrubOffice.icns"

# The Start Center's app icons (the apps' own glass artwork) and its interface font, Sarabun
# (SIL OFL 1.1), the same files the web apps use: from KhianKrub's web build when it has them,
# else from their source, Google Fonts' OFL repository.
mkdir -p "$APP/Contents/Resources/icons" "$APP/Contents/Resources/fonts"
cp "$KHIANKRUB_SRC/assets/app-icon/khiankrub-192.png" "$KHITKRUB_SRC/assets/app-icon/khitkrub-192.png" \
   "$PHUTKRUB_SRC/assets/app-icon/phutkrub-192.png" "$APP/Contents/Resources/icons/"
for f in Sarabun-Regular.ttf Sarabun-SemiBold.ttf OFL.txt; do
    if [ -f "$KHIANKRUB_SRC/dist/web/fonts/$f" ]; then
        cp "$KHIANKRUB_SRC/dist/web/fonts/$f" "$APP/Contents/Resources/fonts/$f"
    else
        curl -fsSL "https://raw.githubusercontent.com/google/fonts/main/ofl/sarabun/$f" -o "$APP/Contents/Resources/fonts/$f"
    fi
done

add_helper() {
    local name="$1" bin="$2" exe="$3" icon="$4" version="$5"
    local h="$APP/Contents/Helpers/$name.app"
    mkdir -p "$h/Contents/MacOS" "$h/Contents/Resources"
    cp "$bin" "$h/Contents/MacOS/$exe"
    make_icns "$icon" "$h/Contents/Resources/$name.icns"
    helper_plist "$name" "$exe" "app.kruboffice.$(echo "$name" | tr '[:upper:]' '[:lower:]')" "$version" > "$h/Contents/Info.plist"
}
add_helper KhianKrub "$KHIAN_BIN" khiankrub "$KHIANKRUB_SRC/assets/app-icon/khiankrub-512.png" "$khian_version"
add_helper KhitKrub "$KHIT_BIN" khitkrub "$KHITKRUB_SRC/assets/app-icon/khitkrub-512.png" "$khit_version"
add_helper PhutKrub "$PHUT_BIN" phutkrub "$PHUTKRUB_SRC/assets/app-icon/phutkrub-512.png" "$phut_version"

# KrubOffice's Info.plist: one document type per program, with the extensions from the
# launcher's own table (packaging/types.txt, which a unit test keeps equal to it).
python3 -I "$ROOT/packaging/macos/info_plist.py" "$VERSION" "$khian_version" "$khit_version" "$phut_version" \
    < "$ROOT/packaging/types.txt" > "$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist" >/dev/null

# KrubOffice's licence and the notices of the open-source parts in this build.
cp "$ROOT/LICENSE.txt" "$APP/Contents/Resources/LICENSE.txt"
notice_args=("$APP/Contents/Resources/THIRD-PARTY-NOTICES.txt" aarch64-apple-darwin "$ROOT:kruboffice" "$KHIANKRUB_SRC:wordcraft" "$KHITKRUB_SRC:gridcraft" "$PHUTKRUB_SRC:deckcraft" --fonts "$APP/Contents/Resources/fonts")
if [ -d "$KHIANKRUB_SRC/../craft-fonts/fonts" ]; then notice_args+=(--fonts "$KHIANKRUB_SRC/../craft-fonts/fonts"); fi
python3 -I "$ROOT/packaging/notices.py" "${notice_args[@]}"

# Smaller download: drop debugging symbols (the bundle was 233 MB unstripped).
strip -x "$APP/Contents/MacOS/kruboffice" "$APP"/Contents/Helpers/*.app/Contents/MacOS/*

# Sign inside out: the helpers, then the whole app.
for h in "$APP"/Contents/Helpers/*.app; do
    codesign --force --sign "$SIGN" --timestamp=none "$h"
done
codesign --force --sign "$SIGN" --timestamp=none "$APP"
codesign --verify --deep --strict "$APP"
echo "Built $APP (KrubOffice $VERSION: KhianKrub $khian_version, KhitKrub $khit_version, PhutKrub $phut_version)"
