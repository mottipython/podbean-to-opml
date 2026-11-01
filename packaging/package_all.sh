#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_DIR="${REPO_DIR}/dist/packages"
BIN_SRC="${REPO_DIR}/target/release/podcast-rescue"

VERSION="0.2.0"
PKG_NAME="podcast-rescue"

echo "================================================================="
echo " Packaging ${PKG_NAME} v${VERSION} for RPM, DEB, and Arch"
echo "================================================================="

# Ensure binary is built
if [ ! -f "${BIN_SRC}" ]; then
  echo "[*] Compiling release binary..."
  (cd "${REPO_DIR}" && cargo build --release)
fi

mkdir -p "${DIST_DIR}"
BUILD_TMP="$(mktemp -d /tmp/pkg_build.XXXXXX)"
trap 'rm -rf "${BUILD_TMP}"' EXIT

# Shared desktop file
cat <<EOF > "${BUILD_TMP}/${PKG_NAME}.desktop"
[Desktop Entry]
Name=Podcast Rescue
Comment=Universal Podcast Subscription Rescue & OPML Exporter
Exec=podcast-rescue
Icon=audio-x-generic
Terminal=false
Type=Application
Categories=AudioVideo;Audio;Network;
Keywords=podcast;opml;podbean;spotify;apple;export;
EOF

# -------------------------------------------------------------
# 1. BUILD RPM PACKAGE
# -------------------------------------------------------------
echo "[*] Building RPM package..."
RPM_ROOT="${BUILD_TMP}/rpmbuild"
mkdir -p "${RPM_ROOT}"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

cat <<EOF > "${RPM_ROOT}/SPECS/${PKG_NAME}.spec"
Name:           ${PKG_NAME}
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        Universal Podcast Subscription Rescue and OPML Exporter
License:        MIT
URL:            https://github.com/mottipython/podbean-to-opml

%description
Universal cross-platform tool to rescue podcast subscriptions from walled-garden
podcast apps (Podbean, Spotify, Apple Podcasts, YouTube Music) into standard OPML format.

%prep
# No prep needed

%build
# Pre-compiled

%install
rm -rf \$RPM_BUILD_ROOT
mkdir -p \$RPM_BUILD_ROOT%{_bindir}
mkdir -p \$RPM_BUILD_ROOT%{_datadir}/applications
mkdir -p \$RPM_BUILD_ROOT%{_datadir}/doc/%{name}

install -m 755 ${BIN_SRC} \$RPM_BUILD_ROOT%{_bindir}/${PKG_NAME}
ln -sf ${PKG_NAME} \$RPM_BUILD_ROOT%{_bindir}/podbean-to-opml
install -m 644 ${BUILD_TMP}/${PKG_NAME}.desktop \$RPM_BUILD_ROOT%{_datadir}/applications/
install -m 644 ${REPO_DIR}/README.md \$RPM_BUILD_ROOT%{_datadir}/doc/%{name}/
install -m 644 ${REPO_DIR}/LICENSE \$RPM_BUILD_ROOT%{_datadir}/doc/%{name}/

%files
%{_bindir}/${PKG_NAME}
%{_bindir}/podbean-to-opml
%{_datadir}/applications/${PKG_NAME}.desktop
%{_datadir}/doc/%{name}/README.md
%{_datadir}/doc/%{name}/LICENSE

%changelog
* Sun Nov 01 2026 Motti Schleider <motti.schleider@gmail.com> - 0.2.0-1
- Universal cross-platform release with browser GUI and multi-platform support
EOF

rpmbuild --define "_topdir ${RPM_ROOT}" -bb "${RPM_ROOT}/SPECS/${PKG_NAME}.spec" >/dev/null
find "${RPM_ROOT}/RPMS" -name "*.rpm" -exec cp {} "${DIST_DIR}/" \;
echo "    [+] RPM created: $(ls "${DIST_DIR}"/*.rpm | head -1)"

# -------------------------------------------------------------
# 2. BUILD DEB PACKAGE (Debian / Ubuntu / Pop!_OS / Mint)
# -------------------------------------------------------------
echo "[*] Building DEB package..."
DEB_ROOT="${BUILD_TMP}/deb"
mkdir -p "${DEB_ROOT}/data/usr/bin"
mkdir -p "${DEB_ROOT}/data/usr/share/applications"
mkdir -p "${DEB_ROOT}/data/usr/share/doc/${PKG_NAME}"
mkdir -p "${DEB_ROOT}/control"

cp "${BIN_SRC}" "${DEB_ROOT}/data/usr/bin/${PKG_NAME}"
chmod 755 "${DEB_ROOT}/data/usr/bin/${PKG_NAME}"
ln -sf "${PKG_NAME}" "${DEB_ROOT}/data/usr/bin/podbean-to-opml"
cp "${BUILD_TMP}/${PKG_NAME}.desktop" "${DEB_ROOT}/data/usr/share/applications/"
cp "${REPO_DIR}/README.md" "${DEB_ROOT}/data/usr/share/doc/${PKG_NAME}/"
cp "${REPO_DIR}/LICENSE" "${DEB_ROOT}/data/usr/share/doc/${PKG_NAME}/copyright"

INSTALLED_SIZE=$(du -sk "${DEB_ROOT}/data" | cut -f1)

