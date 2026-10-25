import { createMemo, For, Show } from 'solid-js';
import type { Component } from 'solid-js';
import type { LucideProps } from 'lucide-solid';
import ArrowDown from 'lucide-solid/icons/arrow-down';
import ArrowUp from 'lucide-solid/icons/arrow-up';
import CircleAlert from 'lucide-solid/icons/circle-alert';
import CircleCheck from 'lucide-solid/icons/circle-check';
import CirclePause from 'lucide-solid/icons/circle-pause';
import Layers from 'lucide-solid/icons/layers';
import Tag from 'lucide-solid/icons/tag';
import Zap from 'lucide-solid/icons/zap';

import { FILTERS, type FilterId } from '../format';
import { torrents } from '../torrents';
import { Icon } from './Icon';

const row =
  'group flex h-7 w-full cursor-default items-center gap-2 rounded-control px-2 text-left text-sm outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus aria-[current=true]:bg-highlight aria-[current=true]:text-accent';
const label = 'px-2 pt-4 pb-1 text-[11px] font-semibold tracking-[.06em] text-muted uppercase';

type Filter = { id: FilterId; label: string; icon: Component<LucideProps> };

/** Status filters, in the web UI's order. Choosing one filters the table (phase 4). */
const STATUS: Filter[] = [
  { id: 'all', label: 'All', icon: Layers },
  { id: 'active', label: 'Active', icon: Zap },
  { id: 'downloading', label: 'Downloading', icon: ArrowDown },
  { id: 'seeding', label: 'Seeding', icon: ArrowUp },
  { id: 'paused', label: 'Paused', icon: CirclePause },
  { id: 'finished', label: 'Finished', icon: CircleCheck },
  { id: 'error', label: 'Error', icon: CircleAlert },
];

/** Left sidebar: status filters and labels, with live counts. */
export function Sidebar() {
  // Labels in use, A to Z, with how many torrents have each.
  const labels = createMemo(() => {
    const counts = new Map<string, number>();
    for (const t of torrents()) for (const l of t.labels ?? []) counts.set(l, (counts.get(l) ?? 0) + 1);
    return [...counts].sort(([a], [b]) => a.localeCompare(b));
  });

  return (
    <nav class="flex w-52 flex-none flex-col overflow-y-auto border-r border-line bg-panel p-2" aria-label="Filters">
      <div class={label}>Status</div>
      <For each={STATUS}>
        {(f) => (
          <button class={row} aria-current={f.id === 'all'}>
            <Icon of={f.icon} class="text-muted group-aria-[current=true]:text-accent" />
            <span class="min-w-0 flex-1 truncate">{f.label}</span>
            <span class="text-xs text-faint tabular-nums">{torrents().filter(FILTERS[f.id]).length}</span>
          </button>
        )}
      </For>
      <div class={label}>Labels</div>
      <Show when={labels().length} fallback={<p class="px-2 text-xs text-faint">No labels yet.</p>}>
        <For each={labels()}>
          {([name, count]) => (
            <button class={row}>
              <Icon of={Tag} class="text-muted" />
              <span class="min-w-0 flex-1 truncate">{name}</span>
              <span class="text-xs text-faint tabular-nums">{count}</span>
            </button>
          )}
        </For>
      </Show>
    </nav>
  );
}
