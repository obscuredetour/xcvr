// Every torrent, keyed by info hash, patched from the poller's diffs. Fine-grained:
// a diff that changes one torrent's speed updates only what shows that speed.

import { batch } from 'solid-js';
import { createStore, produce, reconcile } from 'solid-js/store';

import { api, type Diff, onTorrents, type Torrent } from './api';
import { toast } from './toast';

const [store, setStore] = createStore<{ byHash: Record<string, Torrent>; order: string[] }>({ byHash: {}, order: [] });

/** Every torrent, in Transmission's order. */
export const torrents = (): Torrent[] => store.order.map((h) => store.byHash[h]!).filter(Boolean);

export const torrentCount = () => store.order.length;

export function torrent(hash: string): Torrent | undefined {
  return store.byHash[hash];
}

export function applyDiff(diff: Diff): void {
  batch(() => {
    if (diff.reset) {
      const byHash = Object.fromEntries(diff.added.map((t) => [t.hash_string!, t]));
      setStore('byHash', reconcile(byHash));
      setStore(
        'order',
        diff.added.map((t) => t.hash_string!),
      );
      return;
    }
    if (diff.added.length) {
      for (const t of diff.added) setStore('byHash', t.hash_string!, t);
      setStore('order', (order) => [...order, ...diff.added.map((t) => t.hash_string!)]);
    }
    // A change carries only what changed; merging keeps the rest.
    for (const change of diff.changed) setStore('byHash', change.hash_string!, change);
    if (diff.removed.length) {
      const gone = new Set(diff.removed);
      setStore('order', (order) => order.filter((h) => !gone.has(h)));
      setStore(
        'byHash',
        produce((byHash) => {
          for (const h of gone) delete byHash[h];
        }),
      );
    }
  });
  // In-window for now; native notifications come with the tray (phase 9).
  for (const hash of diff.completed) toast(`Finished: ${store.byHash[hash]?.name ?? 'a torrent'}`);
}

/** Follow the poller: diffs first, then the current list, so nothing is missed in between. */
export async function watchTorrents(): Promise<() => void> {
  const off = await onTorrents(applyDiff);
  const list = await api.torrents();
  applyDiff({ reset: true, added: list, changed: [], removed: [], completed: [] });
  return off;
}
