#!/usr/bin/env bash
# Build KrubOffice for Debian and Ubuntu (amd64 or arm64, whichever this machine is): the launcher and the three
# programs, and dist/kruboffice_<version>_<arch>.deb.
#
#   packaging/linux/build-deb.sh
#
# Needs Rust, Python 3, dpkg-dev (dpkg-deb, dpkg-shlibdeps), curl, and the programs' build libraries:
#   sudo apt install build-essential pkg-config python3 dpkg-dev curl libxkbcommon-dev libwayland-dev libx11-dev libasound2-dev
# Source folders default to siblings of this repo (as for macOS); KHIANKRUB_SRC, KHITKRUB_SRC and PHUTKRUB_SRC override.
#
# Layout: everything lives in /usr/lib/kruboffice (the launcher looks for the three programs, fonts/ and
# icons/ next to its own executable, as on Windows); /usr/bin/kruboffice is a symlink to the launcher.
# No Python and no virtual environment at run time: all four programs are plain Rust executables.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SUITE_ROOT="$(cd "$ROOT/.." && pwd)"
KHIANKRUB_SRC="${KHIANKRUB_SRC:-$SUITE_ROOT/khiankrub}"
KHITKRUB_SRC="${KHITKRUB_SRC:-$SUITE_ROOT/khitkrub}"
PHUTKRUB_SRC="${PHUTKRUB_SRC:-$SUITE_ROOT/phutkrub}"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
[ -n "$VERSION" ] || { echo "version not found in Cargo.toml" >&2; exit 1; }
ARCH="$(dpkg --print-architecture)"
case "$ARCH" in amd64) TRIPLE=x86_64-unknown-linux-gnu ;; arm64) TRIPLE=aarch64-unknown-linux-gnu ;; *) echo "unsupported architecture: $ARCH" >&2; exit 1 ;; esac

# Each program's About shows its source commit (instead of "development build").
WORDCRAFT_BUILD_SHA="$(git -C "$KHIANKRUB_SRC" rev-parse HEAD 2>/dev/null || true)"; export WORDCRAFT_BUILD_SHA
GRIDCRAFT_BUILD_SHA="$(git -C "$KHITKRUB_SRC" rev-parse HEAD 2>/dev/null || true)"; export GRIDCRAFT_BUILD_SHA
DECKCRAFT_BUILD_SHA="$(git -C "$PHUTKRUB_SRC" rev-parse HEAD 2>/dev/null || true)"; export DECKCRAFT_BUILD_SHA

build() { (cd "$1" && cargo build --release -p "$2" --target "$TRIPLE" >&2); }
build "$ROOT" kruboffice
build "$KHIANKRUB_SRC" wordcraft
build "$KHITKRUB_SRC" gridcraft
build "$PHUTKRUB_SRC" deckcraft

DIST="$ROOT/dist"
PKG="$DIST/deb/kruboffice_${VERSION}_${ARCH}"
LIB="$PKG/usr/lib/kruboffice"
rm -rf "$DIST/deb"
mkdir -p "$PKG/DEBIAN" "$LIB/fonts" "$LIB/icons" "$PKG/usr/bin" "$PKG/usr/share/applications" \
         "$PKG/usr/share/icons/hicolor/512x512/apps" "$PKG/usr/share/doc/kruboffice"

install -m 0755 "$ROOT/target/$TRIPLE/release/kruboffice" "$LIB/kruboffice"
install -m 0755 "$KHIANKRUB_SRC/target/$TRIPLE/release/wordcraft" "$LIB/khiankrub"
install -m 0755 "$KHITKRUB_SRC/target/$TRIPLE/release/gridcraft" "$LIB/khitkrub"
install -m 0755 "$PHUTKRUB_SRC/target/$TRIPLE/release/deckcraft" "$LIB/phutkrub"
ln -s ../lib/kruboffice/kruboffice "$PKG/usr/bin/kruboffice"
strip --strip-unneeded "$LIB"/kruboffice "$LIB"/khiankrub "$LIB"/khitkrub "$LIB"/phutkrub

