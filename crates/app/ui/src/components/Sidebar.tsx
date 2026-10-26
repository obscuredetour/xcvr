import { createSignal, For, Show } from 'solid-js';
import type { Component, JSX, ParentProps } from 'solid-js';
import type { LucideProps } from 'lucide-solid';
import ArrowDown from 'lucide-solid/icons/arrow-down';
import ArrowUp from 'lucide-solid/icons/arrow-up';
import ChevronRight from 'lucide-solid/icons/chevron-right';
import CircleAlert from 'lucide-solid/icons/circle-alert';
import CircleCheck from 'lucide-solid/icons/circle-check';
import CirclePause from 'lucide-solid/icons/circle-pause';
import Folder from 'lucide-solid/icons/folder';
import FolderDown from 'lucide-solid/icons/folder-down';
import Layers from 'lucide-solid/icons/layers';
import Rows3 from 'lucide-solid/icons/rows-3';
import Tag from 'lucide-solid/icons/tag';
import TagsIcon from 'lucide-solid/icons/tags';
import Zap from 'lucide-solid/icons/zap';

import { allFolders, isDefaultFolder } from '../folders';
import { FILTERS, type FilterId } from '../format';
import { allLabels, deleteLabel, renameLabel } from '../labels';
import { torrents } from '../torrents';
import {
  folderFilter,
  foldedSections,
  groupBy,
  type GroupBy,
  labelFilter,
  setFolderFilter,
  setGroupBy,
  setLabelFilter,
  setStatusFilter,
  statusFilter,
  toggleSection,
} from '../view';
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
  title?: string;
  onClick(): void;
  onContextMenu?(e: MouseEvent): void;
}) {
  return (
    <button
      class={row}
      aria-current={props.current}
      title={props.title}
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

/** The button beside a heading that groups the table by its kind (or stops grouping). */
function GroupButton(props: { by: Exclude<GroupBy, null>; noun: string }) {
  const on = () => groupBy() === props.by;
  return (
    <button
      class="grid size-6 place-items-center rounded-button text-muted normal-case hover:bg-hover hover:text-fg aria-pressed:bg-highlight aria-pressed:text-accent"
      aria-pressed={on()}
      title={on() ? 'Stop grouping the table' : `Group the table by ${props.noun}`}
      aria-label={`Group by ${props.noun}`}
      onClick={() => setGroupBy(on() ? null : props.by)}
    >
      <Icon of={Rows3} size={14} />
    </button>
  );
}

/**
 * A sidebar section under a heading that folds it (remembered). Folded, it
 * still shows `folded`: the item that's filtering the table, if any.
 */
function Section(props: ParentProps<{ id: string; title: string; aside?: JSX.Element; folded?: JSX.Element }>) {
  const isFolded = () => foldedSections().has(props.id);
  return (
    <section>
      <div class={`${heading} flex items-center justify-between`}>
        <button
          class="group/fold flex items-center gap-1 rounded-inset uppercase outline-none hover:text-fg focus-visible:ring-2 focus-visible:ring-focus"
          aria-expanded={!isFolded()}
          title={isFolded() ? `Show ${props.title.toLowerCase()}` : `Hide ${props.title.toLowerCase()}`}
          onClick={() => toggleSection(props.id)}
        >
          {props.title}
          <Icon
            of={ChevronRight}
            size={12}
            class={`transition-[transform,opacity] ${isFolded() ? '' : 'rotate-90 opacity-0 group-hover/fold:opacity-100 group-focus-visible/fold:opacity-100'}`}
          />
        </button>
        {props.aside}
      </div>
      <Show when={!isFolded()} fallback={props.folded}>
        {props.children}
      </Show>
    </section>
  );
}

/**
 * Left sidebar: status filters, labels and download folders, with live counts.
 * They combine (seeding torrents labelled "linux" in "isos"); choosing the
 * current label or folder again clears it.
 */
export function Sidebar() {
  const labels = allLabels;
  const unlabeled = () => torrents().filter((t) => !t.labels?.length).length;
  const toggleLabel = (label: string) => setLabelFilter(labelFilter() === label ? null : label);
  const toggleFolder = (path: string) => setFolderFilter(folderFilter() === path ? null : path);
  const [menu, setMenu] = createSignal<{ x: number; y: number; label: string } | null>(null);
  const labelCount = (label: string) => labels().find(([name]) => name === label)?.[1] ?? 0;

  const statusItem = (f: Filter) => (
    <Item
      icon={f.icon}
      label={f.label}
      count={torrents().filter(FILTERS[f.id]).length}
      current={statusFilter() === f.id}
      onClick={() => setStatusFilter(f.id)}
    />
  );
  const labelItem = ([name, count]: [string, number]) => (
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
  );
  const noLabelItem = () => (
    <Item
      icon={TagsIcon}
      label="No label"
      italic
      count={unlabeled()}
      current={labelFilter() === ''}
      onClick={() => toggleLabel('')}
    />
  );
  const folderItem = (f: { path: string; name: string; count: number }) => (
    <Item
      icon={isDefaultFolder(f.path) ? FolderDown : Folder}
      label={f.name}
      title={f.path}
      count={f.count}
      current={folderFilter() === f.path}
      onClick={() => toggleFolder(f.path)}
    />
  );

  return (
    <nav class="flex w-52 flex-none flex-col overflow-y-auto border-r border-line bg-panel p-2" aria-label="Filters">
      <Section
        id="status"
        title="Status"
        folded={<For each={STATUS.filter((f) => f.id !== 'all' && f.id === statusFilter())}>{statusItem}</For>}
      >
        <For each={STATUS}>{statusItem}</For>
      </Section>

      <Section
        id="labels"
        title="Labels"
        aside={<GroupButton by="label" noun="label" />}
        folded={
          <Show when={labelFilter() !== null}>
            {labelFilter() === '' ? noLabelItem() : labelItem([labelFilter()!, labelCount(labelFilter()!)])}
          </Show>
        }
      >
        <Show when={labels().length} fallback={<p class="px-2 text-xs text-faint">No labels yet.</p>}>
          <For each={labels()}>{labelItem}</For>
          <Show when={unlabeled()}>{noLabelItem()}</Show>
        </Show>
      </Section>

      <Section
        id="folders"
        title="Folders"
        aside={<GroupButton by="folder" noun="folder" />}
        folded={<For each={allFolders().filter((f) => f.path === folderFilter())}>{folderItem}</For>}
      >
        <Show when={allFolders().length} fallback={<p class="px-2 text-xs text-faint">No torrents yet.</p>}>
          <For each={allFolders()}>{folderItem}</For>
        </Show>
      </Section>
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
