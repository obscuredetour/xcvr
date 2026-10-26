// Download folders on the host: the ones the add dialog offers (favourites,
// starred, and recently used ones; per device, like the other view
// preferences), and the ones torrents are in, for the sidebar's Folders. Paths
// are the host's, so there's no folder picker: they're typed or chosen.

import { createMemo, createRoot, createSignal } from 'solid-js';

import type { Torrent } from './api';
import { session } from './connection';
import { storage } from './storage';
import { torrents } from './torrents';

const MAX_RECENT = 8;

function storedList(key: string): string[] {
  try {
    const parsed: unknown = JSON.parse(storage.get(key) ?? '[]');
    return Array.isArray(parsed) ? parsed.filter((x): x is string => typeof x === 'string') : [];
  } catch {
    return [];
  }
}

/** Trailing slashes off, so "/data/" and "/data" are one folder. */
export const normalizeFolder = (dir: string) => dir.trim().replace(/(.)\/+$/, '$1');

const [recentFolders, setRecent] = createSignal(storedList('recentFolders'));
const [favouriteFolders, setFavourites] = createSignal(storedList('favouriteFolders'));
export { favouriteFolders, recentFolders };

/** A folder was just used: it goes to the top of the recents. */
export function rememberFolder(dir: string): void {
  const d = normalizeFolder(dir);
  if (!d) return;
  const next = [d, ...recentFolders().filter((x) => x !== d)].slice(0, MAX_RECENT);
  setRecent(next);
  storage.set('recentFolders', JSON.stringify(next));
}

export const isFavourite = (dir: string) => favouriteFolders().includes(normalizeFolder(dir));

export function toggleFavourite(dir: string): void {
  const d = normalizeFolder(dir);
  if (!d) return;
  const next = isFavourite(d) ? favouriteFolders().filter((x) => x !== d) : [...favouriteFolders(), d].sort();
  setFavourites(next);
  storage.set('favouriteFolders', JSON.stringify(next));
}

// ---------- Where torrents are (the sidebar's Folders) ----------

/** A torrent's download folder: the folder its files are in. */
export const folderOf = (t: Torrent) => normalizeFolder(t.download_dir ?? '');

/** Transmission's default download folder, when known. */
const defaultFolder = () => {
  const dir = session()?.download_dir;
  return dir ? normalizeFolder(dir) : null;
};

/**
 * A folder as the sidebar names it: relative to the default download folder
 * ("isos" for /downloads/isos), the default folder by its own name, and any
 * other folder by its full path.
 */
export function folderName(path: string): string {
  const root = defaultFolder();
  if (root && path === root) return path.split('/').filter(Boolean).at(-1) ?? path;
  if (root && path.startsWith(`${root === '/' ? '' : root}/`)) return path.slice(root === '/' ? 1 : root.length + 1);
  return path || '(no folder)';
}

/** Whether it's the default download folder. */
export const isDefaultFolder = (path: string) => path === defaultFolder();

/**
 * Every folder that has torrents, with how many: the default folder first, then
 * its subfolders, then any others, each A to Z by name.
 */
export const allFolders = createRoot(() =>
  createMemo((): { path: string; name: string; count: number }[] => {
    const counts = new Map<string, number>();
    for (const t of torrents()) counts.set(folderOf(t), (counts.get(folderOf(t)) ?? 0) + 1);
    const rank = (path: string) => (isDefaultFolder(path) ? 0 : folderName(path).startsWith('/') ? 2 : 1);
    return [...counts]
      .map(([path, count]) => ({ path, name: folderName(path), count }))
      .sort(
        (a, b) =>
          rank(a.path) - rank(b.path) ||
          a.name.localeCompare(b.name, undefined, { sensitivity: 'base', numeric: true }),
      );
  }),
);
