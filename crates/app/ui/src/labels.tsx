// Labels: Transmission's per-torrent tags, kept on the host so both computers
// see the same ones. Everything that changes them comes here; the Rust side
// works out each torrent's new list (labels.rs).

import { createMemo, createRoot } from 'solid-js';

import { api, errorMessage } from './api';
import { confirm } from './confirm';
import { askUser } from './question';
import { toast } from './toast';
import { torrent, torrents } from './torrents';
import { labelFilter, setLabelFilter } from './view';

/** Every label in use, A to Z, with how many torrents have it. */
export const allLabels = createRoot(() =>
  createMemo((): [string, number][] => {
    const counts = new Map<string, number>();
    for (const t of torrents()) for (const l of t.labels ?? []) counts.set(l, (counts.get(l) ?? 0) + 1);
    return [...counts].sort(([a], [b]) => a.localeCompare(b, undefined, { sensitivity: 'base', numeric: true }));
  }),
);

/** Labels typed with commas between them, tidied. */
export const splitLabels = (text: string) => [
  ...new Set(
    text
      .split(',')
      .map((l) => l.trim())
      .filter(Boolean),
  ),
];

/** How many of `hashes` have each label. */
export function labelCounts(hashes: readonly string[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const h of hashes) for (const l of torrent(h)?.labels ?? []) counts.set(l, (counts.get(l) ?? 0) + 1);
  return counts;
}

export async function editLabels(hashes: string[], add: string[], remove: string[] = []): Promise<void> {
  if (!hashes.length || (!add.length && !remove.length)) return;
  try {
    await api.editLabels(hashes, add, remove);
  } catch (err) {
    toast(errorMessage(err), { error: true });
  }
}

/** Ask for a new label (or several, with commas) and add it to `hashes`. */
export async function addLabelTo(hashes: string[]): Promise<void> {
  const answer = await askUser({
    title: 'Add Label',
    question:
      hashes.length === 1
        ? `A label for “${torrent(hashes[0]!)?.name ?? 'this torrent'}”. Separate several with commas.`
        : `A label for ${hashes.length} torrents. Separate several with commas.`,
    choices: [],
    placeholder: 'linux, iso',
  });
  if (answer && 'text' in answer) await editLabels(hashes, splitLabels(answer.text));
}

/** Rename a label everywhere. */
export async function renameLabel(from: string): Promise<void> {
  const answer = await askUser({
    title: 'Rename Label',
    question: `A new name for “${from}”, on every torrent that has it. A name already in use merges them.`,
    choices: [],
    placeholder: from,
    initial: from,
  });
  const to = answer && 'text' in answer ? answer.text.trim() : '';
  if (!to || to === from) return;
  try {
    const { torrents: n } = await api.renameLabel(from, to);
    if (labelFilter() === from) setLabelFilter(to);
    toast(`Renamed on ${n} ${n === 1 ? 'torrent' : 'torrents'}.`);
  } catch (err) {
    toast(errorMessage(err), { error: true });
  }
}

/** Take a label off every torrent, after asking. */
export async function deleteLabel(label: string): Promise<void> {
  const n = torrents().filter((t) => t.labels?.includes(label)).length;
  const ok = await confirm({
    title: `Delete “${label}”?`,
    message: () => (
      <p>
        {n === 1
          ? 'It comes off the one torrent that has it. The torrent and its other labels stay.'
          : `It comes off all ${n} torrents that have it. The torrents and their other labels stay.`}
      </p>
    ),
    confirm: 'Delete Label',
    danger: true,
  });
  if (!ok) return;
  try {
    await api.deleteLabel(label);
    if (labelFilter() === label) setLabelFilter(null);
  } catch (err) {
    toast(errorMessage(err), { error: true });
  }
}
