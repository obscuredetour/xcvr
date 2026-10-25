# Transceiver (xcvr) — Build Plan

Companion to the project brief. Decisions here override the brief where they differ.

## Decisions

| Topic | Decision |
| --- | --- |
| Baseline | Fork Magnitide's app shell, theming, tooling and UI primitives (copy, not a shared package). |
| Identifier | `com.obscuredetour.xcvr` (Magnitide's is actually `com.magnitide.app`; this follows the GitHub org). |
| RPC protocol | JSON-RPC 2.0, snake_case keys only. The mini runs 4.1.1 (rpc semver 6.0.1); the legacy format is deprecated. No dual support. |
| Torrent identity | Key everything by `hash_string`. Integer ids are not stable across Transmission restarts. RPC calls can take hashes in `ids`. |
| Tags | Use Transmission's `labels`. They live on the host, so both clients see the same tags, and other clients can read them too. |
| Polling | Full fetch of list fields every tick (tens of torrents makes this cheap). Diff by hash in Rust. No `recently_active` and no `removed` bookkeeping. |
| Sorting / filtering / grouping | In TypeScript. At this size there's no reason to do it in Rust (Magnitide does it in Rust for 50k books). |
| Window close | Hides the window; the app keeps running in the tray so notifications still fire. Quit from the tray or ⌘Q. Dock icon stays on macOS. |
| Secrets | RPC password in the system keychain via Magnitide's `keys.rs` (macOS Keychain / KWallet via Secret Service). |
| Linux target | CachyOS, Plasma, Wayland. Package with a local PKGBUILD, not `.deb`/`.rpm`. |
| Path mapping | SMB is mounted on both clients, so Open/Reveal moves up into the inspector work. |

## Reuse map from Magnitide

**Take as-is or lightly renamed**

- Repo tooling: `rustfmt.toml`, `.cargo/config.toml` (ts-rs export dir), prettier config, trimmed `checks.yml`.
- `crates/app` skeleton: `build.rs`, `tauri.conf.json` (overlay title bar, traffic lights, CSP), capabilities, `linux/*.desktop` template, `main.rs` setup incl. `glib::set_prgname`, `menu.rs` (Settings ⌘, and zoom), `keys.rs`.
- Theming: `styles.css`, `themes/*`, `theme.ts`, `AccentPicker`, `ThemeMenu`, `settings/Appearance`, `zoom.ts`.
- Primitives: `Menu` / `ContextMenu` / `SubMenu`, `confirm.tsx`, `toast.tsx`, `question.tsx`, `Select`, `Autocomplete`, `Icon` (+ `scripts/fetch-lucide-icons.py`), `buttons.ts`, `widths.ts`, `storage.ts`, `keys.ts` + `settings/Shortcuts`, `settings/parts.tsx`, `settings/Settings` shell, `EmptyState`, `DropOverlay`, `scripts/app-icon.sh`.

**Adapt**

- `BookTable` → `TorrentTable`: keep column sizing/order/visibility, virtualizer, selection model, header keyboard moves, column picker. Replace columns; sort in TS.
- `DetailsPanel` shell (resize handle, persisted width, `widthFlags`) → `Inspector` right sidebar.
- `Sidebar` (shelves pattern) → status filters + labels with counts.
- `view.ts` → persisted column state, sort, group-by, inspector width.
- `TopBar`, `StatusBar`, `SelectionBar` → search, global speeds and connection state, bulk actions.
- `api.ts` pattern: the only path to the backend, typed from `bindings/`.

**Leave behind**

catalog, core, plugins, plugins-wasm, plugin-sdk, server, sources, calibre-compat, covers, devices, reader, updater, release workflow.

## Repo layout

```
xcvr/
  Cargo.toml                 workspace
  crates/
    transmission/            RPC client, types, poller diff, completion detection. No Tauri.
    app/                     Tauri shell: commands, poller task, tray, notifications, handlers, keychain
      ui/                    Solid + TS + Tailwind v4 (from Magnitide)
  packaging/arch/PKGBUILD
  docs/architecture.md
```

The `transmission` crate has no Tauri dependency, following Magnitide's rule of keeping business logic out of commands. Its tests run against a mock server and a Transmission container.

## RPC notes (4.1.x)

- `POST /transmission/rpc`, `{"jsonrpc":"2.0","method":"torrent_get","params":{...},"id":n}`. `params` must be an object.
- The 409 → `X-Transmission-Session-Id` handshake and HTTP Basic auth still apply.
- Errors come back as a JSON-RPC `error` object. Map them to typed errors and show a toast. `torrent_add` can report a duplicate; treat that as a soft message.
- List fields per tick: `id, hash_string, name, status, percent_done, metadata_percent_complete, total_size, size_when_done, left_until_done, rate_download, rate_upload, eta, upload_ratio, uploaded_ever, added_date, done_date, activity_date, seconds_seeding, is_finished, is_stalled, error, error_string, queue_position, labels, download_dir, peers_connected`.
- Inspector-only fields (polled only for the open torrent): `files, file_stats, peers, tracker_stats, tracker_list, piece_count, piece_size, comment, creator, date_created, magnet_link`, per-torrent limits.
- `wanted` is a boolean array in 4.1.
- `labels` are replaced as a whole by `torrent_set`. Adding a tag to many torrents means a read-modify-write per torrent, or grouping torrents by their current label set into batches. Transmission trims labels and refuses empty ones and commas (`label::normalize` does the same, so the UI can say so before sending).
- The 409 also carries `X-Transmission-Rpc-Version`; the client refuses servers below 6.0 (Transmission 4.0 and older) with a clear message.
- `free_space` on a folder that doesn't exist is an RPC `SystemError` ("No such file or directory"). The add dialog should show "folder not found yet", not an error toast. A fresh Transmission's default download folder may not exist until a torrent finishes.
- Renaming works on torrents whose data isn't on disk yet; Transmission renames the metadata.
- Methods in use: `torrent_get/add/set/start/start_now/stop/verify/reannounce/remove/set_location/rename_path`, `queue_move_top/up/down/bottom`, `session_get/set`, `session_stats`, `free_space`.

## Poller rules

- Tick: 1.5s when the window is visible; 5s when it's hidden (tray only); `session_stats` every tick.
- Poll immediately after any command, so actions show up without waiting a tick.
- Diff by `hash_string`: emit added, changed (changed fields only) and removed hashes. The frontend patches a Solid store keyed by hash.
- Completion: `done_date` goes from 0 to non-zero, or `left_until_done` reaches 0 while the previous status was downloading. Never fire from the first fetch after a connect. Ignore drops and recoveries caused by verify.
- Connection states: `connecting`, `connected`, `auth_failed`, `unreachable`, with exponential backoff (max 30s). Show the state in the status bar and tray, and disable commands while not connected. Reset on system wake.

## Phases

Each phase ends with something runnable on both the Mac and the CachyOS machine.

**0. Prep (you)**
- Update the mini to Transmission 4.1.3 (4.1.4 is also out; either works, both speak RPC 6.0.1).
- Note the host download path and the SMB mount path on each client.

**1. Scaffold from Magnitide**
- `git init`, workspace, app crate, UI with themes and primitives; rename to xcvr/Transceiver.
- Empty shell: top bar, sidebar, empty table, empty inspector, settings dialog with Appearance and Shortcuts.
- Done when `pnpm dev` + `cargo tauri dev` run on both machines and fmt, clippy, tsc and prettier pass.

**2. `transmission` crate**
- Client: config, session-id retry, Basic auth, typed requests and responses for every method above, error mapping.
- Tests: mock server (409 retry, 401, RPC error); integration tests against a throwaway `linuxserver/transmission` container (pinned to 4.1.x); one read-only smoke test against the mini, behind an env var.
- Done: 12 unit tests, 11 mock-server tests, 6 container tests against 4.1.3 (`scripts/test-container.sh`), and the smoke test (checked against a container; still to run against the mini).

**3. Poller + connection**
- Tokio task in the app crate; diff and completion logic in the `transmission` crate, with unit tests.
- Connection settings (host, port, user, password → keychain), Test Connection, connection profiles stored as a list.
- macOS Local Network privacy: without permission, connecting to a LAN host fails with "No route to host" (EHOSTUNREACH) even though the host is up. `crates/app/Info.plist` gives the prompt its reason. When Test Connection gets EHOSTUNREACH on macOS, say to allow Transceiver in System Settings → Privacy & Security → Local Network, not just "couldn't reach".
- Done:
  - `Snapshot`/`Diff` in the `transmission` crate, with unit tests.
  - The poller (`crates/app/src/poller.rs`), which emits `connection`, `torrents` and `stats` events.
  - A wrong password stops polling instead of retrying, since Transmission's anti-brute-force can lock out an address.
  - A wall-clock jump means the computer woke from sleep, and it triggers a full resync.
  - Profiles live in `connections.json`, passwords in the keychain (`secrets.rs`).
  - Settings › Connection has Test and Save & Connect.
  - The status bar and sidebar counts are live.
  - The main area shows a simple live list until phase 4's table.
  - The poller is tested in Tauri's mock app against the container.

**4. Vertical slice: live read-only table**
- Store keyed by hash, patched from diffs. `TorrentTable` columns: name, status, progress, size, down, up, ETA, ratio, added, labels.
- Status filters, text search, sort. Status bar shows global speeds.
- Checkpoint: watch real torrents update on both machines before building further.

**5. Actions + add**
- Row and bulk actions: start, start now, stop, verify, reannounce, queue moves, remove (confirm dialog, optional data delete).
- Selection (click, ⌘/Ctrl, Shift, select all), keyboard shortcuts, context menu.
- Add dialog: paste magnet, pick `.torrent` (dialog plugin), drop on window. Download dir with recent/favourites and free space, labels, start paused.

**6. Inspector sidebar (must-have)**
- Tabs or sections: General, Files (tree, priority, wanted), Peers, Trackers, Options (speed limits, seed ratio/idle limits, bandwidth priority).
- Rename (`torrent_rename_path`) and Set Location (move data, or point at existing data).
- Open / Reveal through path mapping (settings: host prefix → local prefix per client).

**7. Labels (tags)**
- Sidebar label list with counts, with rename and delete across torrents.
- Edit labels on the selection with autocomplete; labels in the add dialog.
- Filter, sort and group by label in the table (collapsible group headers).

**8. Preferences (must-have)**
- `session_get/set`: default download dir, incomplete dir, global and alt speed limits + schedule, queue sizes, seeding limits, peer port and limits, encryption, blocklist.
- Alt-speed (turtle) toggle in the top bar.

**9. Tray, notifications, handlers**
- Tray menu: speeds, active count, pause all / resume all, add magnet from clipboard, show window, quit. On Linux, tray clicks don't fire events, so speeds go in the menu and tooltip.
- Completion notifications.
- single-instance (with `deep-link` feature), `magnet:` deep link, `.torrent` association (`RunEvent::Opened` on macOS, argv on Linux), autostart.
- Show-window shortcut: global-shortcut on macOS. On Wayland it doesn't work, so add an `xcvr --show` argument (forwarded by single-instance) and bind it in KDE System Settings → Shortcuts.
- Handlers only work in built apps: install a local `.app` and a local PKGBUILD build to test.

**10. Clean up view**
- A dialog or saved view that lists candidates: seeding longer than N days, ratio above X, no upload activity in N days, or finished (hit its seed limit).
- Checkbox list with totals (space freed), bulk remove with or without data. All from fields the poller already has, so it's cheap.

**11. Packaging**
- macOS: `.app` / `.dmg`, unsigned.
- CachyOS: `packaging/arch/PKGBUILD` building from the local checkout, installing the `.desktop` with `MimeType=x-scheme-handler/magnet;application/x-bittorrent;`.
- Optional: GitHub Actions checks (Linux), like Magnitide's.

MVP = phases 1–9. Phase 10 and the brief's "Later" items (multiple host profiles UI, saved filters, tracker editing) follow.

## Answered

1. **Grouping by label:** Gmail-style. A torrent with two labels shows in both groups, plus an "Unlabeled" group. Selection is by hash, so selecting a torrent highlights it in every group it appears in, and bulk actions never run twice. A torrent's first label still sorts first in its label chips, in case a "primary label" idea is wanted later.
2. **Integration tests:** OrbStack on the Mac runs the Transmission container; the same tests run on CachyOS with Docker or Podman.
3. **GitHub:** a private repo under `obscuredetour`, set up when CI is wanted.
