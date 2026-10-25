// What can be done to torrents, from anywhere: the toolbar, the right-click
// menu and the keyboard all come here. Failures show as a toast; the poller
// polls straight after each one, so the table catches up at once.

import { createSignal } from 'solid-js';

import { api, errorMessage, type Torrent, type TorrentAction } from './api';
import { confirm } from './confirm';
import { selectedHash, selectedHashes } from './selection';
import { toast } from './toast';
import { torrent } from './torrents';

/** The selection's hashes, the primary one first (empty when nothing is selected). */
export function chosen(): string[] {
  const primary = selectedHash();
  const rest = [...selectedHashes()].filter((h) => h !== primary);
  return primary ? [primary, ...rest] : rest;
}

/** Torrents that are running or waiting their turn: pausing applies to them. */
export const isRunning = (t: Torrent | undefined) => !!t && t.status !== undefined && t.status !== 0;

export async function act(action: TorrentAction, hashes: string[] = chosen()): Promise<void> {
  if (!hashes.length) return;
  try {
    await api.torrentAction(action, hashes);
  } catch (err) {
    toast(errorMessage(err), { error: true });
  }
}

/** Pause what's running if any of them are, else start them all (Space). */
export function toggleRunning(hashes: string[] = chosen()): Promise<void> {
  const anyRunning = hashes.some((h) => isRunning(torrent(h)));
  return act(anyRunning ? 'stop' : 'start', hashes);
}

const nameOf = (hashes: string[]) =>
  hashes.length === 1 ? `“${torrent(hashes[0]!)?.name ?? 'this torrent'}”` : `${hashes.length} torrents`;

/** Remove torrents, after asking; `withData` deletes their downloaded files on the host too. */
export async function remove(withData: boolean, hashes: string[] = chosen()): Promise<void> {
  if (!hashes.length) return;
  const many = hashes.length > 1;
  const ok = await confirm({
    title: withData ? `Remove and delete ${many ? 'their' : 'its'} files?` : `Remove ${nameOf(hashes)}?`,
    message: () =>
      withData ? (
        <p>
          {nameOf(hashes)} {many ? 'are' : 'is'} removed from Transmission, and{' '}
          {many ? 'their downloaded files are' : 'its downloaded files are'} deleted from the host. This can’t be
          undone.
        </p>
      ) : (
        <p>{many ? 'They’re' : 'It’s'} removed from Transmission. Downloaded files stay on the host.</p>
      ),
    confirm: withData ? 'Remove and Delete Files' : many ? `Remove ${hashes.length} Torrents` : 'Remove',
    danger: true,
  });
  if (!ok) return;
  try {
    await api.removeTorrents(hashes, withData);
  } catch (err) {
    toast(errorMessage(err), { error: true });
  }
}

/** Copy the magnet links of torrents, one per line. */
export async function copyMagnets(hashes: string[] = chosen()): Promise<void> {
  if (!hashes.length) return;
  try {
    const links = await api.magnetLinks(hashes);
    await navigator.clipboard.writeText(links.join('\n'));
    toast(links.length === 1 ? 'Magnet link copied' : `${links.length} magnet links copied`);
  } catch (err) {
    toast(errorMessage(err), { error: true });
  }
}

/** The right-click menu: where, and for which torrents. Shown by <TorrentMenuHost/>. */
const [menu, setMenu] = createSignal<{ x: number; y: number; hashes: string[] } | null>(null);
export { menu as torrentMenu };

export function openTorrentMenu(x: number, y: number, hashes: string[] = chosen()): void {
  if (hashes.length) setMenu({ x, y, hashes });
}

export function closeTorrentMenu(): void {
  setMenu(null);
}
