import { createEffect, createSignal, onCleanup, onMount, Show } from 'solid-js';

import { chosen, openHere, openTorrentMenu, remove, renameTorrent, setLocation, toggleRunning } from './actions';
import { TorrentDialogsHost } from './components/TorrentDialogs';
import { watchInspector } from './inspector';
import { AddDialogHost, chooseTorrentFiles, openAdd, watchInbox, watchPaste } from './adder';
import { onNotice, onOpenSettings, onZoomMenu } from './api';
import { navOrder } from './browse';
import { ConfirmHost } from './confirm';
import { isConnected, session, setTurtle, watchConnection } from './connection';
import { DropOverlay } from './components/DropOverlay';
import { Inspector } from './components/Inspector';
import { TorrentMenuHost } from './components/TorrentMenu';
import { SettingsDialog, type SettingsSection } from './components/settings/Settings';
import { Sidebar } from './components/Sidebar';
import { StatusBar } from './components/StatusBar';
import { TopBar } from './components/TopBar';
import { tableView, TorrentArea } from './components/TorrentArea';
import { QuestionHost } from './question';
import { pruneSelection, select, selectAll, selectedHash, selectedHashes, selectRange } from './selection';
import { Toast, toast } from './toast';
import { watchOriginals } from './originals';
import { scheduleUpdateChecks } from './updates';
import { torrent, watchTorrents } from './torrents';
import { inspectorOpen, setInspectorOpen, setSidebarOpen, sidebarOpen } from './view';
import { resetZoom, stepZoom } from './zoom';

