#!/usr/bin/env bash
# The macOS app icon (.icns) from KhianKrub's glass ข (assets/app-icon/khiankrub-512.png), made the
# way the KrubOffice launcher makes its helpers' icons, so every KhianKrub shows the same icon in
# the Dock (owner, 2026-11-16: the standalone app still used the old flat ข).
#
#   packaging/macos/khiankrub-icns.sh OUT.icns
set -euo pipefail
out="$1"
root="$(cd "$(dirname "$0")/../.." && pwd)"
png="$root/assets/app-icon/khiankrub-512.png"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
set="$work/KhianKrub.iconset"
mkdir -p "$set"
for s in 16 32 128 256 512; do
  sips -z "$s" "$s" "$png" --out "$set/icon_${s}x${s}.png" >/dev/null
  d=$((s * 2))
  if [ "$d" -le 512 ]; then sips -z "$d" "$d" "$png" --out "$set/icon_${s}x${s}@2x.png" >/dev/null; fi
done
iconutil -c icns "$set" -o "$out"
