import { For, Match, Show, Switch } from 'solid-js';
import Ellipsis from 'lucide-solid/icons/ellipsis';
import FolderOpen from 'lucide-solid/icons/folder-open';
import SquareArrowOutUpRight from 'lucide-solid/icons/square-arrow-out-up-right';
import X from 'lucide-solid/icons/x';

import { act, copyMagnets, openHere, remove, renameTorrent, setLocation } from '../actions';
import { smallButton } from '../buttons';
import { formatEta, formatPercent, formatSize, progressOf, statusLabel, statusTone } from '../format';
import { details, type InspectorTab, inspectorTab, setInspectorTab } from '../inspector';
import { keys } from '../keys';
import { selectedHash, selectedHashes } from '../selection';
import { torrent } from '../torrents';
import { INSPECTOR_WIDTH, inspectorWidth, setInspectorWidth } from '../view';
import { widthFlags } from '../widths';
import { Icon } from './Icon';
import { FilesTab } from './inspector/Files';
import { GeneralTab } from './inspector/General';
import { OptionsTab } from './inspector/Options';
import { PeersTab } from './inspector/Peers';
import { TrackersTab } from './inspector/Trackers';
import { Menu, MenuItem, MenuSeparator } from './Menu';

/**
 * The right sidebar: the selected torrent in full, in tabs (General, Files,
 * Peers, Trackers, Options), with Open and Reveal. Resizable from its left
 * edge, like Magnitide's details panel. Several selected: what can be done to
 * them together.
 */
export function Inspector(props: { onClose(): void }) {
  return (
    <aside
      ref={(el) => widthFlags(el, { wide: 416 })}
      class="group/inspector relative flex flex-none animate-slide-in flex-col border-l border-line bg-panel select-text max-[760px]:absolute max-[760px]:inset-y-0 max-[760px]:right-0 max-[760px]:w-[min(28rem,100%-2rem)]! max-[760px]:max-w-full! max-[760px]:shadow-lift"
      // The table keeps at least 320px beside it.
      style={{ width: `${inspectorWidth()}px`, 'max-width': 'calc(100% - 320px)' }}
      aria-label="Inspector"
    >
      <ResizeHandle />
      <button
        class="absolute top-2.5 right-2.5 z-1 grid size-7 place-items-center rounded-full hover:bg-hover"
        title={`Close (${keys('mod+I')})`}
        aria-label="Close inspector"
        onClick={() => props.onClose()}
      >
        <Icon of={X} class="text-muted" />
      </button>
      <Switch fallback={<Empty text="Select a torrent" />}>
        <Match when={selectedHashes().size > 1}>
          <Several count={selectedHashes().size} />
        </Match>
        <Match when={selectedHash() && torrent(selectedHash()!)}>
          <Show when={details()} fallback={<Empty text="Loading…" />}>
            <One />
          </Show>
        </Match>
      </Switch>
    </aside>
  );
}

function Empty(props: { text: string }) {
  return <div class="grid flex-1 place-items-center p-6 text-center text-sm text-muted">{props.text}</div>;
}

const TABS: { id: InspectorTab; label: string }[] = [
  { id: 'general', label: 'General' },
  { id: 'files', label: 'Files' },
  { id: 'peers', label: 'Peers' },
  { id: 'trackers', label: 'Trackers' },
  { id: 'options', label: 'Options' },
];

const TONE_BG = { download: 'bg-down', seed: 'bg-up', idle: 'bg-faint', check: 'bg-info', error: 'bg-danger' };

/** One torrent: who it is, how far along, the tabs. */
function One() {
  const t = () => details()!;
  const hash = () => t().hash_string!;
  const count = (id: InspectorTab) =>
    id === 'files'
      ? t().files?.length
      : id === 'peers'
        ? t().peers?.length
        : id === 'trackers'
          ? t().tracker_stats?.length
          : undefined;

  return (
    <>
      <header class="border-b border-line px-5 pt-4 pb-3">
        <h2 class="pr-8 text-[15px]/snug font-semibold break-words">{t().name}</h2>
        <div class="mt-1 flex items-center gap-1.5 text-xs text-muted">
          <span class={`size-1.5 rounded-full ${TONE_BG[statusTone(t())]}`} aria-hidden="true" />
          {statusLabel(t())} · {formatPercent(progressOf(t()))} of {formatSize(t().size_when_done ?? 0)}
          <Show when={formatEta(t())}>{(eta) => <> · {eta()} left</>}</Show>
        </div>
        <Show when={(t().error ?? 0) > 0 && t().error_string}>
          {(msg) => (
            <p
              class="mt-1.5 text-xs"
              classList={{ 'text-warning': t().error === 1, 'text-danger': (t().error ?? 0) >= 2 }}
            >
              {msg()}
            </p>
          )}
        </Show>
        <div class="mt-3 flex flex-wrap items-center gap-1.5">
          <button class={smallButton} title={keys('mod+ArrowDown')} onClick={() => void openHere(hash(), null, false)}>
            <Icon of={SquareArrowOutUpRight} size={12} />
            Open
          </button>
          <button class={smallButton} title={keys('mod+R')} onClick={() => void openHere(hash(), null, true)}>
            <Icon of={FolderOpen} size={12} />
            Reveal
          </button>
          <Menu
            label="More"
            iconOnly
            buttonClass="grid size-7 place-items-center rounded-button text-muted hover:bg-hover hover:text-fg aria-expanded:bg-hover"
            button={<Icon of={Ellipsis} size={16} />}
          >
            <MenuItem shortcut={keys('F2')} onSelect={() => renameTorrent(hash())}>
              Rename…
            </MenuItem>
            <MenuItem shortcut={keys('mod+L')} onSelect={() => setLocation([hash()])}>
              Set Location…
            </MenuItem>
            <MenuSeparator />
            <MenuItem onSelect={() => void act('verify', [hash()])}>Verify Local Data</MenuItem>
            <MenuItem onSelect={() => void copyMagnets([hash()])}>Copy Magnet Link</MenuItem>
          </Menu>
        </div>
      </header>

      <div class="flex gap-0.5 border-b border-line px-3" role="tablist" aria-label="Inspector">
        <For each={TABS}>
          {(tab) => (
            <button
              role="tab"
              aria-selected={inspectorTab() === tab.id}
              class="relative flex h-9 items-center gap-1 px-2 text-xs font-medium text-muted hover:text-fg aria-selected:text-fg"
              onClick={() => setInspectorTab(tab.id)}
            >
              {tab.label}
              <Show when={count(tab.id)}>
                {(n) => <span class="rounded-chip bg-element px-1.5 text-[10px] tabular-nums">{n()}</span>}
              </Show>
              <Show when={inspectorTab() === tab.id}>
                <span class="absolute inset-x-2 -bottom-px h-0.5 rounded-full bg-accent" aria-hidden="true" />
              </Show>
            </button>
          )}
        </For>
      </div>

      <div class="min-h-0 flex-1 overflow-y-auto" role="tabpanel">
        <Switch>
          <Match when={inspectorTab() === 'general'}>
            <GeneralTab t={t()} />
          </Match>
          <Match when={inspectorTab() === 'files'}>
            <FilesTab t={t()} />
          </Match>
          <Match when={inspectorTab() === 'peers'}>
            <PeersTab t={t()} />
          </Match>
          <Match when={inspectorTab() === 'trackers'}>
            <TrackersTab t={t()} />
          </Match>
          <Match when={inspectorTab() === 'options'}>
            <OptionsTab t={t()} />
          </Match>
        </Switch>
      </div>
    </>
  );
}