export function App() {
  let searchInput: HTMLInputElement | undefined;
  const [settingsOpen, setSettingsOpen] = createSignal(false);
  const [settingsSection, setSettingsSection] = createSignal<SettingsSection>();
  const openSettings = (section?: SettingsSection) => {
    setSettingsSection(section);
    setSettingsOpen(true);
  };

  const onKeyDown = (e: KeyboardEvent) => {
    // Key events normally target an element; fall back to <body> for anything else.
    const target = e.target instanceof HTMLElement ? e.target : document.body;
    const inField = target instanceof HTMLInputElement || target instanceof HTMLSelectElement;
    const cmd = e.metaKey || e.ctrlKey;

    // ⌘, opens settings (macOS convention). While it's open, its own controls own the keyboard.
    if (cmd && e.key === ',') {
      e.preventDefault();
      openSettings();
      return;
    }
    // ⌘+ / ⌘− / ⌘0: interface size, anywhere. On macOS the View menu owns ⌘=, ⌘− and ⌘0; ⌘⇧= (⌘+) comes here.
    const mac = document.documentElement.classList.contains('is-mac');
    if (cmd && !e.altKey && (e.key === '+' || (!mac && (e.key === '=' || e.key === '-' || e.key === '0')))) {
      e.preventDefault();
      if (e.key === '0') resetZoom();
      else stepZoom(e.key === '-' ? -1 : 1);
      return;
    }
    if (target.closest('dialog')) return;

    if ((e.key === '/' && !inField) || (e.key === 'f' && cmd)) {
      e.preventDefault();
      searchInput?.focus();
      searchInput?.select();
      return;
    }
    // ⌥⌘S: the sidebar, as in Finder. `code`, since ⌥ changes the key on macOS (ß).
    if (cmd && e.altKey && e.code === 'KeyS') {
      e.preventDefault();
      setSidebarOpen(!sidebarOpen());
      return;
    }
    // ⇧⌘T: the turtle. `code`, since Shift changes the key.
    if (cmd && e.shiftKey && e.code === 'KeyT' && session()) {
      e.preventDefault();
      void setTurtle(!session()!.alt_speed_enabled);
      return;
    }
    if (cmd && !e.altKey && e.key === 'i') {
      e.preventDefault();
      setInspectorOpen(!inspectorOpen());
      return;
    }
    // ⌘N: the add dialog; ⌘O: straight to choosing .torrent files.
    if (cmd && !e.altKey && (e.key === 'n' || e.key === 'o') && isConnected()) {
      e.preventDefault();
      if (e.key === 'n') openAdd();
      else void chooseTorrentFiles();
      return;
    }
    if (inField || target.closest('textarea, [role=menu]')) return;
    if (!isConnected()) return;
    torrentKeys(e, target);
  };

  /** Where Shift+↑/↓ has extended the selection to (the primary selection stays the anchor). */
  let cursor: string | null = null;

  /** The table's keys: moving and extending the selection, and acting on it. */
  const torrentKeys = (e: KeyboardEvent, target: HTMLElement) => {
    const cmd = e.metaKey || e.ctrlKey;
    const list = navOrder();
    const hashes = chosen();

    if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      if (!list.length) return;
      e.preventDefault();
      const step = e.key === 'ArrowUp' ? -1 : 1;
      const from = e.shiftKey && cursor ? cursor : selectedHash();
      const at = list.findIndex((t) => t.hash_string === from);
      const next = list[at < 0 ? 0 : Math.min(list.length - 1, Math.max(0, at + step))]!.hash_string!;
      if (e.shiftKey && selectedHash()) selectRange(next, list);
      else select(next);
      cursor = next;
      tableView.current?.reveal(next);
      return;
    }
    cursor = null;
    if (cmd && e.key === 'a') {
      e.preventDefault();
      selectAll(list);
    } else if (e.key === 'Escape') {
      // First back to one torrent, then nothing.
      if (selectedHashes().size > 1) select(selectedHash());
      else select(null);
    } else if (!hashes.length) {
      return;
    } else if (e.key === 'Enter') {
      if (target.closest('button, a')) return;
      e.preventDefault();
      setInspectorOpen(true);
    } else if (e.key === ' ') {
      if (target.closest('button, a')) return;
      e.preventDefault();
      void toggleRunning();
    } else if ((e.key === 'Backspace' && cmd) || e.key === 'Delete') {
      e.preventDefault();
      void remove(e.shiftKey);
    } else if (e.key === 'F2') {
      e.preventDefault();
      renameTorrent(hashes[0]);
    } else if (cmd && e.key === 'l') {
      e.preventDefault();
      setLocation(hashes);
    } else if (cmd && (e.key === 'r' || e.key === 'ArrowDown')) {
      // ⌘R reveals, ⌘↓ opens (as in Finder).
      e.preventDefault();
      void openHere(hashes[0]!, null, e.key === 'r');
    } else if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) {
      // The keyboard's menu key: open it beside the selected row.
      e.preventDefault();
      tableView.current?.reveal(hashes[0]!);
      const r = document.querySelector(`[data-hash="${hashes[0]}"]`)?.getBoundingClientRect();
      if (r) openTorrentMenu(r.left + 48, r.top + Math.min(r.height, 24));
    }
  };

  onMount(() => {
    document.addEventListener('keydown', onKeyDown);
    // On macOS the menu bar item owns ⌘, (so onKeyDown's ⌘, branch only runs on other platforms).
    const unlisten = onOpenSettings(() => openSettings());
    onCleanup(() => void unlisten.then((off) => off()));
    const unlistenZoom = onZoomMenu((action) =>
      action === 'reset' ? resetZoom() : stepZoom(action === 'in' ? 1 : -1),
    );
    onCleanup(() => void unlistenZoom.then((off) => off()));
    // The poller's reports: the connection, the speeds, and the torrents.
    const stopConnection = watchConnection();
    const stopTorrents = watchTorrents();
    onCleanup(() => {
      void stopConnection.then((off) => off());
      void stopTorrents.then((off) => off());
    });
    // ⌘V with a magnet link (outside a text field) opens the add dialog with it.
    onCleanup(watchPaste());
    // Links and files opened from outside the window.
    onCleanup(watchInbox());
    // Messages from the menu bar / tray.
    const unlistenNotice = onNotice((message) => toast(message, { error: true }));
    onCleanup(() => void unlistenNotice.then((off) => off()));
    // The inspector's torrent, in full, while it shows one.
    watchInspector();
    // Original names, for two-line rows.
    watchOriginals();
    // A newer version, soon after launch and once a day.
    scheduleUpdateChecks();
  });

  // Selected torrents that are gone (removed, or another host) stop being selected.
  createEffect(() => pruneSelection((hash) => torrent(hash) !== undefined));
  onCleanup(() => document.removeEventListener('keydown', onKeyDown));

  return (
    <>
      <TopBar searchRef={(el) => (searchInput = el)} onOpenSettings={openSettings} />
      <main class="fixed inset-x-0 top-(--topbar-h) bottom-(--statusbar-h) flex">
        <Show when={sidebarOpen()}>
          <Sidebar />
        </Show>
        <section class="relative flex min-w-0 flex-1 flex-col" aria-label="Torrents">
          <TorrentArea
            onOpenSettings={() => openSettings('connection')}
            onOpenInspector={() => setInspectorOpen(true)}
          />
        </section>
        <Show when={inspectorOpen()}>
          <Inspector onClose={() => setInspectorOpen(false)} />
        </Show>
      </main>
      <StatusBar />
      <Show when={settingsOpen()}>
        <SettingsDialog section={settingsSection()} onClose={() => setSettingsOpen(false)} />
      </Show>
      <AddDialogHost />
      <DropOverlay />
      <TorrentMenuHost />
      <TorrentDialogsHost />
      <ConfirmHost />
      <QuestionHost />
      <Toast />
    </>
  );
}
