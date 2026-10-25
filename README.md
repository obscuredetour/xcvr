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
cargo test --workspace          # also regenerates the TS bindings in crates/app/ui/src/bindings
cd crates/app/ui && pnpm lint   # tsc + prettier
```

## Layout

| Path | What it is |
| --- | --- |
| `crates/app` | The Tauri app: window, menu, commands (and later the poller, tray and handlers) |
| `crates/app/ui` | Solid + TypeScript + Tailwind v4 UI. Reaches the backend only through `src/api.ts` |
| `crates/transmission` | (Phase 2) the RPC client, types and poller diffing, with no Tauri dependency |