cat <<EOF > "${DEB_ROOT}/control/control"
Package: ${PKG_NAME}
Version: ${VERSION}-1
Architecture: amd64
Maintainer: Motti Schleider <motti.schleider@gmail.com>
Installed-Size: ${INSTALLED_SIZE}
Section: sound
Priority: optional
Homepage: https://github.com/mottipython/podbean-to-opml
Description: Universal Podcast Subscription Rescue and OPML Exporter
 Rescue your podcast subscriptions from walled gardens (Podbean, Spotify,
 Apple Podcasts, YouTube Music) into standard OPML format with a guided browser GUI.
EOF

echo "2.0" > "${DEB_ROOT}/debian-binary"
(cd "${DEB_ROOT}/control" && tar --owner=0 --group=0 --numeric-owner -czf "${BUILD_TMP}/control.tar.gz" .)
(cd "${DEB_ROOT}/data" && tar --owner=0 --group=0 --numeric-owner -czf "${BUILD_TMP}/data.tar.gz" .)

DEB_OUT="${DIST_DIR}/${PKG_NAME}_${VERSION}-1_amd64.deb"
(cd "${BUILD_TMP}" && ar rcs "${DEB_OUT}" deb/debian-binary control.tar.gz data.tar.gz)
echo "    [+] DEB created: ${DEB_OUT}"

# -------------------------------------------------------------
# 3. BUILD ARCH LINUX PACKAGE (.pkg.tar.zst) & PKGBUILD
# -------------------------------------------------------------
echo "[*] Building Arch Linux package (.pkg.tar.zst)..."
ARCH_ROOT="${BUILD_TMP}/arch"
mkdir -p "${ARCH_ROOT}/usr/bin"
mkdir -p "${ARCH_ROOT}/usr/share/applications"
mkdir -p "${ARCH_ROOT}/usr/share/doc/${PKG_NAME}"
mkdir -p "${ARCH_ROOT}/usr/share/licenses/${PKG_NAME}"

cp "${BIN_SRC}" "${ARCH_ROOT}/usr/bin/${PKG_NAME}"
chmod 755 "${ARCH_ROOT}/usr/bin/${PKG_NAME}"
ln -sf "${PKG_NAME}" "${ARCH_ROOT}/usr/bin/podbean-to-opml"
cp "${BUILD_TMP}/${PKG_NAME}.desktop" "${ARCH_ROOT}/usr/share/applications/"
cp "${REPO_DIR}/README.md" "${ARCH_ROOT}/usr/share/doc/${PKG_NAME}/"
cp "${REPO_DIR}/LICENSE" "${ARCH_ROOT}/usr/share/licenses/${PKG_NAME}/"

BUILD_DATE=$(date -u +%s)
TOTAL_SIZE=$(du -sb "${ARCH_ROOT}" | cut -f1)

cat <<EOF > "${ARCH_ROOT}/.PKGINFO"
pkgname = ${PKG_NAME}
pkgbase = ${PKG_NAME}
pkgver = ${VERSION}-1
pkgdesc = Universal Podcast Subscription Rescue and OPML Exporter
url = https://github.com/mottipython/podbean-to-opml
builddate = ${BUILD_DATE}
packager = Motti Schleider <motti.schleider@gmail.com>
size = ${TOTAL_SIZE}
arch = x86_64
license = MIT
conflict = podbean-to-opml
provides = podbean-to-opml
EOF

ARCH_OUT="${DIST_DIR}/${PKG_NAME}-${VERSION}-1-x86_64.pkg.tar.zst"
(cd "${ARCH_ROOT}" && tar --owner=0 --group=0 --numeric-owner -cf - .PKGINFO usr | zstd -T0 -19 -o "${ARCH_OUT}")
echo "    [+] Arch package created: ${ARCH_OUT}"

# Also generate PKGBUILD for Arch AUR users
cat <<EOF > "${DIST_DIR}/PKGBUILD"
# Maintainer: Motti Schleider <motti.schleider@gmail.com>
pkgname=podcast-rescue
pkgver=${VERSION}
pkgrel=1
pkgdesc="Universal Podcast Subscription Rescue and OPML Exporter"
arch=('x86_64')
url="https://github.com/mottipython/podbean-to-opml"
license=('MIT')
depends=('glibc')
makedepends=('cargo' 'rust')
provides=('podbean-to-opml')
conflicts=('podbean-to-opml')
source=("\$pkgname-\$pkgver.tar.gz::https://github.com/mottipython/podbean-to-opml/archive/refs/tags/v\$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
  cd "\$srcdir/podbean-to-opml-\$pkgver"
  cargo build --release --locked
}

package() {
  cd "\$srcdir/podbean-to-opml-\$pkgver"
  install -Dm755 target/release/podcast-rescue "\$pkgdir/usr/bin/podcast-rescue"
  ln -sf podcast-rescue "\$pkgdir/usr/bin/podbean-to-opml"
  install -Dm644 LICENSE "\$pkgdir/usr/share/licenses/\$pkgname/LICENSE"
  install -Dm644 README.md "\$pkgdir/usr/share/doc/\$pkgname/README.md"
}
EOF
echo "    [+] Arch PKGBUILD created: ${DIST_DIR}/PKGBUILD"

echo ""
echo "================================================================="
echo " Packaging completed successfully! Artifacts in dist/packages/:"
echo "================================================================="
ls -lh "${DIST_DIR}"/
echo "================================================================="
