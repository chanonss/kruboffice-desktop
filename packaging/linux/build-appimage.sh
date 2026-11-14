#!/usr/bin/env bash
# Build KrubOffice as an AppImage (x86_64): one file that runs on most Linux distributions without installing.
#
#   packaging/linux/build-deb.sh && packaging/linux/build-appimage.sh
#
# Run after build-deb.sh: it packs the same files the .deb installs (dist/deb/kruboffice_<version>_amd64/usr), so
# both packages hold identical programs. Writes dist/KrubOffice-<version>-x86_64.AppImage.
#
# Inside the AppImage the launcher runs from usr/lib/kruboffice and finds the three programs, fonts/ and icons/
# next to itself, as in the .deb. Graphics drivers (Vulkan/OpenGL) and the C library come from the system, so the
# AppImage runs on distributions at least as new as the build machine (Ubuntu 22.04, glibc 2.35).
#
# appimagetool and the AppImage runtime come from the AppImage project's fixed releases and are checked against the
# SHA-256 sums below; change the version and the sum together.
set -euo pipefail

APPIMAGETOOL_URL="https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage"
APPIMAGETOOL_SHA256="ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0"
RUNTIME_URL="https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64"
RUNTIME_SHA256="2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
[ -n "$VERSION" ] || { echo "version not found in Cargo.toml" >&2; exit 1; }
[ "$(uname -m)" = x86_64 ] || { echo "the AppImage is built on x86_64 only" >&2; exit 1; }
DIST="$ROOT/dist"
STAGE="$DIST/deb/kruboffice_${VERSION}_amd64"
[ -x "$STAGE/usr/lib/kruboffice/kruboffice" ] || { echo "run packaging/linux/build-deb.sh first ($STAGE missing)" >&2; exit 1; }

TOOLS="$DIST/appimage-tools"
mkdir -p "$TOOLS"
fetch() { # url sha256 file
    if [ ! -f "$3" ] || ! echo "$2  $3" | sha256sum -c --quiet - 2>/dev/null; then
        curl -fsSL "$1" -o "$3.part"
        echo "$2  $3.part" | sha256sum -c --quiet - || { echo "checksum mismatch for $1" >&2; rm -f "$3.part"; exit 1; }
        mv "$3.part" "$3"
    fi
}
fetch "$APPIMAGETOOL_URL" "$APPIMAGETOOL_SHA256" "$TOOLS/appimagetool"
fetch "$RUNTIME_URL" "$RUNTIME_SHA256" "$TOOLS/runtime-x86_64"
chmod +x "$TOOLS/appimagetool"

APPDIR="$DIST/KrubOffice.AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR"
cp -a "$STAGE/usr" "$APPDIR/usr"
rm -f "$APPDIR/usr/bin/kruboffice"            # the .deb's symlink to /usr/lib; AppRun starts the launcher instead
rmdir "$APPDIR/usr/bin" 2>/dev/null || true
# The three programs' hidden menu entries name /usr/lib/kruboffice, which does not exist outside the .deb.
rm -f "$APPDIR"/usr/share/applications/kruboffice-*.desktop

cat > "$APPDIR/AppRun" <<'APPRUN'
#!/bin/sh
# Start the KrubOffice launcher from inside the AppImage; it finds the three programs next to itself.
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/lib/kruboffice/kruboffice" "$@"
APPRUN
chmod 0755 "$APPDIR/AppRun"
cp "$APPDIR/usr/share/applications/kruboffice.desktop" "$APPDIR/kruboffice.desktop"
echo "X-AppImage-Version=$VERSION" >> "$APPDIR/kruboffice.desktop"
cp "$APPDIR/usr/share/icons/hicolor/512x512/apps/kruboffice.png" "$APPDIR/kruboffice.png"
ln -s kruboffice.png "$APPDIR/.DirIcon"

OUT="$DIST/KrubOffice-${VERSION}-x86_64.AppImage"
rm -f "$OUT"
# appimagetool is itself an AppImage: extract-and-run works where FUSE is missing (CI runners, containers).
APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 "$TOOLS/appimagetool" --no-appstream --runtime-file "$TOOLS/runtime-x86_64" "$APPDIR" "$OUT"
chmod 0755 "$OUT"
echo "Built $OUT"
