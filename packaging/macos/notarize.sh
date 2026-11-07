#!/usr/bin/env bash
# Send a signed KrubOffice.app or disk image to Apple's notary service, wait for the answer, and
# staple the ticket to it, so macOS opens the app without the "can't be checked for malicious
# software" block, also offline. The release does both: the app first (so the copy a user drags to
# Applications carries its own ticket), then the disk image made from it.
#
#   packaging/macos/notarize.sh dist/KrubOffice.app
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
TARGET="${1:?usage: notarize.sh KrubOffice.app|KrubOffice.dmg}"
[ -e "$TARGET" ] || { echo "no such file: $TARGET"; exit 1; }

if [ -n "${NOTARY_PROFILE:-}" ]; then
    auth=(--keychain-profile "$NOTARY_PROFILE")
elif [ -n "${APPLE_API_KEY_PATH:-}" ] && [ -n "${APPLE_API_KEY_ID:-}" ] && [ -n "${APPLE_API_ISSUER_ID:-}" ]; then
    auth=(--key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY_ID" --issuer "$APPLE_API_ISSUER_ID")
else
    echo "notarize.sh: set NOTARY_PROFILE, or APPLE_API_KEY_PATH, APPLE_API_KEY_ID and APPLE_API_ISSUER_ID"
    exit 1
fi

# Unsigned or ad-hoc signed code is refused by the notary: say so before uploading.
codesign --verify --deep --strict "$TARGET"
if codesign -dv "$TARGET" 2>&1 | grep -q "Signature=adhoc"; then
    echo "notarize.sh: $TARGET is ad-hoc signed; build with MACOS_SIGN_IDENTITY set"
    exit 1
fi

# An app goes up as a zip (the notary takes zip, dmg and pkg); the ticket is stapled to the app.
SUBMIT="$TARGET"
if [ -d "$TARGET" ]; then
    work="$(mktemp -d)"
    trap 'rm -rf "$work"' EXIT
    SUBMIT="$work/$(basename "$TARGET").zip"
    ditto -c -k --keepParent "$TARGET" "$SUBMIT"
fi

echo "Submitting $TARGET to Apple's notary service (this usually takes a few minutes)…"
out="$(xcrun notarytool submit "$SUBMIT" "${auth[@]}" --wait --output-format json)"
echo "$out"
status="$(printf '%s' "$out" | python3 -I -c 'import json, sys; print(json.load(sys.stdin).get("status", ""))')"
if [ "$status" != "Accepted" ]; then
    id="$(printf '%s' "$out" | python3 -I -c 'import json, sys; print(json.load(sys.stdin).get("id", ""))')"
    echo "notarize.sh: Apple answered '$status'. Its log says why:"
    [ -n "$id" ] && xcrun notarytool log "$id" "${auth[@]}" || true
    exit 1
fi

xcrun stapler staple "$TARGET"
xcrun stapler validate "$TARGET"
# What Gatekeeper will say on a user's Mac.
if [ -d "$TARGET" ]; then
    spctl --assess --type execute --verbose=2 "$TARGET"
else
    spctl --assess --type open --context context:primary-signature --verbose=2 "$TARGET"
fi
echo "Notarized and stapled: $TARGET"
