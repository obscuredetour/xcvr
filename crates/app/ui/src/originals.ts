// Original names of renamed torrents, for the second line of expanded rows and
// the Original Name column. Fetched while either shows them, and again when a
// torrent is added, removed or renamed (the backend reads each host .torrent
// copy once, so it's cheap).

import { createEffect, createSignal, on } from 'solid-js';

import { api } from './api';
import { isConnected } from './connection';
import { torrents } from './torrents';
import { effectiveColumnVisibility, secondLine, tallRows } from './view';

const [names, setNames] = createSignal<Record<string, string>>({});

/** The torrent's name before it was renamed, when known. */
export const originalName = (hash: string | undefined): string | undefined => (hash ? names()[hash] : undefined);

/** Mounted once in App. */
export function watchOriginals(): void {
  // What the answer depends on: each torrent and its current name.
  const shown = () => (tallRows() && secondLine() === 'original') || effectiveColumnVisibility()['original'] === true;
  const key = () =>
    shown() && isConnected()
      ? torrents()
          .map((t) => `${t.hash_string}:${t.name}`)
          .join('\n')
      : null;
  createEffect(
    on(key, (k) => {
      if (k === null) return;
      api
        .originalNames()
        .then(setNames)
        .catch(() => {});
    }),
  );
}