/** Several selected: their totals, and what can be done to them together. */
function Several(props: { count: number }) {
  const ts = () => [...selectedHashes()].map((h) => torrent(h)).filter((t) => t !== undefined);
  const size = () => ts().reduce((n, t) => n + (t.size_when_done ?? 0), 0);
  const done = () => ts().reduce((n, t) => n + (t.size_when_done ?? 0) - (t.left_until_done ?? 0), 0);
  return (
    <div class="p-5 pr-12">
      <h2 class="text-[15px] font-semibold">{props.count} torrents</h2>
      <p class="mt-1 text-xs text-muted">
        {formatSize(done())} of {formatSize(size())} downloaded
      </p>
      <div class="mt-4 flex flex-wrap gap-1.5">
        <button class={smallButton} onClick={() => void act('start')}>
          Resume
        </button>
        <button class={smallButton} onClick={() => void act('stop')}>
          Pause
        </button>
        <button class={smallButton} onClick={() => setLocation()}>
          Set Location…
        </button>
        <button class={`${smallButton} text-danger`} onClick={() => void remove(false)}>
          Remove…
        </button>
      </div>
    </div>
  );
}

/**
 * The inspector's left edge: drag to resize, double-click for the default
 * width, or focus it and use ←/→ (Shift for bigger steps). Hidden while the
 * inspector overlays a narrow window.
 */
function ResizeHandle() {
  const step = (e: KeyboardEvent) => (e.shiftKey ? 64 : 16);
  return (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize the inspector"
      aria-valuenow={inspectorWidth()}
      aria-valuemin={INSPECTOR_WIDTH.min}
      aria-valuemax={INSPECTOR_WIDTH.max}
      tabIndex={0}
      title="Drag to resize · double-click to reset"
      class="group/resize absolute inset-y-0 -left-1 z-2 w-2 cursor-col-resize outline-none max-[760px]:hidden"
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        const handle = e.currentTarget;
        handle.setPointerCapture(e.pointerId);
        handle.dataset.dragging = '';
        const startX = e.clientX;
        const startW = inspectorWidth();
        // Dragging left widens the inspector.
        const move = (ev: PointerEvent) => setInspectorWidth(startW + startX - ev.clientX, false);
        const end = () => {
          delete handle.dataset.dragging;
          setInspectorWidth(inspectorWidth());
          handle.removeEventListener('pointermove', move);
          handle.removeEventListener('pointerup', end);
          handle.removeEventListener('pointercancel', end);
        };
        handle.addEventListener('pointermove', move);
        handle.addEventListener('pointerup', end);
        handle.addEventListener('pointercancel', end);
      }}
      onDblClick={() => setInspectorWidth(INSPECTOR_WIDTH.initial)}
      onKeyDown={(e) => {
        if (e.key === 'ArrowLeft') setInspectorWidth(inspectorWidth() + step(e));
        else if (e.key === 'ArrowRight') setInspectorWidth(inspectorWidth() - step(e));
        else if (e.key === 'Home') setInspectorWidth(INSPECTOR_WIDTH.min);
        else if (e.key === 'End') setInspectorWidth(INSPECTOR_WIDTH.max);
        else return;
        e.preventDefault();
        e.stopPropagation();
      }}
    >
      {/* The visible line: on hover, while dragging, and when focused from the keyboard. */}
      <div class="mx-auto h-full w-0.5 bg-accent opacity-0 transition-opacity group-hover/resize:opacity-60 group-focus-visible/resize:opacity-100 group-data-dragging/resize:opacity-100" />
    </div>
  );
}
