# Arch Linux

`transceiver-bin/` packages each release's `.deb` from GitHub for Arch (no build), meant for the
[AUR](https://aur.archlinux.org) as `transceiver-bin`. It isn't on the AUR yet, so for now install
it from this folder:

```sh
git clone https://github.com/obscuredetour/xcvr.git
cd xcvr/packaging/aur/transceiver-bin
makepkg -si
```

`makepkg -si` downloads the release, checks it, and installs it with pacman (asking for your
password). Pull and run it again for a newer version.

Installed this way, Transceiver leaves updates to pacman: its own updater (for the AppImage, `.deb`
and `.rpm`) sees that dpkg doesn't own the program, and Settings › About says so.

## Each release

Once the release is published:

```sh
scripts/aur-update.sh 0.2.0
```

sets the version and the checksums of the `.deb` and `LICENSE` in `PKGBUILD` and `.SRCINFO`. Commit
them here.

For a change beyond a new version, check it on Arch: `namcap PKGBUILD`, then `makepkg -si`, then
`namcap` on the built package, and regenerate `.SRCINFO` with `makepkg --printsrcinfo > .SRCINFO`.

## Publishing it on the AUR

When AUR registration reopens:

1. Make an account at aur.archlinux.org and add an SSH public key to it (My Account › SSH Public Key).
2. Clone the package's repository (empty until the first push; cloning a new name creates it):
   ```sh
   git clone ssh://aur@aur.archlinux.org/transceiver-bin.git
   ```
3. Copy `PKGBUILD` and `.SRCINFO` into it, then commit and push:
   ```sh
   cp packaging/aur/transceiver-bin/{PKGBUILD,.SRCINFO} ../transceiver-bin/
   cd ../transceiver-bin && git add PKGBUILD .SRCINFO && git commit -m "transceiver-bin 0.1.0" && git push
   ```

After that, each release is `scripts/aur-update.sh`, then the same copy, commit and push.
