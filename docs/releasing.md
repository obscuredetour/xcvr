# Releasing

How a version of Transceiver gets built and published. The [README](../README.md) says how to build
and test it day to day.

1. Set the version in `Cargo.toml` (`[workspace.package]`, which every crate shares) and
   `crates/app/ui/package.json`, and push that to `main`.
2. Tag it with that version: `git tag v<version> && git push origin v<version>`.
   `.github/workflows/release.yml` builds macOS (Apple silicon and Intel `.dmg`s) and Linux
   (`.AppImage`, `.deb`, `.rpm`) into a draft release.
3. Look the draft over and publish it. The in-app updater reads the latest release's `latest.json`,
   so publishing is what offers it to everyone.
4. Update the Arch package: `scripts/aur-update.sh <version>`, and commit it
   ([packaging/aur/README.md](../packaging/aur/README.md)).

The macOS builds are ad-hoc signed, not notarized (there's no Apple Developer ID): they run, but the
first time, Gatekeeper asks the user to confirm (System Settings › Privacy & Security › Open Anyway),
and the release notes say so.

## Updates

Transceiver updates itself (Settings › About, and a toast when the daily check finds a version)
from the latest release's `latest.json`, which `tauri-action` writes from the builds' updater
signatures. The macOS app, the AppImage, and the `.deb` and `.rpm` (while dpkg or rpm owns the
program) update this way; the Arch package and source builds are left to their package manager
(`crates/app/src/updates.rs`).

The installers are signed with an updater key, and the release fails without it. Make it once:

```sh
cargo tauri signer generate -w ~/.tauri/xcvr.key
```

It asks for a password (optional). Then:

- Put its public half, the contents of `~/.tauri/xcvr.key.pub`, in `plugins.updater.pubkey` in
  `crates/app/tauri.conf.json`.
- Add the private key and its password as repository secrets (Settings › Secrets and variables ›
  Actions), or with the GitHub CLI:

  ```sh
  gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/xcvr.key
  gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD   # only if you gave it one
  ```

Keep the private key somewhere safe as well as in the secret: every copy out there accepts only
updates signed with it, so a lost key means people have to download the next version by hand, once,
to get one with a new public key.

To try a signed build locally:

```sh
TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/xcvr.key)" cargo tauri build --config '{"bundle":{"createUpdaterArtifacts":true}}'
```
