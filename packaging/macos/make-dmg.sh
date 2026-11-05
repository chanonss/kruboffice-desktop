#!/usr/bin/env bash
# Put KrubOffice.app on a drag-to-Applications disk image.
#   packaging/macos/make-dmg.sh APP OUT.dmg
set -euo pipefail
APP="$1"
OUT="$2"
test -d "$APP" || { echo "error: $APP is missing" >&2; exit 1; }
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
rm -f "$OUT"
hdiutil create -volname "KrubOffice" -srcfolder "$STAGE" -fs HFS+ -format UDZO -imagekey zlib-level=9 "$OUT" >/dev/null
hdiutil verify "$OUT" >/dev/null
echo "Built $OUT"
