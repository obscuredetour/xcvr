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
import type { MappingView } from './bindings/MappingView';
import type { Metainfo } from './bindings/Metainfo';
import type { PathMapping } from './bindings/PathMapping';
import type { PortTest } from './bindings/PortTest';
import type { Renamed } from './bindings/Renamed';
import type { Session } from './bindings/Session';
import type { SessionSet } from './bindings/SessionSet';
import type { TorrentAction } from './bindings/TorrentAction';
import type { TorrentSet } from './bindings/TorrentSet';
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
  MappingView,
  Metainfo,
  PathMapping,
  PortTest,
  Problem,
  Profile,
  Session,
  SessionSet,
  SessionStats,
  TestResult,
  Torrent,
  TorrentAction,
  TorrentSet,
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

  /** Follow a torrent in the inspector (its details now, then `details` events); null stops. */
  inspect: (hash: string | null) => invoke<Torrent | null>('inspect', { hash }),
  /** Change options, files or labels. File lists are indices into `files`; empty lists are ignored. */
  setTorrents: (hashes: string[], set: TorrentSet) => invoke<void>('set_torrents', { hashes, set }),
  /** Rename the torrent (`path` its current name) or a file or folder in it (`path` as in its files). */
  renameTorrent: (hash: string, path: string, name: string) => invoke<Renamed>('rename_torrent', { hash, path, name }),
  /** Every Transmission setting. */
  sessionSettings: () => invoke<Session>('session_settings'),
  /** Change settings; resolves to all of them as they are now. */
  setSession: (set: SessionSet) => invoke<Session>('set_session', { set }),
  /** The turtle and speed limits as of the last poll (then `session` events). */
  sessionBrief: () => invoke<Session | null>('session_brief'),
  testPort: () => invoke<PortTest>('test_port'),
  /** Fetch the blocklist now; resolves to how many rules it has. */
  updateBlocklist: () => invoke<number>('update_blocklist'),
  /** Add and take away labels, keeping each torrent's others. Resolves to how many changed. */
  editLabels: (hashes: string[], add: string[], remove: string[]) =>
    invoke<{ torrents: number }>('edit_labels', { hashes, add, remove }),
  /** Rename a label on every torrent (merging where `to` is there already). */
  renameLabel: (from: string, to: string) => invoke<{ torrents: number }>('rename_label', { from, to }),
  /** Take a label off every torrent. */
  deleteLabel: (label: string) => invoke<{ torrents: number }>('delete_label', { label }),
  /** Its name before it was renamed, when that's known and differs (else null). */
  originalName: (hash: string) => invoke<string | null>('original_name', { hash }),
  setLocation: (hashes: string[], location: string, moveData: boolean) =>
    invoke<void>('set_location', { hashes, location, moveData }),
  pathMappings: () => invoke<MappingView[]>('path_mappings'),
  setPathMappings: (list: PathMapping[]) => invoke<MappingView[]>('set_path_mappings', { list }),
  /** Open a torrent (or its file at index `file`) here, or reveal it, through the path mappings. */
  openTorrent: (hash: string, file: number | null, reveal: boolean) =>
    invoke<void>('open_torrent', { hash, file, reveal }),
};

export function onSession(handler: (s: Session) => void): Promise<UnlistenFn> {
  return listen<Session>('session', (e) => handler(e.payload));
}

export function onDetails(handler: (t: Torrent) => void): Promise<UnlistenFn> {
  return listen<Torrent>('details', (e) => handler(e.payload));
}

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
