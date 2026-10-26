// Adding torrents: what the add dialog holds (.torrent files and links), and
// the ways in: the Add button (⌘N), choosing files (⌘O), dropping files on
// the window, and pasting a magnet link. Each opens the dialog, or adds to it
// when it's already open, so the options are always seen before anything goes.

import { open as openFilePicker } from '@tauri-apps/plugin-dialog';
import { createEffect, createSignal, on, Show } from 'solid-js';

import { api, errorMessage, onInbox } from './api';
import { AddDialog } from './components/AddDialog';
import { isConnected } from './connection';
import { toast } from './toast';

const [dialogOpen, setDialogOpen] = createSignal(false);
/** Paths of .torrent files on this machine. */
const [files, setFiles] = createSignal<string[]>([]);
/** Magnet links and URLs, as typed or pasted: one per line. */
const [linksText, setLinksText] = createSignal('');
export { files, linksText, setFiles, setLinksText };

/** The magnet links and URLs in some text: whitespace-separated, in order, without repeats. */
export function linksIn(text: string): string[] {
  const found = text.split(/\s+/).filter((w) => /^magnet:\?/i.test(w) || /^https?:\/\/\S+$/i.test(w));
  return [...new Set(found)];
}

export const isTorrentFile = (path: string) => /\.torrent$/i.test(path);

/** Open the dialog with these (or add them to it when it's open already). */
export function openAdd(seed: { files?: string[]; links?: string[] } = {}): void {
  if (!isConnected()) {
    toast('Connect to Transmission first (Settings › Connection).', { error: true });
    return;
  }
  if (!dialogOpen()) {
    setFiles([]);
    setLinksText('');
  }
  if (seed.files?.length) setFiles((have) => [...have, ...seed.files!.filter((f) => !have.includes(f))]);
  if (seed.links?.length) {
    const have = linksIn(linksText());
    const fresh = seed.links.filter((l) => !have.includes(l));
    if (fresh.length) setLinksText((t) => [t.trim(), ...fresh].filter(Boolean).join('\n'));
  }
  setDialogOpen(true);
}

export function closeAdd(): void {
  setDialogOpen(false);
}

/** ⌘O, or the dialog's button: pick .torrent files on this machine. */
export async function chooseTorrentFiles(): Promise<void> {
  const picked = await openFilePicker({
    multiple: true,
    title: 'Add Torrents',
    filters: [{ name: 'Torrent files', extensions: ['torrent'] }],
  });
  const paths = Array.isArray(picked) ? picked : picked ? [picked] : [];
  if (paths.length) openAdd({ files: paths });
}

/**
 * ⌘V outside a text field: magnet links or URLs on the clipboard open the
 * dialog with them. Returns the cleanup.
 */
export function watchPaste(): () => void {
  const onPaste = (e: ClipboardEvent) => {
    const target = e.target instanceof HTMLElement ? e.target : null;
    if (target?.closest('input, textarea, [contenteditable], dialog')) return;
    const links = linksIn(e.clipboardData?.getData('text') ?? '');
    if (!links.length) return;
    e.preventDefault();
    openAdd({ links });
  };
  document.addEventListener('paste', onPaste);
  return () => document.removeEventListener('paste', onPaste);
}

/**
 * Magnet links and .torrent files opened from outside (a link clicked in the
 * browser, a file opened in Finder or the file manager, the tray's Add from
 * Clipboard). They wait in the backend until connected, then open the dialog.
 * Returns the cleanup.
 */
export function watchInbox(): () => void {
  let warned = false;
  const take = async () => {
    if (!isConnected()) {
      if (!warned) toast('Connect to Transmission to add this torrent. It’ll be waiting.', { error: true });
      warned = true;
      return;
    }
    try {
      const { links, files } = await api.takeInbox();
      if (links.length || files.length) openAdd({ links, files });
    } catch (err) {
      toast(errorMessage(err), { error: true });
    }
  };
  createEffect(
    on(isConnected, (connected) => {
      if (connected) {
        warned = false;
        void take();
      }
    }),
  );
  const unlisten = onInbox(() => void take());
  return () => void unlisten.then((off) => off());
}

/** Mounted once in App. */
export function AddDialogHost() {
  return (
    <Show when={dialogOpen()}>
      <AddDialog onClose={closeAdd} />
    </Show>
  );
}
