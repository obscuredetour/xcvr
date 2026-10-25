import { createSignal, onMount, Show } from 'solid-js';

import { relocating, renaming, setRelocating, setRenaming } from '../actions';
import { api, errorMessage } from '../api';
import { primaryButton, secondaryButton } from '../buttons';
import { rememberFolder } from '../folders';
import { toast } from '../toast';
import { torrent } from '../torrents';
import { FolderMenu } from './AddDialog';

const field =
  'h-8 w-full rounded-control border border-line bg-field px-2.5 text-sm outline-none select-text focus:border-focus focus:ring-3 focus:ring-focus/20';

/** The rename and set-location dialogs. Mounted once in App. */
export function TorrentDialogsHost() {
  return (
    <>
      <Show when={renaming()} keyed>
        {(r) => <RenameDialog hash={r.hash} path={r.path} current={r.current} onClose={() => setRenaming(null)} />}
      </Show>
      <Show when={relocating()} keyed>
        {(hashes) => <LocationDialog hashes={hashes} onClose={() => setRelocating(null)} />}
      </Show>
    </>
  );
}

/** A small modal with a title, its fields, and Cancel / the action. */
function Modal(props: {
  title: string;
  action: string;
  busy: boolean;
  disabled?: boolean;
  error: string | null;
  onSubmit(): void;
  onClose(): void;
  children: import('solid-js').JSX.Element;
}) {
  let dialog!: HTMLDialogElement;
  onMount(() => dialog.showModal());
  return (
    <dialog
      ref={dialog}
      aria-label={props.title}
      class="m-auto w-[min(480px,calc(100vw-48px))] animate-menu-in rounded-dialog border border-line bg-popover p-0 text-fg shadow-lift"
      onCancel={(e) => {
        e.preventDefault();
        if (!props.busy) props.onClose();
      }}
    >
      <form
        class="px-6 pt-5 pb-4"
        onSubmit={(e) => {
          e.preventDefault();
          if (!props.busy && !props.disabled) props.onSubmit();
        }}
      >
        <h2 class="mb-3 text-base font-semibold">{props.title}</h2>
        {props.children}
        <Show when={props.error}>
          {(msg) => (
            <p class="mt-3 text-xs text-danger" role="alert">
              {msg()}
            </p>
          )}
        </Show>
        <div class="mt-5 flex justify-end gap-2">
          <button type="button" class={secondaryButton} disabled={props.busy} onClick={() => props.onClose()}>
            Cancel
          </button>
          <button type="submit" class={primaryButton} disabled={props.busy || props.disabled}>
            {props.action}
          </button>
        </div>
      </form>
    </dialog>
  );
}

/** Rename a torrent, or a file or folder in it. Transmission renames it on the host. */
function RenameDialog(props: { hash: string; path: string; current: string; onClose(): void }) {
  const [name, setName] = createSignal(props.current);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const isTorrent = () => !props.path.includes('/');

  const submit = async () => {
    setBusy(true);
    setError(null);
    try {
      await api.renameTorrent(props.hash, props.path, name());
      props.onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title={isTorrent() ? 'Rename Torrent' : 'Rename'}
      action="Rename"
      busy={busy()}
      disabled={!name().trim() || name().trim() === props.current}
      error={error()}
      onSubmit={() => void submit()}
      onClose={props.onClose}
    >
      <input
        ref={(el) =>
          queueMicrotask(() => {
            el.focus();
            // The name without its extension selected, as in Finder.
            const dot = el.value.lastIndexOf('.');
            el.setSelectionRange(0, dot > 0 && !isTorrent() ? dot : el.value.length);
          })
        }
        class={field}
        aria-label="New name"
        spellcheck={false}
        value={name()}
        onInput={(e) => setName(e.currentTarget.value)}
      />
      <p class="mt-2 text-xs text-muted">
        {isTorrent()
          ? 'Renames its folder (or file) on the host too. Its files inside keep their names.'
          : 'Renames it on the host too.'}
      </p>
    </Modal>
  );
}

/** Point torrents at another folder on the host: moving their data there, or finding it there. */
function LocationDialog(props: { hashes: string[]; onClose(): void }) {
  const first = () => torrent(props.hashes[0]!);
  const [folder, setFolder] = createSignal(first()?.download_dir ?? '');
  const [moveData, setMoveData] = createSignal(true);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const many = () => props.hashes.length > 1;

  const submit = async () => {
    setBusy(true);
    setError(null);
    try {
      await api.setLocation(props.hashes, folder(), moveData());
      rememberFolder(folder());
      toast(moveData() ? 'Moving… Transmission moves the files in the background.' : 'Location set.');
      props.onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title={many() ? `Set Location of ${props.hashes.length} Torrents` : 'Set Location'}
      action={moveData() ? 'Move' : 'Set Location'}
      busy={busy()}
      disabled={!folder().trim()}
      error={error()}
      onSubmit={() => void submit()}
      onClose={props.onClose}
    >
      <Show when={!many()}>
        <p class="mb-3 truncate text-sm text-muted" title={first()?.name}>
          {first()?.name}
        </p>
      </Show>
      <label class="mb-1.5 block text-xs font-semibold text-muted" for="location-folder">
        Folder on the host
      </label>
      <div class="flex items-center gap-1.5">
        <input
          id="location-folder"
          class={field}
          spellcheck={false}
          autocomplete="off"
          value={folder()}
          onInput={(e) => setFolder(e.currentTarget.value)}
        />
        <FolderMenu defaultDir="" onChoose={setFolder} />
      </div>
      <fieldset class="mt-4 space-y-2 text-sm">
        <label class="flex cursor-pointer items-start gap-2">
          <input type="radio" name="move" class="mt-1" checked={moveData()} onChange={() => setMoveData(true)} />
          <span>
            Move the data there
            <span class="block text-xs text-muted">Transmission moves the files on the host.</span>
          </span>
        </label>
        <label class="flex cursor-pointer items-start gap-2">
          <input type="radio" name="move" class="mt-1" checked={!moveData()} onChange={() => setMoveData(false)} />
          <span>
            The data is already there
            <span class="block text-xs text-muted">Nothing is moved; Transmission looks for the files there.</span>
          </span>
        </label>
      </fieldset>
    </Modal>
  );
}
