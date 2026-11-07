// Renaming a torrent's folder and files for a media library: keep the names,
// or rename it as
// - a show: episodes `Show (Year) - s01e01 - Title`, its folder `Season 01`
//   (or the show's name, for several seasons), with titles from TMDB (with a
//   key) or TVmaze (without one) when the show's chosen there;
// - a movie: its folder and the film `Title (Year)`;
// - just its folder (or file).
// The backend plans the names (naming.rs); this shows the plan as it changes.
// Used by the add dialog (before adding) and the Smart Rename dialog (a torrent
// already added).

import { createEffect, createMemo, createResource, createSignal, For, on, onCleanup, Show } from 'solid-js';
import ArrowRight from 'lucide-solid/icons/arrow-right';
import Search from 'lucide-solid/icons/search';
import X from 'lucide-solid/icons/x';

import {
  api,
  errorMessage,
  type MediaKind,
  type NameOptions,
  type NamePlan,
  type PlanSource,
  type FoundTitle,
} from '../api';
import { smallButton } from '../buttons';
import { Segmented, Toggle } from './settings/parts';
import { Icon } from './Icon';

const field =
  'h-8 w-full rounded-control border border-line bg-field px-2.5 text-sm outline-none select-text focus:border-focus focus:ring-3 focus:ring-focus/20';
const sectionLabel = 'mb-1.5 text-xs font-semibold text-muted';

const lastPart = (path: string) => path.split('/').pop() ?? path;
const pad = (n: number) => String(n).padStart(2, '0');
/** "The Beverly Hillbillies (1962)". */
export const titleLabel = (s: FoundTitle) => (s.year ? `${s.name} (${s.year})` : s.name);

const isSeasonFolder = (p: string) => /^(season|series|s)[ ._-]?\d+$/i.test(p.trim()) || /^specials$/i.test(p.trim());

/**
 * The show a folder is for, when it looks like a show's: "/tv/The Beverly
 * Hillbillies (1962)" (a year) or "/tv/Show/Season 01" (a season inside it).
 * A plain downloads folder isn't one.
 */
export function showFromFolder(folder: string): string | null {
  const parts = folder.split(/[\\/]/).filter(Boolean);
  let seasonAt = -1;
  parts.forEach((p, i) => {
    if (isSeasonFolder(p)) seasonAt = i;
  });
  const show = seasonAt > 0 ? parts[seasonAt - 1] : parts.at(-1);
  if (!show || isSeasonFolder(show)) return null;
  return seasonAt > 0 || /\((19|20)\d\d\)\s*$/.test(show) ? show.trim() : null;
}

type Mode = MediaKind | 'folder';

