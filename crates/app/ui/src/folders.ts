// Download folders on the host the add dialog offers: favourites (starred) and
// recently used ones. Per device, like the other view preferences. Paths are
// the host's, so there's no folder picker: they're typed or chosen from these.

import { createSignal } from 'solid-js';

import { storage } from './storage';

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
