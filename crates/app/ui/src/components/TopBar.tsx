import { getCurrentWindow } from '@tauri-apps/api/window';
import { onCleanup, onMount } from 'solid-js';
import PanelLeft from 'lucide-solid/icons/panel-left';
import PanelRight from 'lucide-solid/icons/panel-right';
import Pause from 'lucide-solid/icons/pause';
import Play from 'lucide-solid/icons/play';
import Plus from 'lucide-solid/icons/plus';
import Search from 'lucide-solid/icons/search';
import Settings from 'lucide-solid/icons/settings';
import Trash from 'lucide-solid/icons/trash';
import X from 'lucide-solid/icons/x';

import { act, chosen, isRunning, remove } from '../actions';
import { openAdd } from '../adder';
import { isConnected } from '../connection';
import { deleteKeys } from '../keys';
import { torrent } from '../torrents';
import type { SettingsSection } from './settings/Settings';
import { inspectorOpen, query, setInspectorOpen, setQuery, setSidebarOpen, sidebarOpen } from '../view';
import { ThemeMenu } from './ThemeMenu';
import { Icon } from './Icon';
import { keys } from '../keys';

const iconButton =
  'grid size-8 flex-none place-items-center rounded-button text-muted hover:bg-hover hover:text-fg aria-pressed:text-fg disabled:opacity-40 disabled:hover:bg-transparent';

/** Searches names and labels as you type (tens of torrents: no need to wait for a pause). */
function SearchField(props: Pick<Props, 'searchRef'>) {
  const text = query;
  const setText = setQuery;
  return (
    <div class="relative flex w-full max-w-[460px] items-center justify-self-center" data-tauri-drag-region>
      <Icon of={Search} class="pointer-events-none absolute left-2.5 text-faint" />
      <input
        ref={props.searchRef}
        type="search"
        placeholder="Search names and labels"
        autocomplete="off"
        spellcheck={false}
        value={text()}
        class="peer h-8 w-full rounded-button border border-line bg-field pr-[30px] pl-8 transition-[border-color,box-shadow] duration-150 outline-none select-text focus:border-focus focus:ring-3 focus:ring-focus/20 [&::-webkit-search-cancel-button]:hidden"
        onInput={(e) => setText(e.currentTarget.value)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            e.stopPropagation();
            if (e.currentTarget.value) setText('');
            else e.currentTarget.blur();
          }
        }}
      />
      <kbd class="pointer-events-none absolute right-2 rounded-inset border border-line px-1.5 font-sans text-[11px]/4 text-faint peer-focus:hidden peer-[:not(:placeholder-shown)]:hidden">
        /
      </kbd>
      {text() && (
        <button
          class="absolute right-2 grid size-5 place-items-center rounded-inset text-muted hover:bg-hover hover:text-fg"
          aria-label="Clear search"
          onClick={() => setText('')}
        >
          <Icon of={X} size={12} />
        </button>
      )}
    </div>
  );
}

/** Whether a pointer event is on an open dialog's backdrop, over the toolbar (`bar`). */
function onBackdropOverBar(e: MouseEvent, bar: HTMLElement): boolean {
  if (!(e.target instanceof HTMLDialogElement) || !e.target.open) return false;
  // The backdrop's clicks target the dialog itself; make sure it's outside the dialog's box.
  const box = e.target.getBoundingClientRect();
  const inside = e.clientX >= box.left && e.clientX <= box.right && e.clientY >= box.top && e.clientY <= box.bottom;
  return !inside && e.clientY < bar.getBoundingClientRect().bottom;
}

/**
 * macOS: the toolbar is the window's title bar (dragging or double-clicking
 * its empty space moves or zooms the window), so a modal dialog's backdrop
 * over it does the same, and macOS hides the traffic lights in full screen,
 * so their room goes too (data-fullscreen, styles.css).
 */
