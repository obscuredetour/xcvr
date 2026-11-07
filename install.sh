#!/bin/sh
# Installs Transceiver's AppImage from its latest release, for you, in ~/.local:
#
#   curl -fsSL https://github.com/obscuredetour/xcvr/releases/latest/download/install.sh | sh
#
# The AppImage goes in ~/.local/lib/transceiver, with `xcvr` in ~/.local/bin,
# an app-menu entry and its icon; magnet links and .torrent files open in
# Transceiver unless something else already opens them. No sudo. From then on
# it updates itself (Settings › About); running this again updates it too.
#
#   sh install.sh --uninstall    removes it (your connection and settings stay)
#
# XCVR_VERSION=v0.2.0 installs that release instead of the latest.
# XCVR_RELEASE_URL=file:///…/dir installs from a folder holding a release's
# files (the AppImage, transceiver.png and SHA256SUMS).

set -eu

repo="obscuredetour/xcvr"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
lib="$HOME/.local/lib/transceiver"
app="$lib/Transceiver.AppImage"
bin="$HOME/.local/bin"
# Named after the window's app_id, so the desktop finds the window's icon through it.
desktop="$data/applications/Transceiver.desktop"
icon="$data/icons/hicolor/256x256/apps/xcvr.png"
# Marks the menu entry as this script's.
marker="X-Transceiver-Installer=install.sh"
types="x-scheme-handler/magnet application/x-bittorrent"

say() { printf '%s\n' "$*"; }
fail() {
  printf 'transceiver: %s\n' "$*" >&2
  exit 1
}

main() {
  case "${1:-}" in
    "") install ;;
    --uninstall) uninstall ;;
    -h | --help) say "Usage: install.sh [--uninstall]" ;;
    *) fail "unknown option $1 (try --help)" ;;
  esac
}

install() {
  [ "$(id -u)" -ne 0 ] || fail "run this as yourself, not as root: Transceiver installs for you, in ~/.local."
  [ "$(uname -s)" = Linux ] || fail "this installs the Linux AppImage. On macOS, use the .dmg from the release."
  arch="$(uname -m)"
  [ "$arch" = x86_64 ] || fail "Transceiver's AppImage is for x86_64 so far, and this is $arch."
  command -v curl >/dev/null || fail "curl isn't installed."
  command -v sha256sum >/dev/null || fail "sha256sum isn't installed (it comes with coreutils)."

  if [ -n "${XCVR_RELEASE_URL:-}" ]; then
    url="$XCVR_RELEASE_URL"
  elif [ -n "${XCVR_VERSION:-}" ]; then
    url="https://github.com/$repo/releases/download/$XCVR_VERSION"
  else
    url="https://github.com/$repo/releases/latest/download"
  fi

  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  curl -fsSL -o "$tmp/SHA256SUMS" "$url/SHA256SUMS" ||
    fail "couldn't download SHA256SUMS from $url (releases before 0.2.0 don't have one)."
  # The AppImage's name has the version in it: Transceiver_0.2.0_amd64.AppImage.
  name="$(sed -n 's/^[0-9a-f]\{64\}  \(Transceiver_.*\.AppImage\)$/\1/p' "$tmp/SHA256SUMS" | head -n 1)"
  [ -n "$name" ] || fail "SHA256SUMS doesn't list an AppImage."
  version="$(printf '%s' "$name" | sed 's/^Transceiver_\(.*\)_amd64\.AppImage$/\1/')"

  say "Downloading Transceiver $version"
  curl -fL --progress-bar -o "$tmp/$name" "$url/$name" || fail "couldn't download $name."
  curl -fsSL -o "$tmp/transceiver.png" "$url/transceiver.png" || fail "couldn't download transceiver.png."
  (cd "$tmp" && sha256sum --check --quiet --ignore-missing SHA256SUMS) ||
    fail "the download doesn't match its checksum; try again."
  [ -s "$tmp/transceiver.png" ] && grep -q " transceiver.png$" "$tmp/SHA256SUMS" ||
    fail "SHA256SUMS doesn't list transceiver.png."

  mkdir -p "$lib" "$bin" "$data/applications" "$(dirname "$icon")"
  # Renamed into place, so a Transceiver that's running keeps the file it started from.
  cp "$tmp/$name" "$lib/.Transceiver.AppImage.new"
  chmod 755 "$lib/.Transceiver.AppImage.new"
  mv -f "$lib/.Transceiver.AppImage.new" "$app"
  ln -sfn "$app" "$bin/xcvr"
  cp "$tmp/transceiver.png" "$icon"

  # Magnet links and .torrent files: Transceiver's, unless something else already has them
  # (asked before the entry's written, which would answer for itself).
  unclaimed=""
  if command -v xdg-mime >/dev/null; then
    for type in $types; do
      [ -n "$(xdg-mime query default "$type" 2>/dev/null)" ] || unclaimed="$unclaimed $type"
    done
  fi

  # The entry runs the AppImage by its full path: ~/.local/bin isn't always on PATH.
  cat >"$desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Transceiver
GenericName=Torrent Client
Comment=A desktop client for Transmission
Exec="$app" %U
Icon=xcvr
StartupWMClass=Transceiver
Categories=Network;FileTransfer;P2P;
MimeType=x-scheme-handler/magnet;application/x-bittorrent;
Terminal=false
$marker
EOF
  command -v update-desktop-database >/dev/null && update-desktop-database "$data/applications" 2>/dev/null || true

  [ -z "$unclaimed" ] || mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}"
  for type in $unclaimed; do
    xdg-mime default Transceiver.desktop "$type" 2>/dev/null || true
  done

  say "Installed Transceiver $version in $lib."
  if ! command -v fusermount3 >/dev/null && ! command -v fusermount >/dev/null; then
    say "AppImages need FUSE to run, and it isn't installed. $(install_hint)"
  fi
  case ":$PATH:" in
    *":$bin:"*) ;;
    *) say "$bin isn't on your PATH, so start Transceiver from your app menu (or add it to PATH)." ;;
  esac
  say "Open Transceiver from your app menu (if it's open, quit it and open it again)."
}

# How to install FUSE on this distribution, if it's one we know.
install_hint() {
  [ -r /etc/os-release ] || return 0
  # shellcheck disable=SC1091
  distro="$(. /etc/os-release && echo "${ID:-} ${ID_LIKE:-}")"
  case " $distro " in
    *" arch "*) say "Install it with: sudo pacman -S --needed fuse3" ;;
    *" debian "* | *" ubuntu "*) say "Install it with: sudo apt install fuse3" ;;
    *" fedora "* | *" rhel "*) say "Install it with: sudo dnf install fuse3" ;;
    *suse*) say "Install it with: sudo zypper install fuse3" ;;
  esac
}

uninstall() {
  [ "$(id -u)" -ne 0 ] || fail "run this as yourself, not as root, so it finds your installation."

  rm -f "$app"
  rmdir "$lib" 2>/dev/null || true
  # Only our link and entry; a Transceiver installed some other way stays.
  if [ -L "$bin/xcvr" ] && [ "$(readlink "$bin/xcvr")" = "$app" ]; then
    rm -f "$bin/xcvr"
  fi
  if [ -f "$desktop" ] && grep -q "^$marker$" "$desktop"; then
    rm -f "$desktop"
    command -v update-desktop-database >/dev/null && update-desktop-database "$data/applications" 2>/dev/null || true
  fi
  rm -f "$icon"

  say "Removed Transceiver. Your connection and settings stay, in ~/.config/com.obscuredetour.xcvr."
}

# Everything above only defines functions, so a download that's cut short
# runs nothing.
main "$@"