/** What the panel holds; made by the dialog that shows it, which reads `options()` on submit. */
export function createNames(props: { source: () => PlanSource | null; defaultShow: () => string | null }) {
  const [renaming, setRenaming] = createSignal(false);
  const [modeChoice, setMode] = createSignal<Mode | null>(null);
  const [topText, setTopText] = createSignal<string | null>(null);
  const [showText, setShowText] = createSignal<string | null>(null);
  const [foundShow, setFoundShow] = createSignal<FoundTitle | null>(null);
  const [movieText, setMovieText] = createSignal<string | null>(null);
  const [tmdbMovie, setTmdbMovie] = createSignal<FoundTitle | null>(null);
  const [tag, setTag] = createSignal(false);
  const [ownFolder, setOwnFolder] = createSignal(true);
  const [plan, setPlan] = createSignal<NamePlan | null>(null);
  const [problem, setProblem] = createSignal<string | null>(null);

  // Chosen, else what the files look like (a show until the plan says).
  const mode = (): Mode => modeChoice() ?? (plan() ? (plan()!.kind ?? 'folder') : 'show');

  // Typed values win; until then, what the folder, TMDB or the files suggest.
  const show = () => showText() ?? props.defaultShow() ?? plan()?.guessedShow ?? '';
  const movie = () => movieText() ?? plan()?.guessedMovie ?? '';
  const suggestedTop = () => {
    const p = plan();
    if (mode() !== 'show' || !p || p.singleFile) return '';
    if (p.season != null) return `Season ${pad(p.season)}`;
    // A series in several seasons: the show's folder, unless it's going into one.
    if (p.seasonCount > 1 && !props.defaultShow()) return show();
    return '';
  };
  const top = () => topText() ?? suggestedTop();

  const options = createMemo<NameOptions | null>(() => {
    if (!renaming()) return null;
    const none: NameOptions = { top: null, episodes: null, movie: null };
    switch (mode()) {
      case 'show':
        return {
          ...none,
          top: top().trim() || null,
          episodes: show().trim()
            ? {
                show: show().trim(),
                tmdb: foundShow()?.source === 'tmdb' ? foundShow()!.id : null,
                tvmaze: foundShow()?.source === 'tvmaze' ? foundShow()!.id : null,
              }
            : null,
        };
      case 'movie':
        return movie().trim()
          ? {
              ...none,
              movie: { title: movie().trim(), tmdb: tmdbMovie()?.id ?? null, tag: tag(), ownFolder: ownFolder() },
            }
          : none;
      case 'folder':
        return { ...none, top: top().trim() || null };
    }
  });

  // The plan, asked for again once changes pause.
  const key = () => {
    const source = props.source();
    const opts = options();
    return source && opts ? JSON.stringify({ source, opts }) : null;
  };
  let timer: ReturnType<typeof setTimeout> | undefined;
  let asked = 0;
  createEffect(
    on(key, (k) => {
      clearTimeout(timer);
      if (!k) return;
      timer = setTimeout(() => {
        const n = ++asked;
        const { source, opts } = JSON.parse(k) as { source: PlanSource; opts: NameOptions };
        api.namePlan(source, opts).then(
          (p) => {
            if (n !== asked) return;
            setPlan(p);
            setProblem(null);
          },
          (e: unknown) => {
            if (n !== asked) return;
            setPlan(null);
            setProblem(errorMessage(e));
          },
        );
      }, 250);
    }),
  );
  onCleanup(() => clearTimeout(timer));

  return {
    renaming,
    setRenaming,
    mode,
    setMode,
    top,
    setTopText,
    show,
    setShowText,
    foundShow,
    setFoundShow,
    movie,
    setMovieText,
    tmdbMovie,
    setTmdbMovie,
    tag,
    setTag,
    ownFolder,
    setOwnFolder,
    plan,
    problem,
    options,
  };
}

export type Names = ReturnType<typeof createNames>;

const MODES = [
  ['show', 'TV Show'],
  ['movie', 'Movie'],
  ['folder', 'Just the Name'],
] as const;

/** The Names section: keep or rename, how, the fields, and the plan. */
export function NamesPanel(props: { names: Names; title?: string; hideToggle?: boolean }) {
  const n = () => props.names;
  const single = () => n().plan()?.singleFile ?? false;

  return (
    <div>
      <div class="flex items-center justify-between gap-4" hidden={props.hideToggle}>
        <div>
          <div class="text-sm font-medium">{props.title ?? 'Rename'}</div>
          <div class="text-xs text-muted">
            {n().renaming() ? 'Renamed on the host once it’s added.' : 'Keeps the names it comes with.'}
          </div>
        </div>
        <Toggle label="Rename" checked={n().renaming()} onChange={(on) => n().setRenaming(on)} />
      </div>

      <Show when={n().renaming()}>
        <div class="space-y-3" classList={{ 'mt-3': !props.hideToggle }}>
          <div class="flex">
            <Segmented label="Rename as" value={n().mode()} options={MODES} onChange={(m) => n().setMode(m)} />
          </div>

          <Show when={n().mode() === 'show'}>
            <div class="text-xs text-muted">Episodes as Show (Year) - s01e01 - Title, for Plex, Jellyfin and Kodi.</div>
            <Show when={!single()}>
              <TopField names={n()} label="Folder name" />
            </Show>
            <TitleField
              kind="show"
              label="Show"
              placeholder="The Beverly Hillbillies (1962)"
              value={n().show()}
              onInput={(v) => n().setShowText(v)}
              chosen={n().foundShow()}
              onChoose={(s) => {
                n().setFoundShow(s);
                if (s) n().setShowText(titleLabel(s));
              }}
              chosenText="Titles from"
              noKeyText="Titles come from the file names, or from TVmaze once you find the show there."
            />
          </Show>

          <Show when={n().mode() === 'movie'}>
            <div class="text-xs text-muted">
              The folder and the film as Title (Year), its subtitles too; extras and samples keep their names.
            </div>
            <TitleField
              kind="movie"
              label="Movie"
              placeholder="The Matrix (1999)"
              value={n().movie()}
              onInput={(v) => n().setMovieText(v)}
              chosen={n().tmdbMovie()}
              onChoose={(s) => {
                n().setTmdbMovie(s);
                if (s) n().setMovieText(titleLabel(s));
              }}
              chosenText="On"
              noKeyText="With a TMDB key (Settings › General), you can look the movie up there."
            />
            <Show when={n().tmdbMovie()}>
              <ToggleRow
                label="TMDB ID in the folder name"
                description="Adds {tmdb-…}, so Plex matches the right film."
                checked={n().tag()}
                onChange={(on) => n().setTag(on)}
              />
            </Show>
            <Show when={single()}>
              <ToggleRow
                label="In a folder of its own"
                description="It’s one file; this moves it into a Title (Year) folder in its download folder."
                checked={n().ownFolder()}
                onChange={(on) => n().setOwnFolder(on)}
              />
            </Show>
          </Show>

          <Show when={n().mode() === 'folder'}>
            <TopField names={n()} label={single() ? 'File name' : 'Folder name'} />
          </Show>

          <Preview names={n()} />
        </div>
      </Show>
    </div>
  );
}

