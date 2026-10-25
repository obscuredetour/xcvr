import ArrowDown from 'lucide-solid/icons/arrow-down';
import ArrowUp from 'lucide-solid/icons/arrow-up';

import { Icon } from './Icon';

/** The bar along the bottom: connection state, torrent count, global speeds. Live from phase 3. */
export function StatusBar() {
  return (
    <footer class="fixed inset-x-0 bottom-0 z-10 flex h-(--statusbar-h) items-center gap-3 border-t border-line bg-bg pr-3 pl-4 text-xs text-muted">
      <span class="flex items-center gap-1.5 whitespace-nowrap">
        <span class="size-2 rounded-full bg-faint" aria-hidden="true" />
        Not connected
      </span>
      <span class="text-[12.5px] whitespace-nowrap tabular-nums">0 torrents</span>
      <div class="ml-auto flex items-center gap-3 tabular-nums">
        <span class="flex items-center gap-1" title="Download speed">
          <Icon of={ArrowDown} size={12} />0 kB/s
        </span>
        <span class="flex items-center gap-1" title="Upload speed">
          <Icon of={ArrowUp} size={12} />0 kB/s
        </span>
      </div>
    </footer>
  );
}
