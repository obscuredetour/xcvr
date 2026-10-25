import { Show } from 'solid-js';
import ArrowDown from 'lucide-solid/icons/arrow-down';
import ArrowUp from 'lucide-solid/icons/arrow-up';

import { described, isConnected, stats } from '../connection';
import { formatSpeed } from '../format';
import { torrentCount } from '../torrents';
import { Icon } from './Icon';
import { StatusDot } from './StatusDot';

/** The bar along the bottom: the connection, how many torrents, global speeds. */
export function StatusBar() {
  const d = described;
  const count = () => `${torrentCount()} ${torrentCount() === 1 ? 'torrent' : 'torrents'}`;
  return (
    <footer class="fixed inset-x-0 bottom-0 z-10 flex h-(--statusbar-h) items-center gap-3 border-t border-line bg-bg pr-3 pl-4 text-xs text-muted">
      <span class="flex min-w-0 items-center gap-1.5 whitespace-nowrap" title={d().detail ?? undefined} role="status">
        <StatusDot tone={d().tone} />
        <span class="truncate">{d().host ? `${d().host} · ${d().short}` : d().short}</span>
      </span>
      <Show when={isConnected()}>
        <span class="text-[12.5px] whitespace-nowrap tabular-nums">{count()}</span>
      </Show>
      <Show when={isConnected() && stats()}>
        {(s) => (
          <div class="ml-auto flex items-center gap-3 tabular-nums">
            <span class="flex items-center gap-1" title="Download speed">
              <Icon of={ArrowDown} size={12} />
              {formatSpeed(s().download_speed)}
            </span>
            <span class="flex items-center gap-1" title="Upload speed">
              <Icon of={ArrowUp} size={12} />
              {formatSpeed(s().upload_speed)}
            </span>
          </div>
        )}
      </Show>
    </footer>
  );
}
