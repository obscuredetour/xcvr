// Which torrents the table shows, and in what order: the sidebar's status and
// label filters, the search box, then the sort. Re-runs when a torrent changes
// in a way that matters (sorting by speed re-sorts as speeds change).

import { createMemo, createRoot } from 'solid-js';

import type { Torrent } from './api';
import { FILTERS, type FilterId } from './format';
import { torrents } from './torrents';
import { collapsedGroups, groupByLabel, labelFilter, query, sort, type SortDir, statusFilter } from './view';

/** How each sortable column sorts: the value to compare, and which way it starts. */
export const SORTS: Record<string, { value(t: Torrent): number | string; first: SortDir }> = {
  name: { value: (t) => t.name ?? '', first: 'asc' },
  status: { value: (t) => statusRank(t), first: 'asc' },
  progress: { value: (t) => t.percent_done ?? 0, first: 'desc' },
  size: { value: (t) => t.size_when_done ?? 0, first: 'desc' },
  down: { value: (t) => t.rate_download ?? 0, first: 'desc' },
  up: { value: (t) => t.rate_upload ?? 0, first: 'desc' },
  // Unknown and never (-1, -2) after every real estimate.
  eta: { value: (t) => (t.status === 4 && (t.eta ?? -1) >= 0 ? t.eta! : Number.MAX_SAFE_INTEGER), first: 'asc' },
  // Infinite (-2) is the highest ratio; none yet (-1) the lowest.
  ratio: { value: (t) => (t.upload_ratio === -2 ? Number.MAX_SAFE_INTEGER : (t.upload_ratio ?? -1)), first: 'desc' },
  uploaded: { value: (t) => t.uploaded_ever ?? 0, first: 'desc' },
  added: { value: (t) => t.added_date ?? 0, first: 'desc' },
  done: { value: (t) => t.done_date ?? 0, first: 'desc' },
  labels: { value: (t) => (t.labels ?? []).join(', '), first: 'asc' },
  peers: { value: (t) => t.peers_connected ?? 0, first: 'desc' },
  queue: { value: (t) => t.queue_position ?? 0, first: 'asc' },
  location: { value: (t) => t.download_dir ?? '', first: 'asc' },
  seeding: { value: (t) => t.seconds_seeding ?? 0, first: 'desc' },
};

/** Status order for sorting: what needs attention first, then what's moving, then what's resting. */
function statusRank(t: Torrent): number {
  if ((t.error ?? 0) >= 2) return 0;
  return { 4: 1, 2: 2, 6: 3, 3: 4, 1: 5, 5: 6, 0: 7 }[t.status ?? 0] ?? 8;
}

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

function compare(a: number | string, b: number | string): number {
  return typeof a === 'string' && typeof b === 'string' ? collator.compare(a, b) : Number(a) - Number(b);
}

/** Lower case, accents dropped, for matching. */
const fold = (s: string) => s.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();

/** Every word of the search appears in the name or a label. */
function matches(t: Torrent, words: string[]): boolean {
  const haystack = fold([t.name ?? '', ...(t.labels ?? [])].join(' '));
  return words.every((w) => haystack.includes(w));
}

/** The torrents the table shows, filtered and sorted. */
export const shownTorrents = createRoot(() =>
  createMemo((): Torrent[] => {
    const status = FILTERS[statusFilter() as FilterId] ?? FILTERS.all;
    const label = labelFilter();
    const words = fold(query()).split(/\s+/).filter(Boolean);
    const list = torrents().filter(
      (t) =>
        status(t) &&
        (label === null || (label === '' ? !t.labels?.length : (t.labels ?? []).includes(label))) &&
        (words.length === 0 || matches(t, words)),
    );

    const { key, dir } = sort();
    const by = SORTS[key] ?? SORTS.added!;
    const sign = dir === 'asc' ? 1 : -1;
    // Ties by name, so the order is stable as values change.
    return list.sort((a, b) => sign * compare(by.value(a), by.value(b)) || compare(a.name ?? '', b.name ?? ''));
  }),
);

/** A row of the table: a label's header (when grouping), or a torrent. */
export type TableItem =
  | { kind: 'group'; label: string; count: number; collapsed: boolean }
  | { kind: 'torrent'; t: Torrent; group: string | null };

/**
 * What the table shows, top to bottom. Grouped by label, a torrent shows under
 * each label it has (Gmail-style), and under "No label" ('') when it has none;
 * labels A to Z, "No label" last. With a label chosen in the sidebar, just that
 * group shows.
 */
export const tableItems = createRoot(() =>
  createMemo((): TableItem[] => {
    const list = shownTorrents();
    if (!groupByLabel()) return list.map((t) => ({ kind: 'torrent', t, group: null }));

    const groups = new Map<string, Torrent[]>();
    const only = labelFilter();
    for (const t of list) {
      for (const label of t.labels?.length ? t.labels : ['']) {
        if (only !== null && label !== only) continue;
        const members = groups.get(label);
        if (members) members.push(t);
        else groups.set(label, [t]);
      }
    }
    const order = [...groups.keys()].sort((a, b) => (a === '' ? 1 : b === '' ? -1 : collator.compare(a, b)));
    return order.flatMap((label): TableItem[] => {
      const members = groups.get(label)!;
      const collapsed = collapsedGroups().has(label);
      return [
        { kind: 'group', label, count: members.length, collapsed },
        ...(collapsed ? [] : members.map((t): TableItem => ({ kind: 'torrent', t, group: label }))),
      ];
    });
  }),
);

/** Torrents in the order they're on screen, each once: what ↑/↓ and Shift-click go through. */
export const navOrder = createRoot(() =>
  createMemo((): Torrent[] => {
    if (!groupByLabel()) return shownTorrents();
    const seen = new Set<string>();
    const out: Torrent[] = [];
    for (const item of tableItems()) {
      if (item.kind === 'torrent' && !seen.has(item.t.hash_string!)) {
        seen.add(item.t.hash_string!);
        out.push(item.t);
      }
    }
    return out;
  }),
);

/** Whether anything is narrowing the list (for the empty message). */
export const isFiltered = () => statusFilter() !== 'all' || labelFilter() !== null || query().trim() !== '';