# The Start Center's font (Sarabun, SIL OFL 1.1) and the apps' icons, next to the launcher.
for f in Sarabun-Regular.ttf Sarabun-SemiBold.ttf OFL.txt; do
    if [ -f "$KHIANKRUB_SRC/dist/web/fonts/$f" ]; then cp "$KHIANKRUB_SRC/dist/web/fonts/$f" "$LIB/fonts/$f"
    else curl -fsSL "https://raw.githubusercontent.com/google/fonts/main/ofl/sarabun/$f" -o "$LIB/fonts/$f"; fi
done
cp "$KHIANKRUB_SRC/assets/app-icon/khiankrub-192.png" "$KHITKRUB_SRC/assets/app-icon/khitkrub-192.png" \
   "$PHUTKRUB_SRC/assets/app-icon/phutkrub-192.png" "$LIB/icons/"
cp "$ROOT/packaging/windows/kruboffice-512.png" "$PKG/usr/share/icons/hicolor/512x512/apps/kruboffice.png"
mkdir -p "$PKG/usr/share/icons/hicolor/192x192/apps"
for app in khiankrub khitkrub phutkrub; do
    cp "$LIB/icons/$app-192.png" "$PKG/usr/share/icons/hicolor/192x192/apps/kruboffice-$app.png"
done

python3 -I "$ROOT/packaging/linux/make_desktop.py" "$ROOT/packaging/types.txt" "$PKG/usr/share/applications/kruboffice.desktop"

# Licences: KrubOffice's own, and the notices of the open-source parts in this build.
cp "$ROOT/LICENSE-MIT" "$ROOT/LICENSE-APACHE" "$PKG/usr/share/doc/kruboffice/"
notice_args=("$PKG/usr/share/doc/kruboffice/THIRD-PARTY-NOTICES.txt" "$TRIPLE" "$ROOT:kruboffice" "$KHIANKRUB_SRC:wordcraft" "$KHITKRUB_SRC:gridcraft" "$PHUTKRUB_SRC:deckcraft" --fonts "$LIB/fonts")
if [ -d "$KHIANKRUB_SRC/../craft-fonts/fonts" ]; then notice_args+=(--fonts "$KHIANKRUB_SRC/../craft-fonts/fonts"); fi
python3 -I "$ROOT/packaging/notices.py" "${notice_args[@]}"

# Run-time libraries the executables link to (dpkg-shlibdeps wants a debian/control in the current folder).
SHLIBS="$(mktemp -d)"; trap 'rm -rf "$SHLIBS"' EXIT
mkdir "$SHLIBS/debian"
printf 'Source: kruboffice\n\nPackage: kruboffice\nArchitecture: any\n' > "$SHLIBS/debian/control"
DEPENDS="$(cd "$SHLIBS" && dpkg-shlibdeps -O -e"$LIB/kruboffice" -e"$LIB/khiankrub" -e"$LIB/khitkrub" -e"$LIB/phutkrub" 2>/dev/null | sed -n 's/^shlibs:Depends=//p')"

SIZE="$(du -sk --exclude=DEBIAN "$PKG" | cut -f1)"
cat > "$PKG/DEBIAN/control" <<CONTROL
Package: kruboffice
Version: $VERSION
Section: editors
Priority: optional
Architecture: $ARCH
Installed-Size: $SIZE
Maintainer: KrubOffice <maintainer@kruboffice.invalid>
Depends: ${DEPENDS:-libc6}
Recommends: mesa-vulkan-drivers, libvulkan1, libxkbcommon-x11-0, fonts-noto-core, xdg-desktop-portal | zenity
Homepage: https://kruboffice.app/
Description: KrubOffice: documents, spreadsheets and presentations in one app
 KrubOffice bundles KhianKrub (documents), KhitKrub (spreadsheets) and
 PhutKrub (presentations), in Thai and English. Open any file and KrubOffice
 picks the program. Free and open source.
CONTROL

# Files owned by root, the usual modes.
find "$PKG" -type d -exec chmod 0755 {} +
dpkg-deb --root-owner-group --build "$PKG" "$DIST/kruboffice_${VERSION}_${ARCH}.deb"
echo "Built $DIST/kruboffice_${VERSION}_${ARCH}.deb"
