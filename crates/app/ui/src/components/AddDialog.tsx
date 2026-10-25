import { createEffect, createMemo, createResource, createSignal, For, on, onCleanup, onMount, Show } from 'solid-js';
import ChevronDown from 'lucide-solid/icons/chevron-down';
import FileIcon from 'lucide-solid/icons/file';
import Lock from 'lucide-solid/icons/lock';
import Magnet from 'lucide-solid/icons/magnet';
import Star from 'lucide-solid/icons/star';
import X from 'lucide-solid/icons/x';

import { chooseTorrentFiles, files, linksIn, linksText, setFiles, setLinksText } from '../adder';
import { type AddSource, api, asProblem, errorMessage, type Problem } from '../api';
import { primaryButton, secondaryButton, smallButton } from '../buttons';
import {
  favouriteFolders,
  isFavourite,
  normalizeFolder,
  recentFolders,
  rememberFolder,
  toggleFavourite,
} from '../folders';
import { formatSize } from '../format';
import { keys } from '../keys';
import { toast } from '../toast';
import { torrents } from '../torrents';
import { Toggle } from './settings/parts';
import { Icon } from './Icon';
import { Menu, MenuItem, MenuLabel, MenuSeparator } from './Menu';

const field =
  'h-8 w-full rounded-control border border-line bg-field px-2.5 text-sm outline-none select-text focus:border-focus focus:ring-3 focus:ring-focus/20';
const sectionLabel = 'mb-1.5 text-xs font-semibold text-muted';

const basename = (path: string) => path.split(/[\\/]/).pop() ?? path;

/** A link as people read it: a magnet's display name, or a URL's file name. */
export function linkName(link: string): string {
  if (/^magnet:/i.test(link)) {
    const dn = /[?&]dn=([^&]+)/i.exec(link)?.[1];
    if (dn) {
      try {
        return decodeURIComponent(dn.replace(/\+/g, ' '));
      } catch {
        return dn;
      }
    }
    const hash = /btih:([a-z0-9]+)/i.exec(link)?.[1];
    return hash ? `Magnet ${hash.slice(0, 12).toLowerCase()}…` : 'Magnet link';
  }
  try {
    return decodeURIComponent(new URL(link).pathname.split('/').pop() || link);
  } catch {
    return link;
  }
}

/**
 * Add torrents: the .torrent files and links gathered so far (adder.tsx), and
 * where they go, their labels and whether they start. Mount to open.
 */
