import { createSignal, onCleanup, onMount, Show } from 'solid-js';
import Download from 'lucide-solid/icons/download';

import { isTorrentFile, openAdd } from '../adder';
import { onDragDrop } from '../api';
import { isConnected } from '../connection';
import { toast } from '../toast';
import { Icon } from './Icon';

/** Covers the window while files are dragged over it; dropped .torrent files go to the add dialog. */
export function DropOverlay() {
  const [count, setCount] = createSignal<number | null>(null);

  onMount(() => {
    const unlisten = onDragDrop((e) => {
      if (e.type === 'enter') setCount(e.paths.filter(isTorrentFile).length);
      else if (e.type === 'leave') setCount(null);
      else if (e.type === 'drop') {
        setCount(null);
        if (!isConnected()) return;
        const torrents = e.paths.filter(isTorrentFile);
        if (torrents.length) openAdd({ files: torrents });
        else toast('Only .torrent files can be added.', { error: true });
      }
    });
    onCleanup(() => void unlisten.then((off) => off()));
  });

  return (
    <Show when={count() !== null && isConnected()}>
      <div class="pointer-events-none fixed inset-0 z-40 grid place-items-center bg-bg/70 p-8 backdrop-blur-sm">
        <div
          class="flex w-[min(420px,100%)] flex-col items-center gap-2 rounded-dialog border-2 border-dashed px-8 py-10 text-center shadow-lift"
          classList={{ 'border-accent bg-popover': !!count(), 'border-line bg-element': !count() }}
        >
          <Icon of={Download} size={40} class={`mb-1 ${count() ? 'text-accent' : 'text-muted'}`} />
          <Show when={count()} fallback={<p class="text-lg font-semibold">Only .torrent files can be added</p>}>
            {(n) => (
              <>
                <p class="text-lg font-semibold">Drop to add {n() === 1 ? 'a torrent' : `${n()} torrents`}</p>
                <p class="text-sm text-muted">You’ll choose where they go before they’re added.</p>
              </>
            )}
          </Show>
        </div>
      </div>
    </Show>
  );
}