function useMacTitleBar(bar: () => HTMLElement) {
  onMount(() => {
    if (!document.documentElement.classList.contains('is-mac')) return;
    // A modal dialog covers the page with its backdrop, so the drag region never
    // sees presses there; move or zoom the window from the backdrop instead, and
    // keep that press from counting as a click on the backdrop (which closes
    // some dialogs).
    const press = (e: MouseEvent) => {
      if (e.button !== 0 || !onBackdropOverBar(e, bar())) return;
      e.preventDefault();
      e.stopPropagation();
      const win = getCurrentWindow();
      void (e.detail === 2 ? win.toggleMaximize() : win.startDragging());
    };
    const click = (e: MouseEvent) => {
      if (onBackdropOverBar(e, bar())) e.stopPropagation();
    };
    addEventListener('mousedown', press, true);
    addEventListener('click', click, true);
    onCleanup(() => {
      removeEventListener('mousedown', press, true);
      removeEventListener('click', click, true);
    });

    try {
      const win = getCurrentWindow();
      const sync = () =>
        win.isFullscreen().then(
          (full) => document.documentElement.toggleAttribute('data-fullscreen', full),
          () => {},
        );
      void sync();
      const unlisten = win.onResized(() => void sync());
      onCleanup(() => void unlisten.then((off) => off()));
    } catch {
      // Not running in Tauri.
    }
  });
}

type Props = {
  searchRef?: (el: HTMLInputElement) => void;
  /** Opens Settings, at `section` if given. */
  onOpenSettings(section?: SettingsSection): void;
};

/**
 * The toolbar, one row: the sidebar toggle, Add, and Resume / Pause / Remove for
 * the selection (beside the traffic lights on macOS), search in the middle, then
 * the inspector toggle, theme and
 * settings. Its empty space drags the window (data-tauri-drag-region, on each
 * element that's empty space, since it doesn't pass to children).
 */
export function TopBar(props: Props) {
  let bar!: HTMLElement;
  useMacTitleBar(() => bar);
  const selection = () => chosen().map((h) => torrent(h));

  return (
    <header
      ref={bar}
      class="fixed inset-x-0 top-0 z-10 grid h-(--topbar-h) grid-cols-[minmax(max-content,1fr)_minmax(160px,2fr)_minmax(max-content,1fr)] items-center gap-4 border-b border-line bg-bg/80 pr-2 pl-[calc(var(--lights-w)+12px)] backdrop-blur-[18px] backdrop-saturate-[1.6]"
      data-tauri-drag-region
    >
      <div class="flex items-center gap-1 justify-self-start" data-tauri-drag-region>
        <button
          class={iconButton}
          aria-pressed={sidebarOpen()}
          title={`${sidebarOpen() ? 'Hide' : 'Show'} sidebar (${keys('mod+alt+S')})`}
          aria-label="Sidebar"
          onClick={() => setSidebarOpen(!sidebarOpen())}
        >
          <Icon of={PanelLeft} size={16} />
        </button>
        <button
          class={iconButton}
          title={`Add torrents (${keys('mod+N')})`}
          aria-label="Add torrents"
          disabled={!isConnected()}
          onClick={() => openAdd()}
        >
          <Icon of={Plus} size={16} />
        </button>
        <span class="mx-1 h-4 w-px bg-line" aria-hidden="true" />
        {/* What to do with the selection. */}
        <button
          class={iconButton}
          title={`Resume (${keys('Space')})`}
          aria-label="Resume"
          disabled={!selection().some((t) => t?.status === 0)}
          onClick={() => void act('start')}
        >
          <Icon of={Play} size={16} />
        </button>
        <button
          class={iconButton}
          title={`Pause (${keys('Space')})`}
          aria-label="Pause"
          disabled={!selection().some(isRunning)}
          onClick={() => void act('stop')}
        >
          <Icon of={Pause} size={16} />
        </button>
        <button
          class={iconButton}
          title={`Remove (${keys(deleteKeys)}; with ${keys('shift')} to delete files too)`}
          aria-label="Remove"
          disabled={!selection().length}
          onClick={(e) => void remove(e.shiftKey)}
        >
          <Icon of={Trash} size={16} />
        </button>
      </div>
      <SearchField searchRef={props.searchRef} />
      <div class="flex items-center gap-1 justify-self-end" data-tauri-drag-region>
        <button
          class={iconButton}
          aria-pressed={inspectorOpen()}
          title={`${inspectorOpen() ? 'Hide' : 'Show'} inspector (${keys('mod+I')})`}
          aria-label="Inspector"
          onClick={() => setInspectorOpen(!inspectorOpen())}
        >
          <Icon of={PanelRight} size={16} />
        </button>
        <ThemeMenu />
        <button
          class={iconButton}
          title={`Settings (${keys('mod+,')})`}
          aria-label="Settings"
          onClick={() => props.onOpenSettings()}
        >
          <Icon of={Settings} size={16} />
        </button>
      </div>
    </header>
  );
}
