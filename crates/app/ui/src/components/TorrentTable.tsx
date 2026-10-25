import {
  type ColumnDef,
  columnOrderingFeature,
  columnResizingFeature,
  columnSizingFeature,
  columnVisibilityFeature,
  createColumnHelper,
  createTable,
  FlexRender,
  type Header,
  tableFeatures,
  type Updater,
} from '@tanstack/solid-table';
import { createVirtualizer } from '@tanstack/solid-virtual';
import { createSignal, For, Index, onMount, Show } from 'solid-js';
import ChevronDown from 'lucide-solid/icons/chevron-down';
import ChevronUp from 'lucide-solid/icons/chevron-up';
import CircleAlert from 'lucide-solid/icons/circle-alert';
import Ellipsis from 'lucide-solid/icons/ellipsis';
import TriangleAlert from 'lucide-solid/icons/triangle-alert';

import type { Torrent } from '../api';
import { SORTS } from '../browse';
import {
  formatDate,
  formatDateTime,
  formatDuration,
  formatEta,
  formatPercent,
  formatRatio,
  formatSize,
  formatSpeed,
  progressOf,
  statusLabel,
  statusTone,
  type Tone,
} from '../format';
import { keys } from '../keys';
import {
  columnOrder,
  columnSizing,
  effectiveColumnVisibility,
  resetColumnOrder,
  setColumnOrder,
  setColumnSizing,
  setColumnVisibility,
  sort,
  toggleSort,
  zebraRows,
} from '../view';
import { Icon } from './Icon';
import { Menu, MenuItem, MenuLabel, MenuSeparator } from './Menu';

const ROW_H = 32;
/** Always shown: every row needs a name. */
const LOCKED_COLUMN = 'name';

// Sorting and filtering happen before the table (browse.ts); TanStack handles
// column sizing, resizing, order and visibility.
const features = tableFeatures({
  columnSizingFeature,
  columnResizingFeature,
  columnVisibilityFeature,
  columnOrderingFeature,
});

type Meta = {
  /** Sorts by SORTS[id] when true. */
  sortable?: boolean;
  align?: 'end';
  /** Shown in the column picker. */
  label: string;
};

const col = createColumnHelper<typeof features, Torrent>();

// Cells read the torrent (a store proxy) directly, so a change to one field
// updates only the cells that show it.

const muted = (text: string) => <span class="text-muted">{text}</span>;

/** Coming in and going out in the icon's colours (--down, --up), not the accent. */
const TONE_BG: Record<Tone, string> = {
  download: 'bg-down',
  seed: 'bg-up',
  idle: 'bg-faint',
  check: 'bg-info',
  error: 'bg-danger',
};

function NameCell(props: { t: Torrent }) {
  const problem = () => ((props.t.error ?? 0) > 0 ? props.t.error_string || 'Error' : null);
  return (
    <span class="flex min-w-0 items-center gap-1.5">
      <Show when={problem()}>
        {(text) => (
          <span
            class="flex-none"
            classList={{ 'text-warning': props.t.error === 1, 'text-danger': (props.t.error ?? 0) >= 2 }}
            title={text()}
          >
            <Icon of={props.t.error === 1 ? TriangleAlert : CircleAlert} size={14} />
          </span>
        )}
      </Show>
      <span class="truncate font-medium" title={props.t.name}>
        {props.t.name}
      </span>
    </span>
  );
}

function StatusCell(props: { t: Torrent }) {
  return (
    <span class="flex min-w-0 items-center gap-1.5">
      <span class={`size-1.5 flex-none rounded-full ${TONE_BG[statusTone(props.t)]}`} aria-hidden="true" />
      <span class="truncate">{statusLabel(props.t)}</span>
    </span>
  );
}

function ProgressCell(props: { t: Torrent }) {
  const p = () => progressOf(props.t);
  return (
    <span class="flex items-center gap-2">
      <span class="h-1.5 min-w-0 flex-1 overflow-hidden rounded-full bg-element in-data-selected:bg-on-selection/25">
        <span
          class={`block h-full rounded-full in-data-selected:bg-on-selection ${TONE_BG[statusTone(props.t)]}`}
          style={{ width: `${p() * 100}%` }}
        />
      </span>
      <span class="w-11 flex-none text-right text-xs tabular-nums">{formatPercent(p())}</span>
    </span>
  );
}

