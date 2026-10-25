// Which torrents are selected, by hash. The primary one is what the inspector
// shows; the rest are the selection's other members (for bulk actions).

import { createSignal } from 'solid-js';

import type { Torrent } from './api';

const [selectedHash, setSelectedHash] = createSignal<string | null>(null);
const [selectedHashes, setSelectedHashes] = createSignal<ReadonlySet<string>>(new Set());
export { selectedHash, selectedHashes };

/** Select just this one (null: nothing). */
export function select(hash: string | null): void {
  setSelectedHash(hash);
  setSelectedHashes(new Set(hash ? [hash] : []));
}

/** ⌘-click: add it, or take it out. */
export function toggleSelected(hash: string): void {
  const next = new Set(selectedHashes());
  if (next.has(hash)) {
    next.delete(hash);
    if (selectedHash() === hash) setSelectedHash(next.values().next().value ?? null);
  } else {
    next.add(hash);
    setSelectedHash(hash);
  }
  setSelectedHashes(next);
}

/** Shift-click: everything from the primary selection to this one, in `list`'s order. */
export function selectRange(hash: string, list: readonly Torrent[]): void {
  const from = list.findIndex((t) => t.hash_string === selectedHash());
  const to = list.findIndex((t) => t.hash_string === hash);
  if (from < 0 || to < 0) return select(hash);
  const [a, b] = from < to ? [from, to] : [to, from];
  setSelectedHashes(new Set(list.slice(a, b + 1).map((t) => t.hash_string!)));
}

/** Drop selected torrents that are gone (removed, or another host now). */
export function pruneSelection(exists: (hash: string) => boolean): void {
  const kept = [...selectedHashes()].filter(exists);
  if (kept.length !== selectedHashes().size) setSelectedHashes(new Set(kept));
  const primary = selectedHash();
  if (primary && !exists(primary)) setSelectedHash(kept[0] ?? null);
}