function TopField(props: { names: Names; label: string }) {
  return (
    <div>
      <label class={sectionLabel} for="names-top">
        {props.label}
      </label>
      <input
        id="names-top"
        class={field}
        spellcheck={false}
        autocomplete="off"
        placeholder={props.names.plan()?.name ?? 'Keep its name'}
        value={props.names.top()}
        onInput={(e) => props.names.setTopText(e.currentTarget.value)}
      />
    </div>
  );
}

function ToggleRow(props: { label: string; description: string; checked: boolean; onChange(on: boolean): void }) {
  return (
    <div class="flex items-center justify-between gap-4">
      <div>
        <div class="text-sm font-medium">{props.label}</div>
        <div class="text-xs text-muted">{props.description}</div>
      </div>
      <Toggle label={props.label} checked={props.checked} onChange={props.onChange} />
    </div>
  );
}

/**
 * A show's or movie's name, and finding it on TMDB; without a TMDB key, a show
 * on TVmaze (which needs none).
 */
function TitleField(props: {
  kind: MediaKind;
  label: string;
  placeholder: string;
  value: string;
  onInput(value: string): void;
  chosen: FoundTitle | null;
  onChoose(title: FoundTitle | null): void;
  /** Before where the chosen title's from: "Titles from" (TMDB). */
  chosenText: string;
  /** Said when there's no TMDB key. */
  noKeyText: string;
}) {
  const [status] = createResource(() => api.tmdbStatus().catch(() => null));
  const [results, setResults] = createSignal<FoundTitle[] | null>(null);
  const [searching, setSearching] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const id = `names-${props.kind}`;
  /** Where to look: TMDB with a key; TVmaze for a show without one; nowhere for a movie without one. */
  const source = () => (status()?.hasKey ? 'TMDB' : props.kind === 'show' && !status.loading ? 'TVmaze' : null);
  const sourceName = (s: FoundTitle) => (s.source === 'tvmaze' ? 'TVmaze' : 'TMDB');

  const search = async () => {
    const q = props.value.trim();
    if (!q || searching()) return;
    setSearching(true);
    setError(null);
    try {
      const found = source() === 'TMDB' ? await api.tmdbSearch(q, props.kind) : await api.tvmazeSearch(q);
      setResults(found);
      if (!found.length) setError(`Nothing on ${source()} called “${q}”.`);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSearching(false);
    }
  };

  return (
    <div>
      <label class={sectionLabel} for={id}>
        {props.label}
      </label>
      <div class="flex items-center gap-1.5">
        <input
          id={id}
          class={field}
          spellcheck={false}
          autocomplete="off"
          placeholder={props.placeholder}
          value={props.value}
          onInput={(e) => props.onInput(e.currentTarget.value)}
          on:keydown={(e) => {
            // Enter looks it up, rather than submitting the dialog.
            if (e.key === 'Enter' && !e.metaKey && !e.ctrlKey) {
              e.preventDefault();
              if (source()) void search();
            }
          }}
        />
        <Show when={source()}>
          <button
            type="button"
            class={`${smallButton} h-8`}
            disabled={searching() || !props.value.trim()}
            onClick={() => void search()}
          >
            <Icon of={Search} size={14} />
            {searching() ? 'Finding…' : `Find on ${source()}`}
          </button>
        </Show>
      </div>

      <Show when={props.chosen}>
        {(s) => (
          <div class="mt-1.5 flex items-center gap-1.5 text-xs text-muted">
            <span class="truncate">
              {props.chosenText} {sourceName(s())}: <span class="text-fg">{titleLabel(s())}</span>
            </span>
            <button
              type="button"
              class="grid size-5 flex-none place-items-center rounded-button hover:bg-hover hover:text-fg"
              aria-label="Not this one"
              title="Not this one"
              onClick={() => props.onChoose(null)}
            >
              <Icon of={X} size={12} />
            </button>
          </div>
        )}
      </Show>
      <Show when={!props.chosen && status() && !status()!.hasKey}>
        <p class="mt-1.5 text-xs text-muted">{props.noKeyText}</p>
      </Show>
      <Show when={error()}>
        <p class="mt-1.5 text-xs text-danger" role="alert">
          {error()}
        </p>
      </Show>

      <Show when={results()?.length}>
        <ul class="mt-2 max-h-48 divide-y divide-line overflow-y-auto rounded-card border border-line" role="listbox">
          <For each={results()}>
            {(s) => (
              <li>
                <button
                  type="button"
                  class="block w-full px-3 py-2 text-left hover:bg-hover"
                  onClick={() => {
                    props.onChoose(s);
                    setResults(null);
                  }}
                >
                  <div class="truncate text-sm font-medium">{titleLabel(s)}</div>
                  <Show when={s.overview}>
                    <div class="line-clamp-2 text-xs text-muted">{s.overview}</div>
                  </Show>
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}

/** The names before and after: what changes, then (folded) what doesn't and why. */
function Preview(props: { names: Names }) {
  const n = () => props.names;
  const [showKept, setShowKept] = createSignal(false);
  const changes = () => {
    const p = n().plan();
    if (!p) return [];
    const files = p.files.filter((f) => f.to).map((f) => ({ from: lastPart(f.path), to: f.to!, label: '' }));
    const folders = p.folders.filter((f) => f.to).map((f) => ({ from: lastPart(f.path), to: f.to!, label: 'Folder' }));
    const top = p.top ? [{ from: p.name, to: p.top, label: p.singleFile ? 'File' : 'Folder' }] : [];
    const moved = p.ownFolder ? [{ from: 'its download folder', to: p.ownFolder, label: 'Into' }] : [];
    return [...top, ...moved, ...folders, ...files];
  };
  const kept = () => {
    const p = n().plan();
    // A torrent that's one file is renamed as the torrent (the top line), not as a file.
    return p ? p.files.filter((f) => !f.to && !(p.singleFile && p.top)) : [];
  };

  return (
    <div>
      <div class={sectionLabel}>Preview</div>
      <Show when={n().problem()}>
        <p class="text-xs text-danger" role="alert">
          {n().problem()}
        </p>
      </Show>
      <Show when={n().plan()?.problem}>
        <p class="mb-1.5 text-xs text-warning">{n().plan()!.problem}</p>
      </Show>
      <Show when={n().plan()}>
        <Show when={changes().length} fallback={<p class="text-xs text-faint">No names change.</p>}>
          <ul class="max-h-56 divide-y divide-line overflow-y-auto rounded-card border border-line text-xs">
            <For each={changes()}>
              {(c) => (
                <li class="px-3 py-1.5">
                  <div class="truncate font-medium" title={c.to}>
                    <Show when={c.label}>
                      <span class="mr-1 text-muted">{c.label}</span>
                    </Show>
                    {c.to}
                  </div>
                  <div class="flex min-w-0 items-center gap-1 text-faint" title={c.from}>
                    <Icon of={ArrowRight} size={12} class="flex-none rotate-180" />
                    <span class="truncate">{c.from}</span>
                  </div>
                </li>
              )}
            </For>
          </ul>
        </Show>
        <Show when={kept().length}>
          <button
            type="button"
            class="mt-1.5 text-xs text-muted hover:text-fg"
            onClick={() => setShowKept(!showKept())}
          >
            {kept().length === 1 ? '1 file keeps its name' : `${kept().length} files keep their names`}
            {showKept() ? ' ▾' : ' ▸'}
          </button>
          <Show when={showKept()}>
            <ul class="mt-1 space-y-0.5 text-xs text-muted">
              <For each={kept()}>
                {(f) => (
                  <li class="flex min-w-0 gap-2">
                    <span class="truncate">{lastPart(f.path)}</span>
                    <Show when={f.note}>
                      <span class="flex-none text-faint">{f.note}</span>
                    </Show>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        </Show>
      </Show>
      <Show when={!n().plan() && !n().problem()}>
        <p class="text-xs text-faint">Working it out…</p>
      </Show>
    </div>
  );
}