export function AddDialog(props: { onClose(): void }) {
  let dialog!: HTMLDialogElement;
  onMount(() => dialog.showModal());

  const [defaults] = createResource(() => api.addDefaults().catch(() => null));
  /** The folder typed or chosen; null keeps the host's default. */
  const [chosenDir, setChosenDir] = createSignal<string | null>(null);
  const folder = () => chosenDir() ?? defaults()?.downloadDir ?? '';
  /** Start right away; null follows the host's setting. */
  const [startChoice, setStartChoice] = createSignal<boolean | null>(null);
  const starts = () => startChoice() ?? defaults()?.startAdded ?? true;
  const [labelsText, setLabelsText] = createSignal('');
  const [busy, setBusy] = createSignal(false);
  const [failures, setFailures] = createSignal<{ source: string; message: string }[]>([]);

  const links = () => linksIn(linksText());
  const count = () => files().length + links().length;
  const labelList = () => [
    ...new Set(
      labelsText()
        .split(',')
        .map((l) => l.trim())
        .filter(Boolean),
    ),
  ];

  // Labels already in use, for one-click adding.
  const knownLabels = createMemo(() => [...new Set(torrents().flatMap((t) => t.labels ?? []))].sort());
  const addLabel = (label: string) => {
    if (!labelList().includes(label)) setLabelsText([...labelList(), label].join(', '));
  };

  // Free space in the folder, asked for once typing pauses.
  const [settled, setSettled] = createSignal('');
  let timer: ReturnType<typeof setTimeout> | undefined;
  createEffect(
    on(folder, (f) => {
      clearTimeout(timer);
      timer = setTimeout(() => setSettled(normalizeFolder(f)), 300);
    }),
  );
  onCleanup(() => clearTimeout(timer));
  // An empty folder (the host's default not known yet) isn't asked about.
  const [space] = createResource(
    () => settled() || null,
    (path) =>
      api.freeSpace(path).then(
        (s) => ({ ok: s }) as const,
        (e: unknown) => ({ problem: asProblem(e) }) as const,
      ),
  );

  const submit = async () => {
    if (!count() || busy()) return;
    setBusy(true);
    setFailures([]);
    const sources: AddSource[] = [
      ...files().map((path) => ({ kind: 'file' as const, path })),
      ...links().map((link) => ({ kind: 'link' as const, link })),
    ];
    try {
      const results = await api.addTorrents(sources, {
        downloadDir: chosenDir() === null ? null : normalizeFolder(folder()),
        paused: !starts(),
        labels: labelList(),
      });
      const failed: { source: AddSource; message: string }[] = [];
      const added: string[] = [];
      const already: string[] = [];
      results.forEach((r, i) => {
        if (r.outcome === 'failed') failed.push({ source: sources[i]!, message: r.message });
        else (r.outcome === 'added' ? added : already).push(r.name);
      });
      if (chosenDir() !== null && (added.length || already.length)) rememberFolder(folder());

      const done = [
        added.length ? (added.length === 1 ? `Added “${added[0]}”` : `Added ${added.length} torrents`) : '',
        already.length
          ? already.length === 1
            ? `“${already[0]}” was already there`
            : `${already.length} were already there`
          : '',
      ].filter(Boolean);
      if (!failed.length) {
        props.onClose();
        if (done.length) toast(`${done.join('. ')}.`);
        return;
      }
      // Keep what failed, with why, so it can be fixed and tried again.
      setFiles(failed.flatMap((f) => (f.source.kind === 'file' ? [f.source.path] : [])));
      setLinksText(failed.flatMap((f) => (f.source.kind === 'link' ? [f.source.link] : [])).join('\n'));
      setFailures(
        failed.map((f) => ({
          source: f.source.kind === 'file' ? basename(f.source.path) : linkName(f.source.link),
          message: f.message,
        })),
      );
      if (done.length) toast(`${done.join('. ')}.`);
    } catch (err) {
      toast(errorMessage(err), { error: true });
    } finally {
      setBusy(false);
    }
  };

  return (
    <dialog
      ref={dialog}
      aria-label="Add torrents"
      class="m-auto max-h-[calc(100vh-48px)] w-[min(600px,calc(100vw-48px))] animate-menu-in overflow-y-auto rounded-dialog border border-line bg-popover p-6 text-fg shadow-lift"
      onCancel={(e) => {
        e.preventDefault();
        if (!busy()) props.onClose();
      }}
      onKeyDown={(e) => {
        if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
          e.preventDefault();
          void submit();
        }
      }}
    >
      <h2 class="text-lg font-semibold">Add Torrents</h2>

      {/* What's being added. */}
      <div class="mt-4">
        <div class={sectionLabel}>Torrent files</div>
        <Show
          when={files().length}
          fallback={<p class="text-xs text-faint">None yet. Choose some, or drop them on the window.</p>}
        >
          <ul class="divide-y divide-line rounded-card border border-line">
            <For each={files()}>
              {(path) => <FileRow path={path} onRemove={() => setFiles(files().filter((f) => f !== path))} />}
            </For>
          </ul>
        </Show>
        <button class={`mt-2 ${smallButton}`} onClick={() => void chooseTorrentFiles()}>
          Choose .torrent Files… <span class="text-faint">{keys('mod+O')}</span>
        </button>
      </div>

      <div class="mt-4">
        <label class={sectionLabel} for="add-links">
          Magnet links or URLs, one per line
        </label>
        <textarea
          id="add-links"
          class="block min-h-16 w-full resize-y rounded-control border border-line bg-field px-2.5 py-2 font-mono text-xs outline-none select-text focus:border-focus focus:ring-3 focus:ring-focus/20"
          rows={3}
          spellcheck={false}
          placeholder="magnet:?xt=urn:btih:…"
          value={linksText()}
          onInput={(e) => setLinksText(e.currentTarget.value)}
        />
        <Show when={links().length}>
          <ul class="mt-1.5 space-y-0.5 text-xs text-muted">
            <For each={links()}>
              {(link) => (
                <li class="flex min-w-0 items-center gap-1.5">
                  <Icon of={Magnet} size={12} class="flex-none" />
                  <span class="truncate">{linkName(link)}</span>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </div>

      {/* Where they go. */}
      <div class="mt-5">
        <label class={sectionLabel} for="add-folder">
          Download to (a folder on the host)
        </label>
        <div class="flex items-center gap-1.5">
          <input
            id="add-folder"
            class={field}
            spellcheck={false}
            autocomplete="off"
            value={folder()}
            placeholder={defaults.loading ? 'Asking the host…' : '/path/on/the/host'}
            onInput={(e) => setChosenDir(e.currentTarget.value)}
          />
          <button
            class="grid size-8 flex-none place-items-center rounded-button text-muted hover:bg-hover hover:text-fg"
            title={isFavourite(folder()) ? 'Remove from favourites' : 'Add to favourites'}
            aria-label="Favourite folder"
            aria-pressed={isFavourite(folder())}
            disabled={!folder().trim()}
            onClick={() => toggleFavourite(folder())}
          >
            <Icon of={Star} size={16} class={isFavourite(folder()) ? 'fill-star text-star' : ''} />
          </button>
          <FolderMenu defaultDir={defaults()?.downloadDir ?? ''} onChoose={setChosenDir} />
        </div>
        <p class="mt-1 text-xs text-muted" role="status">
          <Show when={space()} fallback={settled() ? 'Checking free space…' : ''}>
            {(s) => {
              const result = s();
              return 'ok' in result
                ? `${formatSize(result.ok.size_bytes)} free of ${formatSize(result.ok.total_size)}`
                : missingOrProblem(result.problem);
            }}
          </Show>
        </p>
      </div>

      {/* Labels and starting. */}
      <div class="mt-5">
        <label class={sectionLabel} for="add-labels">
          Labels, separated by commas
        </label>
        <input
          id="add-labels"
          class={field}
          spellcheck={false}
          autocomplete="off"
          placeholder="linux, iso"
          value={labelsText()}
          onInput={(e) => setLabelsText(e.currentTarget.value)}
        />
        <Show when={knownLabels().filter((l) => !labelList().includes(l)).length}>
          <div class="mt-1.5 flex flex-wrap gap-1">
            <For each={knownLabels().filter((l) => !labelList().includes(l))}>
              {(label) => (
                <button
                  class="rounded-chip bg-element px-2 py-0.5 text-[11px] font-medium text-muted hover:bg-element-hover hover:text-fg"
                  title={`Add “${label}”`}
                  onClick={() => addLabel(label)}
                >
                  + {label}
                </button>
              )}
            </For>
          </div>
        </Show>
      </div>

      <div class="mt-5 flex items-center justify-between gap-4">
        <div>
          <div class="text-sm font-medium">Start right away</div>
          <div class="text-xs text-muted">Otherwise they’re added paused.</div>
        </div>
        <Toggle label="Start right away" checked={starts()} onChange={setStartChoice} />
      </div>

      <Show when={failures().length}>
        <ul class="mt-5 space-y-1 rounded-card border border-danger/30 bg-danger/8 p-3 text-xs" role="alert">
          <For each={failures()}>
            {(f) => (
              <li>
                <span class="font-medium">{f.source}</span>: <span class="text-danger">{f.message}</span>
              </li>
            )}
          </For>
        </ul>
      </Show>

      <div class="mt-6 flex items-center justify-end gap-2">
        <button class={secondaryButton} disabled={busy()} onClick={() => props.onClose()}>
          Cancel
        </button>
        <button
          class={primaryButton}
          disabled={!count() || busy()}
          title={keys('mod+Enter')}
          onClick={() => void submit()}
        >
          {busy() ? 'Adding…' : count() > 1 ? `Add ${count()} Torrents` : 'Add Torrent'}
        </button>
      </div>
    </dialog>
  );
}

/** Free space's failure, said plainly: a folder that's not there yet is normal. */
function missingOrProblem(p: Problem): string {
  return /no such file|not found/i.test(p.message)
    ? 'That folder isn’t on the host yet; Transmission makes it when it’s first used.'
    : p.message;
}

/** One .torrent file: what it is (read on this machine), and a way to take it out. */
function FileRow(props: { path: string; onRemove(): void }) {
  const [info] = createResource(
    () => props.path,
    (path) =>
      api.readTorrentFile(path).then(
        (m) => ({ ok: m }) as const,
        (e: unknown) => ({ problem: errorMessage(e) }) as const,
      ),
  );
  const meta = () => {
    const i = info();
    return i && 'ok' in i ? i.ok : undefined;
  };
  const problem = () => {
    const i = info();
    return i && 'problem' in i ? i.problem : undefined;
  };
  const detail = () => {
    const m = meta();
    if (m) return `${formatSize(m.totalSize)} · ${m.fileCount === 1 ? '1 file' : `${m.fileCount} files`}`;
    return problem() ?? 'Reading…';
  };
  return (
    <li class="flex items-center gap-2.5 px-3 py-2">
      <Icon of={FileIcon} size={16} class="flex-none text-muted" />
      <div class="min-w-0 flex-1">
        <div class="truncate text-sm font-medium" title={props.path}>
          {meta()?.name || basename(props.path)}
        </div>
        <div class="truncate text-xs" classList={{ 'text-muted': !problem(), 'text-danger': !!problem() }}>
          {detail()}
        </div>
      </div>
      <Show when={meta()?.private}>
        <span class="flex items-center gap-1 text-[11px] text-muted" title="A private torrent: its tracker only">
          <Icon of={Lock} size={12} />
          Private
        </span>
      </Show>
      <button
        class="grid size-6 flex-none place-items-center rounded-button text-muted hover:bg-hover hover:text-fg"
        aria-label={`Don’t add ${basename(props.path)}`}
        onClick={() => props.onRemove()}
      >
        <Icon of={X} size={12} />
      </button>
    </li>
  );
}

/** The host's default (when given), favourites and recent folders, to choose from. */
export function FolderMenu(props: { defaultDir: string; onChoose(dir: string): void }) {
  const recents = () => recentFolders().filter((d) => !favouriteFolders().includes(d) && d !== props.defaultDir);
  return (
    <Menu
      label="Choose a folder"
      iconOnly
      align="end"
      buttonClass="grid size-8 flex-none place-items-center rounded-button text-muted hover:bg-hover hover:text-fg aria-expanded:bg-hover"
      panelClass="w-80"
      button={<Icon of={ChevronDown} size={16} />}
    >
      <Show when={props.defaultDir}>
        <MenuLabel>Transmission’s default</MenuLabel>
        <MenuItem onSelect={() => props.onChoose(props.defaultDir)}>
          <span class="truncate">{props.defaultDir}</span>
        </MenuItem>
      </Show>
      <Show when={favouriteFolders().length}>
        <MenuSeparator />
        <MenuLabel>Favourites</MenuLabel>
        <For each={favouriteFolders()}>
          {(dir) => (
            <MenuItem onSelect={() => props.onChoose(dir)}>
              <span class="truncate">{dir}</span>
            </MenuItem>
          )}
        </For>
      </Show>
      <Show when={recents().length}>
        <MenuSeparator />
        <MenuLabel>Recent</MenuLabel>
        <For each={recents()}>
          {(dir) => (
            <MenuItem onSelect={() => props.onChoose(dir)}>
              <span class="truncate">{dir}</span>
            </MenuItem>
          )}
        </For>
      </Show>
    </Menu>
  );
}
