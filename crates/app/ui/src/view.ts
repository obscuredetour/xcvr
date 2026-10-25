// View preferences (per device).

import { createSignal } from 'solid-js';

import { storage } from './storage';

/** Alternating row colours in the torrent table; on unless turned off. */
const [zebraRows, setZebraRowsSignal] = createSignal(storage.get('tableZebra') !== 'off');
export { zebraRows };

export function setZebraRows(on: boolean): void {
  setZebraRowsSignal(on);
  storage.set('tableZebra', on ? 'on' : 'off');
}

/** Width of the inspector in CSS px (it also never squeezes the table below 320px). */
export const INSPECTOR_WIDTH = { min: 300, max: 680, initial: 380 } as const;

const clampInspector = (n: number) => Math.min(INSPECTOR_WIDTH.max, Math.max(INSPECTOR_WIDTH.min, Math.round(n)));
const storedInspector = Number(storage.get('inspectorWidth'));
const [inspectorWidth, setInspectorWidthSignal] = createSignal(
  storedInspector ? clampInspector(storedInspector) : INSPECTOR_WIDTH.initial,
);
export { inspectorWidth };

/** Set the inspector's width; `save` stores it (at the end of a drag, not on every move). */
export function setInspectorWidth(n: number, save = true): void {
  const w = clampInspector(n);
  setInspectorWidthSignal(w);
  if (save) storage.set('inspectorWidth', String(w));
}

/** The inspector; shown unless turned off. */
const [inspectorOpen, setInspectorOpenSignal] = createSignal(storage.get('inspector') !== 'off');
export { inspectorOpen };

export function setInspectorOpen(open: boolean): void {
  setInspectorOpenSignal(open);
  storage.set('inspector', open ? 'on' : 'off');
}

/** The filters sidebar; shown unless turned off. */
const [sidebarOpen, setSidebarOpenSignal] = createSignal(storage.get('sidebar') !== 'off');
export { sidebarOpen };

export function setSidebarOpen(open: boolean): void {
  setSidebarOpenSignal(open);
  storage.set('sidebar', open ? 'on' : 'off');
}

// ---------- The torrent table: columns ----------

function storedJson<T>(key: string, valid: (v: unknown) => v is T, fallback: T): T {
  try {
    const parsed: unknown = JSON.parse(storage.get(key) ?? 'null');
    return valid(parsed) ? parsed : fallback;
  } catch {
    return fallback;
  }
}

const isRecord = (v: unknown): v is Record<string, never> => !!v && typeof v === 'object' && !Array.isArray(v);
const isStrings = (v: unknown): v is string[] => Array.isArray(v) && v.every((x) => typeof x === 'string');

/** Column widths (px), by column id; columns not in it keep their default. */
const [columnSizing, setColumnSizingSignal] = createSignal<Record<string, number>>(
  storedJson('tableColumnSizing', isRecord, {}),
);
export { columnSizing };

export function setColumnSizing(sizing: Record<string, number>): void {
  setColumnSizingSignal(sizing);
  storage.set('tableColumnSizing', JSON.stringify(sizing));
}

/** Columns hidden until they're turned on. */
const DEFAULT_HIDDEN: Record<string, boolean> = {
  uploaded: false,
  done: false,
  peers: false,
  queue: false,
  location: false,
  seeding: false,
};

const [columnVisibility, setColumnVisibilitySignal] = createSignal<Record<string, boolean>>(
  storedJson('tableColumnVisibility', isRecord, {}),
);

/** Visibility of every column that isn't shown by default, and every one the user changed. */
export const effectiveColumnVisibility = (): Record<string, boolean> => ({ ...DEFAULT_HIDDEN, ...columnVisibility() });

export function setColumnVisibility(visibility: Record<string, boolean>): void {
  setColumnVisibilitySignal(visibility);
  storage.set('tableColumnVisibility', JSON.stringify(visibility));
}

/** Column ids in display order; columns it doesn't name keep their default place after these. Empty: the default. */
const [columnOrder, setColumnOrderSignal] = createSignal<string[]>(storedJson('tableColumnOrder', isStrings, []));
export { columnOrder };

export function setColumnOrder(order: string[]): void {
  setColumnOrderSignal(order);
  storage.set('tableColumnOrder', JSON.stringify(order));
}

export function resetColumnOrder(): void {
  setColumnOrder([]);
}

// ---------- The torrent table: sort ----------

export type SortDir = 'asc' | 'desc';
export type Sort = { key: string; dir: SortDir };

const isSort = (v: unknown): v is Sort => {
  if (!isRecord(v)) return false;
  const s = v as Partial<Record<keyof Sort, unknown>>;
  return typeof s.key === 'string' && (s.dir === 'asc' || s.dir === 'desc');
};

/** Newest first until the user picks a column. */
const [sort, setSortSignal] = createSignal<Sort>(storedJson('tableSort', isSort, { key: 'added', dir: 'desc' }));
export { sort };

export function setSort(next: Sort): void {
  setSortSignal(next);
  storage.set('tableSort', JSON.stringify(next));
}

/** Click a column: sort by it, or reverse it if it's already the sort. Numbers start biggest-first. */
export function toggleSort(key: string, firstDir: SortDir): void {
  const s = sort();
  setSort(s.key === key ? { key, dir: s.dir === 'asc' ? 'desc' : 'asc' } : { key, dir: firstDir });
}

// ---------- Filters (the sidebar) and search ----------

/** The sidebar's status filter: an id from FILTERS (format.ts). */
const [statusFilter, setStatusFilterSignal] = createSignal<string>(storage.get('statusFilter') ?? 'all');
export { statusFilter };

export function setStatusFilter(id: string): void {
  setStatusFilterSignal(id);
  storage.set('statusFilter', id);
}

/** The sidebar's label filter: a label, '' for torrents without one, or null for any. */
const [labelFilter, setLabelFilterSignal] = createSignal<string | null>(storage.get('labelFilter'));
export { labelFilter };

export function setLabelFilter(label: string | null): void {
  setLabelFilterSignal(label);
  if (label === null) storage.remove('labelFilter');
  else storage.set('labelFilter', label);
}

/** The search box's text (not kept between launches). */
const [query, setQuery] = createSignal('');
export { query, setQuery };
