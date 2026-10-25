import { For, Show } from 'solid-js';
import X from 'lucide-solid/icons/x';

import type { Torrent } from '../api';
import {
  formatDateTime,
  formatPercent,
  formatRatio,
  formatSize,
  formatSpeed,
  progressOf,
  statusLabel,
} from '../format';
import { selectedHash, selectedHashes } from '../selection';
import { torrent } from '../torrents';
import { INSPECTOR_WIDTH, inspectorWidth, setInspectorWidth } from '../view';
import { widthFlags } from '../widths';
import { Icon } from './Icon';

/**
 * The right sidebar: the selected torrent's details, files, peers, trackers
 * and options (phase 6). Resizable from its left edge, like Magnitide's
 * details panel.
 */
export function Inspector(props: { onClose(): void }) {
  return (
    <aside
      ref={(el) => widthFlags(el, { wide: 416 })}
      class="group/inspector relative flex-none animate-slide-in border-l border-line bg-panel select-text max-[760px]:absolute max-[760px]:inset-y-0 max-[760px]:right-0 max-[760px]:w-[min(28rem,100%-2rem)]! max-[760px]:max-w-full! max-[760px]:shadow-lift"
      // The table keeps at least 320px beside it.
      style={{ width: `${inspectorWidth()}px`, 'max-width': 'calc(100% - 320px)' }}
      aria-label="Inspector"
    >
      <ResizeHandle />
      <button
        class="absolute top-2.5 right-2.5 z-1 grid size-7 place-items-center rounded-full hover:bg-hover"
        title="Close"
        aria-label="Close inspector"
        onClick={() => props.onClose()}
      >
        <Icon of={X} class="text-muted" />
      </button>
      <Show
        when={selectedHashes().size > 1}
        fallback={
          <Show
            when={torrent(selectedHash() ?? '')}
            fallback={
              <div class="grid h-full place-items-center p-6 text-center text-sm text-muted">Select a torrent</div>
            }
          >
            {(t) => <Summary t={t()} />}
          </Show>
        }
      >
        <div class="grid h-full place-items-center p-6 text-center text-sm text-muted">
          {selectedHashes().size} torrents selected
        </div>
      </Show>
    </aside>
  );
}

/** The basics of one torrent. Files, peers, trackers and options join it in the full inspector. */
function Summary(props: { t: Torrent }) {
  const t = () => props.t;
  const rows = (): [string, string][] => [
    ['Status', statusLabel(t())],
    ['Progress', `${formatPercent(progressOf(t()))} of ${formatSize(t().size_when_done ?? 0)}`],
    ['Downloading', t().rate_download ? formatSpeed(t().rate_download!) : '–'],
    ['Uploading', t().rate_upload ? formatSpeed(t().rate_upload!) : '–'],
    ['Ratio', formatRatio(t().upload_ratio)],
    ['Uploaded', formatSize(t().uploaded_ever ?? 0)],
    ['Added', formatDateTime(t().added_date)],
    ['Completed', formatDateTime(t().done_date) || '–'],
    ['Location', t().download_dir ?? ''],
  ];
  return (
    <div class="h-full overflow-y-auto p-5 pr-12">
      <h2 class="text-[15px]/snug font-semibold break-words">{t().name}</h2>
      <Show when={(t().error ?? 0) > 0 && t().error_string}>
        {(msg) => (
          <p class="mt-2 text-xs" classList={{ 'text-warning': t().error === 1, 'text-danger': (t().error ?? 0) >= 2 }}>
            {msg()}
          </p>
        )}
      </Show>
      <Show when={t().labels?.length}>
        <div class="mt-2 flex flex-wrap gap-1">
          <For each={t().labels}>
            {(label) => <span class="rounded-chip bg-element px-1.5 text-[11px] font-medium text-muted">{label}</span>}
          </For>
        </div>
      </Show>
      <dl class="mt-4 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-xs">
        <For each={rows()}>
          {([label, value]) => (
            <>
              <dt class="text-muted">{label}</dt>
              <dd class="min-w-0 break-words tabular-nums">{value}</dd>
            </>
          )}
        </For>
      </dl>
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
