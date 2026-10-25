// A torrent's files as a tree: Transmission lists them flat, with `/`-separated
// paths. Folders roll up their files: size, progress, whether they're wanted
// (all, none, or mixed) and their priority (one, or mixed).

import type { File } from './bindings/File';
import type { FileStat } from './bindings/FileStat';

export type Wanted = boolean | 'mixed';
/** -1 low, 0 normal, 1 high, or mixed. */
export type PriorityValue = -1 | 0 | 1 | 'mixed';

export type FileNode = {
  name: string;
  /** Its path as Transmission knows it (for renaming): the torrent's folder first. */
  path: string;
  /** The files it is or holds, as indices into the torrent's files. */
  indices: number[];
  /** Set for a file: its index. */
  index: number | null;
  children: FileNode[];
  length: number;
  done: number;
  wanted: Wanted;
  priority: PriorityValue;
};

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

/**
 * The tree. When every file sits in one top folder (the torrent's own), its
 * contents are the top level: the inspector's title already names it.
 */
export function buildTree(files: readonly File[], stats: readonly FileStat[]): FileNode[] {
  const root: FileNode = folder('', '');
  files.forEach((f, i) => {
    const parts = f.name.split('/').filter(Boolean);
    let at = root;
    parts.slice(0, -1).forEach((part, depth) => {
      const path = parts.slice(0, depth + 1).join('/');
      let next = at.children.find((c) => c.index === null && c.name === part);
      if (!next) {
        next = folder(part, path);
        at.children.push(next);
      }
      at = next;
    });
    const s = stats[i];
    at.children.push({
      name: parts.at(-1) ?? f.name,
      path: f.name,
      indices: [i],
      index: i,
      children: [],
      length: f.length,
      done: s?.bytes_completed ?? f.bytes_completed,
      wanted: s?.wanted ?? true,
      priority: (s?.priority ?? 0) as -1 | 0 | 1,
    });
  });
  rollUp(root);
  const top = root.children;
  return top.length === 1 && top[0]!.index === null ? top[0]!.children : top;
}

function folder(name: string, path: string): FileNode {
  return { name, path, indices: [], index: null, children: [], length: 0, done: 0, wanted: true, priority: 0 };
}

function rollUp(node: FileNode): void {
  if (node.index !== null) return;
  node.children.forEach(rollUp);
  node.children.sort((a, b) =>
    a.index === null && b.index !== null
      ? -1
      : a.index !== null && b.index === null
        ? 1
        : collator.compare(a.name, b.name),
  );
  node.indices = node.children.flatMap((c) => c.indices);
  node.length = node.children.reduce((n, c) => n + c.length, 0);
  node.done = node.children.reduce((n, c) => n + c.done, 0);
  node.wanted = same(node.children.map((c) => c.wanted)) ?? 'mixed';
  node.priority = same(node.children.map((c) => c.priority)) ?? 'mixed';
}

/** The one value they all have, or null when they differ (or include 'mixed'). */
function same<T>(values: T[]): T | null {
  const first = values[0];
  return first !== undefined && first !== 'mixed' && values.every((v) => v === first) ? first : null;
}

/** The rows to show: every node at the top, and the insides of expanded folders. */
export function visibleRows(
  nodes: readonly FileNode[],
  expanded: ReadonlySet<string>,
  depth = 0,
): { node: FileNode; depth: number }[] {
  return nodes.flatMap((node) => [
    { node, depth },
    ...(node.index === null && expanded.has(node.path) ? visibleRows(node.children, expanded, depth + 1) : []),
  ]);
}
