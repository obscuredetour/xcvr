// The torrent the inspector shows, in full (files, peers, trackers, options):
// fetched when it opens on one, then kept current by the poller's `details`
// events. Reconciled into a store, so a new peer list or a file's progress
// updates in place.

import { createEffect, createRoot, createSignal, on, onCleanup } from 'solid-js';
import { createStore, reconcile } from 'solid-js/store';

import { api, errorMessage, onDetails, type Torrent } from './api';
import { isConnected } from './connection';
import { selectedHash, selectedHashes } from './selection';
import { storage } from './storage';
import { toast } from './toast';
import { inspectorOpen } from './view';

const [store, setStore] = createStore<{ t: Torrent | null }>({ t: null });
/** The followed torrent's details, or null while none is shown (or they're loading). */
export const details = () => store.t;

/** The hash the inspector follows: the selection, when it's one torrent and the inspector is open. */
export const inspectedHash = () =>
  inspectorOpen() && isConnected() && selectedHashes().size === 1 ? selectedHash() : null;

function show(t: Torrent | null): void {
  setStore('t', t ? reconcile(t, { merge: true }) : null);
}

/** Follow the selection while the inspector shows it. Call once (App). */
export function watchInspector(): void {
  createRoot(() => {
    let off: (() => void) | undefined;
    void onDetails((t) => {
      if (t.hash_string === inspectedHash()) show(t);
    }).then((u) => (off = u));
    onCleanup(() => off?.());

    createEffect(
      on(inspectedHash, (hash) => {
        if (details()?.hash_string !== hash) show(null);
        api.inspect(hash).then(
          (t) => {
            if (hash && inspectedHash() === hash) show(t);
          },
          (err: unknown) => {
            if (hash) toast(errorMessage(err), { error: true });
          },
        );
      }),
    );
  });
}

export type InspectorTab = 'general' | 'files' | 'peers' | 'trackers' | 'options';
const TABS: readonly InspectorTab[] = ['general', 'files', 'peers', 'trackers', 'options'];

const storedTab = storage.get('inspectorTab') as InspectorTab | null;
const [tab, setTabSignal] = createSignal<InspectorTab>(storedTab && TABS.includes(storedTab) ? storedTab : 'general');
export { tab as inspectorTab };

export function setInspectorTab(t: InspectorTab): void {
  setTabSignal(t);
  storage.set('inspectorTab', t);
}
