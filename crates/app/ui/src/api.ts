// The only way the UI reaches the backend: typed wrappers around the Rust
// commands and events. Argument and result types will come from `./bindings`,
// which ts-rs generates from the Rust structs (`cargo test` regenerates them).

import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** A readable message from whatever a command rejected with. */
export function errorMessage(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err instanceof Error) return err.message;
  return String(err);
}

/** Settings… in the macOS app menu (⌘,). */
export function onOpenSettings(handler: () => void): Promise<UnlistenFn> {
  return listen('open-settings', handler);
}

/** Zoom In / Zoom Out / Actual Size in the macOS View menu. */
export function onZoomMenu(handler: (action: 'in' | 'out' | 'reset') => void): Promise<UnlistenFn> {
  return listen<'in' | 'out' | 'reset'>('zoom', (e) => handler(e.payload));
}
