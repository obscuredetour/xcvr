# Transceiver (xcvr)

A desktop client for Transmission: Tauri 2, Rust and SolidJS. It talks to a Transmission host on the LAN over RPC and
replaces the stock web UI for day-to-day use. Personal use; macOS and Linux (CachyOS/KDE).

The app shell, theming and UI primitives come from [Magnitide](https://github.com/obscuredetour/magnitide).
[PLAN.md](PLAN.md) has the decisions and the build phases.

## Develop

Needs Rust, pnpm and the Tauri CLI (`cargo install tauri-cli`). On Linux, also WebKitGTK 4.1.

```sh
cd crates/app/ui && pnpm install
cd crates/app && cargo tauri dev
```

## Checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace          # unit and wiremock tests; also regenerates the TS bindings
cd crates/app/ui && pnpm lint   # tsc + prettier
```

The RPC client has two more test suites, both skipped by a plain `cargo test`:

```sh
# Against a throwaway Transmission 4.1.3 in Docker/Podman (OrbStack on macOS): adds and removes torrents.
scripts/test-container.sh

# Read-only, against a real Transmission (the Mac mini). Prints a summary of what it finds.
XCVR_HOST=mac-mini.local XCVR_USER=… XCVR_PASS=… cargo test -p transmission --test smoke -- --ignored --nocapture
```

## Mock Transmission (screenshots)

A stand-in Transmission with a library of freely shared torrents (Linux images, Blender's open movies, public-domain films, open data) that keeps moving: downloads advance and finish, speeds drift, and adding, pausing, labels, renames and moves all work. It holds everything in memory, so restarting it starts fresh.

```bash
cargo run -p transmission --example mock_server
```

Then, in the dev app, connect to `localhost`, port `9092`, with no username. Add `--bind 0.0.0.0` to let another computer connect, or `--port` to use another port. To show original names, map the folder it prints in Settings › Paths.

## Layout

| Path | What it is |
| --- | --- |
| `crates/app` | The Tauri app: window, menu, commands (and later the poller, tray and handlers) |
| `crates/app/ui` | Solid + TypeScript + Tailwind v4 UI. Reaches the backend only through `src/api.ts` |
| `crates/transmission` | The RPC client (JSON-RPC 2.0, Transmission 4.1+) and its types, with no Tauri dependency |
| `crates/transmission/examples/mock_server.rs` | The mock Transmission above |
| `scripts/test-container.sh` | Runs the client's integration tests against Transmission in a container |

## License

MIT, see [LICENSE](LICENSE). The Catppuccin and Dracula themes' colours come from those projects, also MIT.
