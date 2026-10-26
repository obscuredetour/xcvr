#!/bin/sh
# Point packaging/aur/transceiver-bin at a release: its version, pkgrel 1, and
# the sha256 of its .deb and LICENSE (downloaded from the GitHub release and
# its tag), in PKGBUILD and .SRCINFO alike. Run it once the release is
# published (packaging/aur/README.md).
#
#   scripts/aur-update.sh 0.2.0
set -eu
cd "$(dirname "$0")/.."
version="${1:?usage: scripts/aur-update.sh <version>}"
dir=packaging/aur/transceiver-bin
deb="https://github.com/obscuredetour/xcvr/releases/download/v$version/Transceiver_${version}_amd64.deb"
license="https://raw.githubusercontent.com/obscuredetour/xcvr/v$version/LICENSE"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
sha() {
  if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi
}
echo "aur-update: downloading $deb"
curl -fsSL -o "$tmp/release.deb" "$deb"
curl -fsSL -o "$tmp/LICENSE" "$license"
deb_sum="$(sha "$tmp/release.deb")"
license_sum="$(sha "$tmp/LICENSE")"

# PKGBUILD: the version, and the two checksums (one per line, in source order).
awk -v v="$version" -v d="$deb_sum" -v l="$license_sum" '
  /^pkgver=/ { print "pkgver=" v; next }
  /^pkgrel=/ { print "pkgrel=1"; next }
  /^sha256sums=/ { print "sha256sums=('\''" d "'\''"; getline; print "            '\''" l "'\'')"; next }
  { print }
' "$dir/PKGBUILD" > "$tmp/PKGBUILD" && cat "$tmp/PKGBUILD" > "$dir/PKGBUILD"

# .SRCINFO: the same, spelled out.
awk -v v="$version" -v d="$deb_sum" -v l="$license_sum" -v deb="$deb" -v lic="LICENSE-$version::$license" '
  /^\tpkgver = / { print "\tpkgver = " v; next }
  /^\tpkgrel = / { print "\tpkgrel = 1"; next }
  /^\tsource = / { n++; print "\tsource = " (n == 1 ? deb : lic); next }
  /^\tsha256sums = / { m++; print "\tsha256sums = " (m == 1 ? d : l); next }
  { print }
' "$dir/.SRCINFO" > "$tmp/SRCINFO" && cat "$tmp/SRCINFO" > "$dir/.SRCINFO"

echo "aur-update: $dir is $version (deb $deb_sum)"
