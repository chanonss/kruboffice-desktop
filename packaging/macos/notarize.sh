#!/usr/bin/env bash
# Send a signed KrubOffice disk image to Apple's notary service, wait for the answer, and staple the
# ticket to it, so macOS opens the app without the "can't be checked for malicious software" block,
# also offline.
#
#   packaging/macos/notarize.sh dist/KrubOffice-X.Y.Z-macos-universal.dmg
#
# Credentials, one of:
#   NOTARY_PROFILE            a keychain profile made once with `xcrun notarytool store-credentials`
#                             (on the owner's Mac)
#   APPLE_API_KEY_PATH        an App Store Connect API key (.p8 file), with
#   APPLE_API_KEY_ID          its Key ID and
#   APPLE_API_ISSUER_ID       the Issuer ID (in the release workflow)
#
# The image and the app in it must already be signed with a Developer ID Application certificate
# and the hardened runtime (`MACOS_SIGN_IDENTITY=... packaging/macos/build-app.sh`, then make-dmg.sh).
set -euo pipefail
DMG="${1:?usage: notarize.sh KrubOffice.dmg}"
[ -f "$DMG" ] || { echo "no such file: $DMG"; exit 1; }

if [ -n "${NOTARY_PROFILE:-}" ]; then
    auth=(--keychain-profile "$NOTARY_PROFILE")
elif [ -n "${APPLE_API_KEY_PATH:-}" ] && [ -n "${APPLE_API_KEY_ID:-}" ] && [ -n "${APPLE_API_ISSUER_ID:-}" ]; then
    auth=(--key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY_ID" --issuer "$APPLE_API_ISSUER_ID")
else
    echo "notarize.sh: set NOTARY_PROFILE, or APPLE_API_KEY_PATH, APPLE_API_KEY_ID and APPLE_API_ISSUER_ID"
    exit 1
fi

# Unsigned or ad-hoc signed images are refused by the notary: say so before uploading.
codesign --verify --strict "$DMG"
if codesign -dv "$DMG" 2>&1 | grep -q "Signature=adhoc"; then
    echo "notarize.sh: $DMG is ad-hoc signed; build with MACOS_SIGN_IDENTITY set"
    exit 1
fi

echo "Submitting $DMG to Apple's notary service (this usually takes a few minutes)…"
out="$(xcrun notarytool submit "$DMG" "${auth[@]}" --wait --output-format json)"
echo "$out"
status="$(printf '%s' "$out" | python3 -I -c 'import json, sys; print(json.load(sys.stdin).get("status", ""))')"
if [ "$status" != "Accepted" ]; then
    id="$(printf '%s' "$out" | python3 -I -c 'import json, sys; print(json.load(sys.stdin).get("id", ""))')"
    echo "notarize.sh: Apple answered '$status'. Its log says why:"
    [ -n "$id" ] && xcrun notarytool log "$id" "${auth[@]}" || true
    exit 1
fi

xcrun stapler staple "$DMG"
xcrun stapler validate "$DMG"
# What Gatekeeper will say on a user's Mac.
spctl --assess --type open --context context:primary-signature --verbose=2 "$DMG"
echo "Notarized and stapled: $DMG"
