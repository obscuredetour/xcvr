import { createSignal, For, Show } from 'solid-js';
import type { Component } from 'solid-js';
import type { LucideProps } from 'lucide-solid';
import ArrowDown from 'lucide-solid/icons/arrow-down';
import ArrowUp from 'lucide-solid/icons/arrow-up';
import CircleAlert from 'lucide-solid/icons/circle-alert';
import CircleCheck from 'lucide-solid/icons/circle-check';
import CirclePause from 'lucide-solid/icons/circle-pause';
import Layers from 'lucide-solid/icons/layers';
import Rows3 from 'lucide-solid/icons/rows-3';
import Tag from 'lucide-solid/icons/tag';
import TagsIcon from 'lucide-solid/icons/tags';
import Zap from 'lucide-solid/icons/zap';

import { FILTERS, type FilterId } from '../format';
import { allLabels, deleteLabel, renameLabel } from '../labels';
import { torrents } from '../torrents';
import { groupByLabel, labelFilter, setGroupByLabel, setLabelFilter, setStatusFilter, statusFilter } from '../view';
import { Icon } from './Icon';
import { ContextMenu, MenuItem } from './Menu';

const row =
  'group flex h-7 w-full cursor-default items-center gap-2 rounded-control px-2 text-left text-sm outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus aria-[current=true]:bg-highlight aria-[current=true]:text-accent';
const heading = 'px-2 pt-4 pb-1 text-[11px] font-semibold tracking-[.06em] text-muted uppercase';

type Filter = { id: FilterId; label: string; icon: Component<LucideProps> };

/** Status filters, in the web UI's order. */
const STATUS: Filter[] = [
  { id: 'all', label: 'All', icon: Layers },
  { id: 'active', label: 'Active', icon: Zap },
  { id: 'downloading', label: 'Downloading', icon: ArrowDown },
  { id: 'seeding', label: 'Seeding', icon: ArrowUp },
  { id: 'paused', label: 'Paused', icon: CirclePause },
  { id: 'finished', label: 'Finished', icon: CircleCheck },
  { id: 'error', label: 'Error', icon: CircleAlert },
];

function Item(props: {
  icon: Component<LucideProps>;
  label: string;
  count: number;
  current: boolean;
  italic?: boolean;
  onClick(): void;
  onContextMenu?(e: MouseEvent): void;
}) {
  return (
    <button
      class={row}
      aria-current={props.current}
      onClick={() => props.onClick()}
      onContextMenu={(e) => props.onContextMenu?.(e)}
    >
      <Icon of={props.icon} class="text-muted group-aria-[current=true]:text-accent" />
      <span class="min-w-0 flex-1 truncate" classList={{ italic: props.italic }}>
        {props.label}
      </span>
      <span class="text-xs text-faint tabular-nums">{props.count}</span>
    </button>
  );
}

/**
 * Left sidebar: status filters and labels, with live counts. A status and a
 * label combine (seeding torrents labelled "linux"); choosing the current
 * label again clears it.
 */
export function Sidebar() {
  const labels = allLabels;
  const unlabeled = () => torrents().filter((t) => !t.labels?.length).length;
  const toggleLabel = (label: string) => setLabelFilter(labelFilter() === label ? null : label);
  const [menu, setMenu] = createSignal<{ x: number; y: number; label: string } | null>(null);

  return (
    <nav class="flex w-52 flex-none flex-col overflow-y-auto border-r border-line bg-panel p-2" aria-label="Filters">
      <div class={heading}>Status</div>
      <For each={STATUS}>
        {(f) => (
          <Item
            icon={f.icon}
            label={f.label}
            count={torrents().filter(FILTERS[f.id]).length}
            current={statusFilter() === f.id}
            onClick={() => setStatusFilter(f.id)}
          />
        )}
      </For>
      <div class={`${heading} flex items-center justify-between`}>
        Labels
        <button
          class="grid size-6 place-items-center rounded-button text-muted normal-case hover:bg-hover hover:text-fg aria-pressed:bg-highlight aria-pressed:text-accent"
          aria-pressed={groupByLabel()}
          title={groupByLabel() ? 'Stop grouping the table by label' : 'Group the table by label'}
          aria-label="Group by label"
          onClick={() => setGroupByLabel(!groupByLabel())}
        >
          <Icon of={Rows3} size={14} />
        </button>
      </div>
      <Show when={labels().length} fallback={<p class="px-2 text-xs text-faint">No labels yet.</p>}>
        <For each={labels()}>
          {([name, count]) => (
            <Item
              icon={Tag}
              label={name}
              count={count}
              current={labelFilter() === name}
              onClick={() => toggleLabel(name)}
              onContextMenu={(e) => {
                e.preventDefault();
                setMenu({ x: e.clientX, y: e.clientY, label: name });
              }}
            />
          )}
        </For>
        <Show when={unlabeled()}>
          <Item
            icon={TagsIcon}
            label="No label"
            italic
            count={unlabeled()}
            current={labelFilter() === ''}
            onClick={() => toggleLabel('')}
          />
        </Show>
      </Show>
      <Show when={menu()} keyed>
        {(m) => (
          <ContextMenu label={m.label} x={m.x} y={m.y} onClose={() => setMenu(null)}>
            <MenuItem onSelect={() => void renameLabel(m.label)}>Rename…</MenuItem>
            <MenuItem danger onSelect={() => void deleteLabel(m.label)}>
              Delete…
            </MenuItem>
          </ContextMenu>
        )}
      </Show>
    </nav>
  );
}