function LabelsCell(props: { t: Torrent }) {
  return (
    <span class="flex min-w-0 gap-1 overflow-hidden">
      <For each={props.t.labels ?? []}>
        {(label) => (
          <span class="flex-none rounded-chip bg-element px-1.5 text-[11px] font-medium text-muted in-data-selected:bg-on-selection/20">
            {label}
          </span>
        )}
      </For>
    </span>
  );
}

/** A speed, blank at zero so the moving ones stand out. */
const speed = (bps: number | undefined) => (bps ? formatSpeed(bps) : '');

const columns: ColumnDef<typeof features, Torrent, unknown>[] = [
  col.display({
    id: 'name',
    size: 340,
    minSize: 160,
    header: 'Name',
    meta: { sortable: true, label: 'Name' } satisfies Meta,
    cell: (info) => <NameCell t={info.row.original} />,
  }),
  col.display({
    id: 'status',
    size: 130,
    header: 'Status',
    meta: { sortable: true, label: 'Status' } satisfies Meta,
    cell: (info) => <StatusCell t={info.row.original} />,
  }),
  col.display({
    id: 'progress',
    size: 150,
    minSize: 90,
    header: 'Progress',
    meta: { sortable: true, label: 'Progress' } satisfies Meta,
    cell: (info) => <ProgressCell t={info.row.original} />,
  }),
  col.display({
    id: 'size',
    size: 90,
    header: 'Size',
    meta: { sortable: true, align: 'end', label: 'Size' } satisfies Meta,
    cell: (info) => formatSize(info.row.original.size_when_done ?? 0),
  }),
  col.display({
    id: 'down',
    size: 90,
    header: 'Down',
    meta: { sortable: true, align: 'end', label: 'Download Speed' } satisfies Meta,
    cell: (info) => speed(info.row.original.rate_download),
  }),
  col.display({
    id: 'up',
    size: 90,
    header: 'Up',
    meta: { sortable: true, align: 'end', label: 'Upload Speed' } satisfies Meta,
    cell: (info) => speed(info.row.original.rate_upload),
  }),
  col.display({
    id: 'eta',
    size: 80,
    header: 'ETA',
    meta: { sortable: true, align: 'end', label: 'Time Left' } satisfies Meta,
    cell: (info) => formatEta(info.row.original),
  }),
  col.display({
    id: 'ratio',
    size: 70,
    header: 'Ratio',
    meta: { sortable: true, align: 'end', label: 'Ratio' } satisfies Meta,
    cell: (info) => formatRatio(info.row.original.upload_ratio),
  }),
  col.display({
    id: 'uploaded',
    size: 90,
    header: 'Uploaded',
    meta: { sortable: true, align: 'end', label: 'Uploaded' } satisfies Meta,
    cell: (info) => formatSize(info.row.original.uploaded_ever ?? 0),
  }),
  col.display({
    id: 'added',
    size: 110,
    header: 'Added',
    meta: { sortable: true, align: 'end', label: 'Date Added' } satisfies Meta,
    cell: (info) => (
      <span title={formatDateTime(info.row.original.added_date)}>
        {muted(formatDate(info.row.original.added_date))}
      </span>
    ),
  }),
  col.display({
    id: 'done',
    size: 110,
    header: 'Completed',
    meta: { sortable: true, align: 'end', label: 'Date Completed' } satisfies Meta,
    cell: (info) => (
      <span title={formatDateTime(info.row.original.done_date)}>{muted(formatDate(info.row.original.done_date))}</span>
    ),
  }),
  col.display({
    id: 'seeding',
    size: 90,
    header: 'Seeding',
    meta: { sortable: true, align: 'end', label: 'Seeding Time' } satisfies Meta,
    cell: (info) => {
      const s = info.row.original.seconds_seeding ?? 0;
      return s ? formatDuration(s) : '';
    },
  }),
  col.display({
    id: 'peers',
    size: 70,
    header: 'Peers',
    meta: { sortable: true, align: 'end', label: 'Peers' } satisfies Meta,
    cell: (info) => String(info.row.original.peers_connected ?? 0),
  }),
  col.display({
    id: 'queue',
    size: 64,
    header: 'Queue',
    meta: { sortable: true, align: 'end', label: 'Queue Position' } satisfies Meta,
    cell: (info) => `#${(info.row.original.queue_position ?? 0) + 1}`,
  }),
  col.display({
    id: 'labels',
    size: 160,
    header: 'Labels',
    meta: { sortable: true, label: 'Labels' } satisfies Meta,
    cell: (info) => <LabelsCell t={info.row.original} />,
  }),
  col.display({
    id: 'location',
    size: 220,
    header: 'Location',
    meta: { sortable: true, label: 'Location' } satisfies Meta,
    cell: (info) => <span title={info.row.original.download_dir}>{muted(info.row.original.download_dir ?? '')}</span>,
  }),
];

