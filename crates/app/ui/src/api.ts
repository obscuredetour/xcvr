// The only way the UI reaches the backend: typed wrappers around the Rust
// commands (crates/app/src/commands.rs) and events. The types come from
// `./bindings`, which ts-rs generates from the Rust structs (`cargo test`).

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';

import type { AddDefaults } from './bindings/AddDefaults';
import type { AddOptions } from './bindings/AddOptions';
import type { AddResult } from './bindings/AddResult';
import type { AddSource } from './bindings/AddSource';
import type { ConnectionSettings } from './bindings/ConnectionSettings';
import type { FreeSpace } from './bindings/FreeSpace';
import type { Metainfo } from './bindings/Metainfo';
import type { TorrentAction } from './bindings/TorrentAction';
import type { ConnectionState } from './bindings/ConnectionState';
import type { Diff } from './bindings/Diff';
import type { Problem } from './bindings/Problem';
import type { Profile } from './bindings/Profile';
import type { SessionStats } from './bindings/SessionStats';
import type { TestResult } from './bindings/TestResult';
import type { Torrent } from './bindings/Torrent';

export type {
  AddDefaults,
  AddOptions,
  AddResult,
  AddSource,
  ConnectionSettings,
  ConnectionState,
  Diff,
  FreeSpace,
  Metainfo,
  Problem,
  Profile,
  SessionStats,
  TestResult,
  Torrent,
  TorrentAction,
};

/** A readable message from whatever a command rejected with (a Problem, a string, an Error). */
export function errorMessage(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err instanceof Error) return err.message;
  if (err && typeof err === 'object' && 'message' in err && typeof err.message === 'string') return err.message;
  return String(err);
}

/** A command's rejection as a Problem (message and hint). */
export function asProblem(err: unknown): Problem {
  if (err && typeof err === 'object' && 'message' in err && typeof err.message === 'string') {
    const hint = 'hint' in err && typeof err.hint === 'string' ? err.hint : null;
    return { message: err.message, hint };
  }
  return { message: errorMessage(err), hint: null };
}

export const api = {
  connectionSettings: () => invoke<ConnectionSettings>('connection_settings'),
  /** `password`: null keeps the saved one, '' removes it. */
  saveConnection: (profile: Profile, password: string | null) =>
    invoke<ConnectionSettings>('save_connection', { profile, password }),
  /** Disconnect and forget the connection, its password too. */
  removeConnection: () => invoke<ConnectionSettings>('remove_connection'),
  /** `password` null uses the one saved for `profile.id`. */
  testConnection: (profile: Profile, password: string | null) =>
    invoke<TestResult>('test_connection', { profile, password }),
  connectionState: () => invoke<ConnectionState>('connection_state'),
  torrents: () => invoke<Torrent[]>('torrents'),
  sessionStats: () => invoke<SessionStats | null>('session_stats'),
  /** Poll now; also retries a failed connection. */
  pollNow: () => invoke<void>('poll_now'),

  torrentAction: (action: TorrentAction, hashes: string[]) => invoke<void>('torrent_action', { action, hashes }),
  /** `deleteData` also deletes the downloaded files on the host. */
  removeTorrents: (hashes: string[], deleteData: boolean) => invoke<void>('remove_torrents', { hashes, deleteData }),
  magnetLinks: (hashes: string[]) => invoke<string[]>('magnet_links', { hashes }),
  /** Name, size and file count of a .torrent file on this machine. */
  readTorrentFile: (path: string) => invoke<Metainfo>('read_torrent_file', { path }),
  addTorrents: (sources: AddSource[], options: AddOptions) => invoke<AddResult[]>('add_torrents', { sources, options }),
  addDefaults: () => invoke<AddDefaults>('add_defaults'),
  freeSpace: (path: string) => invoke<FreeSpace>('free_space', { path }),
};

export type DragDrop =
  { type: 'enter'; paths: string[] } | { type: 'over' } | { type: 'drop'; paths: string[] } | { type: 'leave' };

/** Files dragged over / dropped onto the window (native paths, from Tauri). */
export function onDragDrop(handler: (e: DragDrop) => void): Promise<UnlistenFn> {
  return getCurrentWebview().onDragDropEvent((e) => handler(e.payload as DragDrop));
}

export function onConnection(handler: (state: ConnectionState) => void): Promise<UnlistenFn> {
  return listen<ConnectionState>('connection', (e) => handler(e.payload));
}

export function onTorrents(handler: (diff: Diff) => void): Promise<UnlistenFn> {
  return listen<Diff>('torrents', (e) => handler(e.payload));
}

export function onStats(handler: (stats: SessionStats) => void): Promise<UnlistenFn> {
  return listen<SessionStats>('stats', (e) => handler(e.payload));
}

/** Settings… in the macOS app menu (⌘,). */
export function onOpenSettings(handler: () => void): Promise<UnlistenFn> {
  return listen('open-settings', handler);
}

/** Zoom In / Zoom Out / Actual Size in the macOS View menu. */
export function onZoomMenu(handler: (action: 'in' | 'out' | 'reset') => void): Promise<UnlistenFn> {
  return listen<'in' | 'out' | 'reset'>('zoom', (e) => handler(e.payload));
}
