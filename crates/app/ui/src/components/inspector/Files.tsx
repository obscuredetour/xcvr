import { createMemo, createSignal, Index, on, Show, createEffect } from 'solid-js';
import ChevronRight from 'lucide-solid/icons/chevron-right';
import FileIcon from 'lucide-solid/icons/file';
import FolderIcon from 'lucide-solid/icons/folder';

import { openHere, setRenaming } from '../../actions';
import { api, errorMessage, type Torrent, type TorrentSet } from '../../api';
import { buildTree, type FileNode, type PriorityValue, visibleRows } from '../../filetree';
import { formatPercent, formatSize } from '../../format';
import { toast } from '../../toast';
import { Icon } from '../Icon';
import { ContextMenu, MenuItem, MenuSeparator } from '../Menu';

const PRIORITIES: [string, string][] = [
  ['1', 'High'],
  ['0', 'Normal'],
  ['-1', 'Low'],
];

/** The torrent's files as a tree: what's downloaded, at what priority. */
export function FilesTab(props: { t: Torrent }) {
  const hash = () => props.t.hash_string!;
  const tree = createMemo(() => buildTree(props.t.files ?? [], props.t.file_stats ?? []));
  const [expanded, setExpanded] = createSignal<ReadonlySet<string>>(new Set());
  // Another torrent starts folded.
  createEffect(
    on(
      hash,
      () => {
        setExpanded(new Set<string>());
      },
      { defer: true },
    ),
  );
  const rows = () => visibleRows(tree(), expanded());
  const [menu, setMenu] = createSignal<{ x: number; y: number; node: FileNode } | null>(null);

  const toggle = (path: string) => {
    const next = new Set(expanded());
    if (next.has(path)) next.delete(path);
    else next.add(path);
    setExpanded(next);
  };

  const change = async (set: TorrentSet) => {
    try {
      await api.setTorrents([hash()], set);
    } catch (err) {
      toast(errorMessage(err), { error: true });
    }
  };
  const want = (node: FileNode, wanted: boolean) =>
    change(wanted ? { files_wanted: node.indices } : { files_unwanted: node.indices });
  const prioritize = (node: FileNode, p: string) =>
    change(
      p === '1'
        ? { priority_high: node.indices }
        : p === '-1'
          ? { priority_low: node.indices }
          : { priority_normal: node.indices },
    );

  return (
    <Show
      when={props.t.files?.length}
      fallback={<p class="p-5 text-xs text-muted">No files yet: the torrent’s metadata is still coming from peers.</p>}
    >
      <ul class="py-1" role="tree" aria-label="Files">
        {/* Index: the tree is rebuilt on every update; rows are reused by position. */}
        <Index each={rows()}>
          {(row) => (
            <FileRow
              node={row().node}
              depth={row().depth}
              open={expanded().has(row().node.path)}
              onToggle={() => toggle(row().node.path)}
              onWant={(w) => void want(row().node, w)}
              onPriority={(p) => void prioritize(row().node, p)}
              onMenu={(e) => {
                e.preventDefault();
                setMenu({ x: e.clientX, y: e.clientY, node: row().node });
              }}
            />
          )}
        </Index>
      </ul>
      <Show when={menu()} keyed>
        {(m) => (
          <ContextMenu label={m.node.name} x={m.x} y={m.y} onClose={() => setMenu(null)}>
            <Show when={m.node.index !== null}>
              <MenuItem onSelect={() => void openHere(hash(), m.node.index, false)}>Open</MenuItem>
            </Show>
            <MenuItem onSelect={() => void openHere(hash(), m.node.indices[0] ?? null, true)}>Reveal</MenuItem>
            <MenuItem onSelect={() => setRenaming({ hash: hash(), path: m.node.path, current: m.node.name })}>
              Rename…
            </MenuItem>
            <MenuSeparator />
            <MenuItem disabled={m.node.wanted === true} onSelect={() => void want(m.node, true)}>
              Download
            </MenuItem>
            <MenuItem disabled={m.node.wanted === false} onSelect={() => void want(m.node, false)}>
              Don’t Download
            </MenuItem>
          </ContextMenu>
        )}
      </Show>
    </Show>
  );
}

function FileRow(props: {
  node: FileNode;
  depth: number;
  open: boolean;
  onToggle(): void;
  onWant(wanted: boolean): void;
  onPriority(p: string): void;
  onMenu(e: MouseEvent): void;
}) {
  const n = () => props.node;
  const isFolder = () => n().index === null;
  const progress = () => (n().length ? n().done / n().length : 1);
  const priority = (): string => (n().priority === 'mixed' ? 'mixed' : String(n().priority as PriorityValue));

  return (
    <li
      role="treeitem"
      aria-expanded={isFolder() ? props.open : undefined}
      class="flex items-center gap-2 py-1 pr-3 hover:bg-hover"
      classList={{ 'opacity-60': n().wanted === false }}
      style={{ 'padding-left': `${12 + props.depth * 16}px` }}
      onContextMenu={(e) => props.onMenu(e)}
    >
      <button
        class="grid size-4 flex-none place-items-center text-muted"
        classList={{ invisible: !isFolder() }}
        aria-label={props.open ? 'Fold' : 'Unfold'}
        tabIndex={isFolder() ? 0 : -1}
        onClick={() => props.onToggle()}
      >
        <Icon
          of={ChevronRight}
          size={12}
          class={props.open ? 'rotate-90 transition-transform' : 'transition-transform'}
        />
      </button>
      <input
        type="checkbox"
        class="flex-none"
        aria-label={`Download ${n().name}`}
        checked={n().wanted === true}
        ref={(el) => {
          // A folder with some files wanted and some not.
          createEffect(() => (el.indeterminate = n().wanted === 'mixed'));
        }}
        onChange={(e) => props.onWant(e.currentTarget.checked)}
      />
      <Icon of={isFolder() ? FolderIcon : FileIcon} size={14} class="flex-none text-muted" />
      <div class="min-w-0 flex-1" onDblClick={() => isFolder() && props.onToggle()}>
        <div class="truncate text-xs" title={n().path}>
          {n().name}
        </div>
        <div class="truncate text-[11px] text-muted tabular-nums">
          {formatSize(n().length)} · {formatPercent(progress())}
        </div>
      </div>
      <select
        class="h-6 flex-none rounded-inset border border-line bg-field px-1 text-[11px] text-muted outline-none focus:border-focus"
        aria-label={`Priority of ${n().name}`}
        value={priority()}
        onChange={(e) => props.onPriority(e.currentTarget.value)}
      >
        <Show when={priority() === 'mixed'}>
          <option value="mixed" disabled>
            Mixed
          </option>
        </Show>
        {PRIORITIES.map(([value, label]) => (
          <option value={value}>{label}</option>
        ))}
      </select>
    </li>
  );
}
