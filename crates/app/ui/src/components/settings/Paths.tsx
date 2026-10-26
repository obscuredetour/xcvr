import { open as openFolderPicker } from '@tauri-apps/plugin-dialog';
import { createResource, createSignal, Index, Show } from 'solid-js';
import ArrowRight from 'lucide-solid/icons/arrow-right';
import CircleAlert from 'lucide-solid/icons/circle-alert';
import CircleCheck from 'lucide-solid/icons/circle-check';
import Plus from 'lucide-solid/icons/plus';
import X from 'lucide-solid/icons/x';

import { api, errorMessage, type MappingView, type PathMapping } from '../../api';
import { primaryButton } from '../../buttons';
import { isMac, keys } from '../../keys';
import { toast } from '../../toast';
import { Icon } from '../Icon';
import { button, Group, Heading } from './parts';

const field =
  'h-7 min-w-0 flex-1 rounded-control border border-line bg-field px-2 text-xs outline-none select-text focus:border-focus';

/** Where the host's folders are on this computer, for Open and Reveal. */
export function PathSettings() {
  const [saved, { mutate }] = createResource(() => api.pathMappings().catch(() => [] as MappingView[]));
  const [rows, setRows] = createSignal<PathMapping[] | null>(null);
  const list = (): PathMapping[] => rows() ?? saved()?.map(({ host, local }) => ({ host, local })) ?? [];
  const status = (m: PathMapping) => saved()?.find((s) => s.host === m.host && s.local === m.local);
  const dirty = () => rows() !== null;

  const edit = (i: number, key: keyof PathMapping, value: string) =>
    setRows(list().map((m, j) => (j === i ? { ...m, [key]: value } : m)));

  // The host's .torrent copies, checked again whenever the mappings are saved.
  const [folder] = createResource(saved, () => api.torrentsFolder().catch(() => null));

  const save = async (next: PathMapping[] = list()) => {
    try {
      mutate(await api.setPathMappings(next));
      setRows(null);
      toast('Saved.');
    } catch (err) {
      toast(errorMessage(err), { error: true });
    }
  };

  /** Pick where the host's torrents folder is mounted here, and map it. */
  const mapTorrentsFolder = async (host: string) => {
    const picked = await openFolderPicker({ directory: true, title: 'Where is the host’s torrents folder mounted?' });
    if (typeof picked === 'string') await save([...list(), { host, local: picked }]);
  };

  const example = isMac ? '/Volumes/Downloads' : '/mnt/mini/Downloads';

  return (
    <>
      <Heading title="Paths">
        <p>
          Open and Reveal find a torrent’s files on this computer, through the share you’ve mounted from the host. Say
          where each of the host’s download folders is here.
        </p>
        <p>For this computer only: the other one mounts the share somewhere else, and has its own list.</p>
      </Heading>

      <div class="mb-1.5 flex gap-2 px-1 text-[11px] font-semibold tracking-[.06em] text-muted uppercase">
        <span class="flex-1">On the host</span>
        <span class="w-4" />
        <span class="flex-1">On this computer</span>
        <span class="w-14" />
      </div>
      <div class="space-y-2">
        {/* By position, not by row: each edit replaces its row, and <For> would rebuild it (losing focus). */}
        <Index each={list()}>
          {(m, i) => (
            <div class="flex items-center gap-2">
              <input
                class={field}
                aria-label="Folder on the host"
                placeholder="/Users/alice/Downloads"
                spellcheck={false}
                value={m().host}
                onInput={(e) => edit(i, 'host', e.currentTarget.value)}
              />
              <Icon of={ArrowRight} size={14} class="flex-none text-muted" />
              <input
                class={field}
                aria-label="Folder on this computer"
                placeholder={example}
                spellcheck={false}
                value={m().local}
                onInput={(e) => edit(i, 'local', e.currentTarget.value)}
              />
              <span class="grid w-6 flex-none place-items-center">
                <Show when={!dirty() && status(m())}>
                  {(s) =>
                    s().localExists ? (
                      <span title="Found here">
                        <Icon of={CircleCheck} size={14} class="text-success" />
                      </span>
                    ) : (
                      <span title="Not found here: is the share mounted?">
                        <Icon of={CircleAlert} size={14} class="text-warning" />
                      </span>
                    )
                  }
                </Show>
              </span>
              <button
                class="grid size-7 flex-none place-items-center rounded-button text-muted hover:bg-hover hover:text-fg"
                aria-label="Remove this mapping"
                onClick={() => setRows(list().filter((_, j) => j !== i))}
              >
                <Icon of={X} size={12} />
              </button>
            </div>
          )}
        </Index>
      </div>
      <Show when={!list().length}>
        <p class="px-1 text-xs text-faint">None yet.</p>
      </Show>

      <div class="mt-4 flex items-center gap-2">
        <button class={button} onClick={() => setRows([...list(), { host: '', local: '' }])}>
          <Icon of={Plus} size={12} />
          Add a Folder
        </button>
        <Show when={dirty()}>
          <button class={`ml-auto ${primaryButton}`} onClick={() => void save()}>
            Save
          </button>
        </Show>
      </div>

      <Show when={folder()}>
        {(f) => (
          <Group
            title="Original names"
            class="mt-8"
            description="A renamed torrent shows its original name, read from the host’s copy of its .torrent file. Transmission keeps those copies in this folder, so map it to show names from before renames made anywhere."
          >
            <div class="space-y-2.5 py-3">
              <code class="block font-mono text-xs break-all text-fg select-text">{f().host}</code>
              <Show
                when={f().local}
                fallback={
                  <>
                    <p class="text-xs text-muted">
                      Share it from the host and mount it here, then choose where it is.
                      <Show when={f().host.includes('/Library/')}>
                        {' '}
                        Finder hides Library on the host: use Go › Go to Folder ({keys('shift+mod+G')}) to reach it.
                      </Show>
                    </p>
                    <button class={button} onClick={() => void mapTorrentsFolder(f().host)}>
                      Choose Where It’s Mounted…
                    </button>
                  </>
                }
              >
                {(local) => (
                  <p class="flex items-start gap-1.5 text-xs text-muted">
                    <Show
                      when={f().readable}
                      fallback={
                        <>
                          <Icon of={CircleAlert} size={14} class="mt-px flex-none text-warning" />
                          <span>
                            Mapped to <span class="break-all text-fg">{local()}</span>, but its files aren’t there. Is
                            the share mounted?
                          </span>
                        </>
                      }
                    >
                      <Icon of={CircleCheck} size={14} class="mt-px flex-none text-success" />
                      <span>
                        Found at <span class="break-all text-fg">{local()}</span>. Renamed torrents show their original
                        name.
                      </span>
                    </Show>
                  </p>
                )}
              </Show>
            </div>
          </Group>
        )}
      </Show>
    </>
  );
}
