#!/usr/bin/env bash
# Build KrubOffice as an .rpm (x86_64) for Fedora, openSUSE, RHEL-family and other rpm-based distributions.
#
#   packaging/linux/build-deb.sh && packaging/linux/build-rpm.sh
#
# Run after build-deb.sh: it packs the same files the .deb installs (dist/deb/kruboffice_<version>_amd64/usr), so the
# .deb, the AppImage and the .rpm hold identical programs. Writes dist/kruboffice-<version>-1.x86_64.rpm.
#
# Needs rpmbuild (Ubuntu: `sudo apt install rpm`). The run-time libraries the executables link to become the package's
# requirements automatically (rpmbuild reads them from the ELF files); the libraries the programs open at run time
# (Vulkan, xkbcommon for X11, a Thai font, the file-dialog portal) are only recommended, as in the .deb. Built on the
# same Ubuntu 22.04 as the .deb (glibc 2.35), so it needs a distribution with glibc 2.35 or newer: Fedora 36+,
# openSUSE Tumbleweed, RHEL, Rocky and Alma 10 (RHEL 9 has 2.34 and is too old).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
[ -n "$VERSION" ] || { echo "version not found in Cargo.toml" >&2; exit 1; }
[ "$(uname -m)" = x86_64 ] || { echo "the .rpm is built on x86_64 only" >&2; exit 1; }
command -v rpmbuild >/dev/null || { echo "rpmbuild is required (Ubuntu: sudo apt install rpm)" >&2; exit 1; }
DIST="$ROOT/dist"
STAGE="$DIST/deb/kruboffice_${VERSION}_amd64"
[ -x "$STAGE/usr/lib/kruboffice/kruboffice" ] || { echo "run packaging/linux/build-deb.sh first ($STAGE missing)" >&2; exit 1; }

TOP="$DIST/rpmbuild"
rm -rf "$TOP"
mkdir -p "$TOP/SPECS" "$TOP/RPMS" "$TOP/BUILD" "$TOP/BUILDROOT"

cat > "$TOP/SPECS/kruboffice.spec" <<SPEC
# The programs are built and stripped already (build-deb.sh): no debug package, no second strip, no build step.
%global debug_package %{nil}
%global __os_install_post %{nil}
%global __brp_check_rpaths %{nil}
# Require the shared libraries the executables link to, provide none (the programs are not libraries).
AutoProv: no

Name:           kruboffice
Version:        $VERSION
Release:        1
Summary:        KrubOffice: documents, spreadsheets and presentations in one app
License:        MIT OR Apache-2.0
URL:            https://kruboffice.app/
BuildArch:      x86_64
Recommends:     mesa-vulkan-drivers
Recommends:     (vulkan-loader or libvulkan1)
Recommends:     (libxkbcommon-x11 or libxkbcommon-x11-0)
Recommends:     (google-noto-sans-thai-fonts or google-noto-sans-thai-ui-fonts or noto-sans-thai-fonts)
Recommends:     (xdg-desktop-portal or zenity)

%description
KrubOffice bundles KhianKrub (documents), KhitKrub (spreadsheets) and
PhutKrub (presentations), in Thai and English. Open any file and KrubOffice
picks the program. Free and open source.

%install
mkdir -p %{buildroot}
cp -a "$STAGE/usr" %{buildroot}/

%files
%license /usr/share/doc/kruboffice/LICENSE-MIT
%license /usr/share/doc/kruboffice/LICENSE-APACHE
%doc /usr/share/doc/kruboffice/THIRD-PARTY-NOTICES.txt
%dir /usr/share/doc/kruboffice
/usr/lib/kruboffice
/usr/bin/kruboffice
/usr/share/applications/kruboffice*.desktop
/usr/share/icons/hicolor/512x512/apps/kruboffice.png
/usr/share/icons/hicolor/192x192/apps/kruboffice-*.png
SPEC

rpmbuild -bb --define "_topdir $TOP" --define "_build_id_links none" "$TOP/SPECS/kruboffice.spec"
cp "$TOP/RPMS/x86_64/kruboffice-$VERSION-1.x86_64.rpm" "$DIST/"
echo "Built $DIST/kruboffice-$VERSION-1.x86_64.rpm"
