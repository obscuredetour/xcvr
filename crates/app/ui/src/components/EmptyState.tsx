import Server from 'lucide-solid/icons/server';

import { primaryButton } from '../buttons';
import { Icon } from './Icon';

/** Where the torrent table goes, before there's a host to show. */
export function EmptyState(props: { onOpenSettings(): void }) {
  return (
    <div class="grid flex-1 place-items-center p-8">
      <div class="flex max-w-sm flex-col items-center text-center">
        <span class="grid size-14 place-items-center rounded-card bg-highlight text-accent">
          <Icon of={Server} size={20} />
        </span>
        <h2 class="mt-4 text-lg font-semibold">Not connected</h2>
        <p class="mt-1.5 text-muted">Connect to the Transmission host to see its torrents here.</p>
        <button class={`mt-5 ${primaryButton}`} onClick={() => props.onOpenSettings()}>
          Connection Settings…
        </button>
      </div>
    </div>
  );
}
