import { createEffect, createSignal, onCleanup, onMount, Show } from 'solid-js';

import { onOpenSettings, onZoomMenu } from './api';
import { ConfirmHost } from './confirm';
import { watchConnection } from './connection';
import { Inspector } from './components/Inspector';
import { SettingsDialog, type SettingsSection } from './components/settings/Settings';
import { Sidebar } from './components/Sidebar';
import { StatusBar } from './components/StatusBar';
import { TopBar } from './components/TopBar';
import { TorrentArea } from './components/TorrentArea';
import { QuestionHost } from './question';
import { pruneSelection } from './selection';
import { Toast } from './toast';
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
    if (cmd && !e.altKey && e.key === 'i') {
      e.preventDefault();
      setInspectorOpen(!inspectorOpen());
      return;
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
      <ConfirmHost />
      <QuestionHost />
      <Toast />
    </>
  );
}