const meta = (h: { column: { columnDef: { meta?: unknown } } }) => h.column.columnDef.meta as Meta;
const apply = <T,>(updater: Updater<T>, prev: T): T =>
  typeof updater === 'function' ? (updater as (old: T) => T)(prev) : updater;

export type TableHandle = {
  /** Scroll a torrent into view. */
  reveal(hash: string): void;
  focus(): void;
};

type Props = {
  rows: Torrent[];
  selectedHashes: ReadonlySet<string>;
  /** Shown when there are no rows. */
  emptyMessage: string;
  /** A click; modifier keys (⌘, Shift) extend the selection. */
  onSelect(hash: string, e: MouseEvent): void;
  /** A double-click: show it in the inspector. */
  onOpen(hash: string): void;
  ref?: (handle: TableHandle) => void;
};

/** Virtualized torrent table with resizable, reorderable, hideable columns. */
export function TorrentTable(props: Props) {
  let scroller!: HTMLDivElement;

  const table = createTable({
    features,
    columns,
    get data() {
      return props.rows;
    },
    getRowId: (row) => row.hash_string!,
    columnResizeMode: 'onChange',
    state: {
      get columnSizing() {
        return columnSizing();
      },
      get columnVisibility() {
        return effectiveColumnVisibility();
      },
      get columnOrder() {
        return columnOrder();
      },
    },
    onColumnSizingChange: (u) => setColumnSizing(apply(u, columnSizing())),
    onColumnVisibilityChange: (u) => setColumnVisibility(apply(u, effectiveColumnVisibility())),
    onColumnOrderChange: (u) => setColumnOrder(apply(u, columnOrder())),
  });

  // ---------- Reordering: drag a header sideways, or ⌥←/⌥→ on a focused one ----------
  // Pointer events rather than HTML5 drag and drop, which the window's file
  // drop (adding torrents) intercepts on some platforms.

  let headerRow!: HTMLDivElement;
  /** The header being dragged: its id, how far it's moved, and where it would land. */
  const [drag, setDrag] = createSignal<{ id: string; dx: number; to: number; marker: number } | null>(null);
  /** A drag ends in a click on the header; this keeps it from sorting. */
  let suppressClick = false;

  /** Move a visible column to position `to` among the visible ones; hidden columns keep their slots. */
  function moveColumn(id: string, to: number): void {
    const all = table.getAllLeafColumns().map((c) => c.id);
    const visible = table.getVisibleLeafColumns().map((c) => c.id);
    const from = visible.indexOf(id);
    if (from < 0 || from === to) return;
    const reordered = visible.filter((v) => v !== id);
    reordered.splice(to, 0, id);
    let next = 0;
    setColumnOrder(all.map((c) => (visible.includes(c) ? reordered[next++]! : c)));
  }

  function startDrag(e: PointerEvent, id: string): void {
    if (e.button !== 0) return;
    const cells = [...headerRow.querySelectorAll<HTMLElement>('[role=columnheader]')].map((el) => ({
      id: el.dataset.columnId!,
      rect: el.getBoundingClientRect(),
    }));
    const self = cells.find((c) => c.id === id);
    if (!self) return;
    const others = cells.filter((c) => c.id !== id);
    const rowLeft = headerRow.getBoundingClientRect().left;
    const startX = e.clientX;
    let started = false;
    // On the window, so a quick drag that leaves the header still counts.
    const move = (ev: PointerEvent) => {
      const dx = ev.clientX - startX;
      if (!started && Math.abs(dx) < 5) return;
      started = true;
      // It lands after every other column whose middle it has passed.
      const centre = self.rect.left + self.rect.width / 2 + dx;
      const to = others.filter((c) => c.rect.left + c.rect.width / 2 < centre).length;
      const edge = to < others.length ? others[to]!.rect.left : others[others.length - 1]!.rect.right;
      setDrag({ id, dx, to, marker: edge - rowLeft });
    };
    const end = () => {
      removeEventListener('pointermove', move);
      removeEventListener('pointerup', end);
      removeEventListener('pointercancel', end);
      const d = drag();
      setDrag(null);
      if (!started) return;
      suppressClick = true;
      setTimeout(() => (suppressClick = false));
      if (d) moveColumn(d.id, d.to);
    };
    addEventListener('pointermove', move);
    addEventListener('pointerup', end);
    addEventListener('pointercancel', end);
  }

  function keyMove(e: KeyboardEvent, id: string): void {
    if (!e.altKey || (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight')) return;
    e.preventDefault();
    e.stopPropagation();
    const visible = table.getVisibleLeafColumns().map((c) => c.id);
    const to = visible.indexOf(id) + (e.key === 'ArrowLeft' ? -1 : 1);
    if (to < 0 || to >= visible.length) return;
    moveColumn(id, to);
    // Keep focus on the moved header (the cells are reused by position).
    headerRow.querySelector<HTMLElement>(`[data-column-id="${id}"] button`)?.focus();
  }

  const virtualizer = createVirtualizer({
    get count() {
      return props.rows.length;
    },
    getScrollElement: () => scroller,
    estimateSize: () => ROW_H,
    overscan: 12,
  });

  onMount(() =>
    props.ref?.({
      reveal(hash) {
        const i = props.rows.findIndex((r) => r.hash_string === hash);
        if (i >= 0) virtualizer.scrollToIndex(i, { align: 'auto' });
      },
      focus: () => scroller.focus(),
    }),
  );

  const rows = () => table.getRowModel().rows;
  const totalWidth = () => table.getTotalSize();

  return (
    <div
      ref={scroller}
      class="relative min-w-0 flex-1 overflow-auto outline-none"
      tabIndex={0}
      role="table"
      aria-label="Torrents"
      aria-rowcount={props.rows.length}
    >
      <div
        ref={headerRow}
        class="sticky top-0 z-2 flex border-b border-line bg-bg/90 backdrop-blur"
        style={{ width: `${totalWidth()}px`, 'min-width': '100%' }}
        role="row"
      >
        {/* Index (not For): header objects are rebuilt on every resize, so reuse the DOM by position. */}
        <Index each={table.getHeaderGroups()[0]?.headers}>
          {(h) => (
            <HeaderCell
              header={h()}
              dragOffset={drag()?.id === h().column.id ? drag()!.dx : null}
              onDragStart={(e) => startDrag(e, h().column.id)}
              onKeyDown={(e) => keyMove(e, h().column.id)}
              suppressClick={() => suppressClick}
            />
          )}
        </Index>
        {/* Where a dragged column would land. */}
        <Show when={drag()}>
          {(d) => (
            <div
              class="pointer-events-none absolute inset-y-1 z-3 w-0.5 -translate-x-1/2 rounded-chip bg-accent"
              style={{ left: `${d().marker}px` }}
            />
          )}
        </Show>
        <div class="sticky right-0 ml-auto flex items-center bg-[linear-gradient(to_right,transparent,var(--bg)_45%)] pr-2 pl-6">
          <ColumnPicker table={table} />
        </div>
      </div>

      <Show when={props.rows.length === 0}>
        <p class="px-4 py-10 text-center text-muted">{props.emptyMessage}</p>
      </Show>

      <div
        class="relative"
        style={{ height: `${virtualizer.getTotalSize()}px`, width: `${totalWidth()}px`, 'min-width': '100%' }}
      >
        <For each={virtualizer.getVirtualItems()}>
          {(v) => {
            const row = () => rows()[v.index];
            return (
              <Show when={row()}>
                {(r) => (
                  <div
                    role="row"
                    aria-rowindex={v.index + 2}
                    class="absolute top-0 left-0 flex w-full cursor-default items-center not-data-selected:hover:bg-hover data-selected:bg-selection data-selected:text-on-selection data-selected:**:text-on-selection"
                    // With stripes, a hairline between rows too; without, nothing.
                    classList={{
                      'bg-stripe': zebraRows() && v.index % 2 === 1,
                      'border-b border-line/75': !zebraRows(),
                    }}
                    data-selected={props.selectedHashes.has(r().id) || undefined}
                    data-hash={r().id}
                    style={{ height: `${ROW_H}px`, transform: `translateY(${v.start}px)` }}
                    onClick={(e) => props.onSelect(r().id, e)}
                    onDblClick={() => props.onOpen(r().id)}
                  >
                    <For each={r().getVisibleCells()}>
                      {(cell) => (
                        <div
                          role="cell"
                          class="flex-none truncate px-3"
                          classList={{ 'text-right tabular-nums': meta(cell).align === 'end' }}
                          style={{ width: `${cell.column.getSize()}px` }}
                        >
                          <FlexRender cell={cell} />
                        </div>
                      )}
                    </For>
                  </div>
                )}
              </Show>
            );
          }}
        </For>
      </div>
    </div>
  );
}

function HeaderCell(props: {
  header: Header<typeof features, Torrent, unknown>;
  /** While this header is dragged: how far it has moved (px). */
  dragOffset: number | null;
  onDragStart(e: PointerEvent): void;
  onKeyDown(e: KeyboardEvent): void;
  suppressClick(): boolean;
}) {
  const h = () => props.header;
  const id = () => h().column.id;
  const sortable = () => !!meta(h()).sortable;
  const active = () => sortable() && sort().key === id();
  return (
    <div
      role="columnheader"
      data-column-id={id()}
      aria-sort={active() ? (sort().dir === 'asc' ? 'ascending' : 'descending') : undefined}
      class="group/h relative flex h-8 flex-none items-center text-xs font-semibold text-muted"
      classList={{ 'z-3 rounded-control bg-popover shadow-lift': props.dragOffset !== null }}
      style={{
        width: `${h().getSize()}px`,
        transform: props.dragOffset !== null ? `translateX(${props.dragOffset}px)` : undefined,
      }}
    >
      {/* Every header can be dragged and moved; click sorts. */}
      <button
        class="flex h-full w-full min-w-0 cursor-default items-center gap-1 px-3 text-left"
        classList={{
          'justify-end': meta(h()).align === 'end',
          'text-fg': active(),
          'hover:text-fg': sortable(),
        }}
        title={`${sortable() ? 'Click to sort; drag' : 'Drag'} to move (or ${keys('alt+ArrowLeft')} ${keys('alt+ArrowRight')})`}
        onPointerDown={(e) => props.onDragStart(e)}
        onKeyDown={(e) => props.onKeyDown(e)}
        onClick={() => {
          if (props.suppressClick() || !sortable()) return;
          toggleSort(id(), SORTS[id()]?.first ?? 'asc');
        }}
      >
        <span class="truncate">
          <FlexRender header={h()} />
        </span>
        <Show when={active()}>
          <Icon of={sort().dir === 'asc' ? ChevronUp : ChevronDown} size={12} />
        </Show>
      </button>
      {/* Drag to resize; double-click to reset. */}
      <div
        class="absolute top-1.5 right-0 bottom-1.5 w-[5px] cursor-col-resize border-r border-line group-hover/h:border-faint data-resizing:border-accent"
        data-resizing={h().column.getIsResizing() || undefined}
        onMouseDown={(e) => h().getResizeHandler()(e)}
        onTouchStart={(e) => h().getResizeHandler()(e)}
        onDblClick={() => h().column.resetSize()}
        onClick={(e) => e.stopPropagation()}
      />
    </div>
  );
}

function ColumnPicker(props: { table: ReturnType<typeof createTable<typeof features, Torrent>> }) {
  return (
    <Menu
      label="Columns"
      iconOnly
      align="end"
      buttonClass="grid size-6 place-items-center rounded-button hover:bg-hover aria-expanded:bg-hover"
      button={<Icon of={Ellipsis} class="text-muted" />}
    >
      <MenuLabel>Columns</MenuLabel>
      {/* In display order. */}
      <For each={props.table.getAllLeafColumns()}>
        {(column) => (
          <MenuItem
            checkbox
            keepOpen
            checked={column.getIsVisible()}
            disabled={column.id === LOCKED_COLUMN}
            onSelect={() => column.toggleVisibility()}
          >
            {(column.columnDef.meta as Meta).label}
          </MenuItem>
        )}
      </For>
      <Show when={columnOrder().length}>
        <MenuSeparator />
        <MenuItem onSelect={() => resetColumnOrder()}>Reset Column Order</MenuItem>
      </Show>
    </Menu>
  );
}
