import { For } from 'solid-js';
import type { Component } from 'solid-js';
import type { LucideProps } from 'lucide-solid';
import ArrowDown from 'lucide-solid/icons/arrow-down';
import ArrowUp from 'lucide-solid/icons/arrow-up';
import CircleAlert from 'lucide-solid/icons/circle-alert';
import CircleCheck from 'lucide-solid/icons/circle-check';
import CirclePause from 'lucide-solid/icons/circle-pause';
import Layers from 'lucide-solid/icons/layers';
import Zap from 'lucide-solid/icons/zap';

import { Icon } from './Icon';

const row =
  'group flex h-7 w-full cursor-default items-center gap-2 rounded-control px-2 text-left text-sm outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus aria-[current=true]:bg-highlight aria-[current=true]:text-accent';
const label = 'px-2 pt-4 pb-1 text-[11px] font-semibold tracking-[.06em] text-muted uppercase';

type Filter = { id: string; label: string; icon: Component<LucideProps> };

/** Status filters, in the web UI's order. Counts and selection come with the torrent store (phase 4). */
const FILTERS: Filter[] = [
  { id: 'all', label: 'All', icon: Layers },
  { id: 'active', label: 'Active', icon: Zap },
  { id: 'downloading', label: 'Downloading', icon: ArrowDown },
  { id: 'seeding', label: 'Seeding', icon: ArrowUp },
  { id: 'paused', label: 'Paused', icon: CirclePause },
  { id: 'finished', label: 'Finished', icon: CircleCheck },
  { id: 'error', label: 'Error', icon: CircleAlert },
];

/** Left sidebar: status filters, then labels. */
export function Sidebar() {
  return (
    <nav class="flex w-52 flex-none flex-col overflow-y-auto border-r border-line bg-panel p-2" aria-label="Filters">
      <div class={label}>Status</div>
      <For each={FILTERS}>
        {(f) => (
          <button class={row} aria-current={f.id === 'all'}>
            <Icon of={f.icon} class="text-muted group-aria-[current=true]:text-accent" />
            <span class="min-w-0 flex-1 truncate">{f.label}</span>
            <span class="text-xs text-faint tabular-nums">0</span>
          </button>
        )}
      </For>
      <div class={label}>Labels</div>
      <p class="px-2 text-xs text-faint">No labels yet.</p>
    </nav>
  );
}
