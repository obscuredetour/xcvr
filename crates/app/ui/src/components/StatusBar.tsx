import { Show } from 'solid-js';
import ArrowDown from 'lucide-solid/icons/arrow-down';
import ArrowUp from 'lucide-solid/icons/arrow-up';
import Turtle from 'lucide-solid/icons/turtle';

import { described, isConnected, session, stats } from '../connection';
import { formatSpeed } from '../format';
import { torrentCount } from '../torrents';
import { Icon } from './Icon';
import { StatusDot } from './StatusDot';

/** The limit in force, in kB/s: the turtle's when it's on, else the global one if it's set. */
function limits(): { down: number | null; up: number | null; turtle: boolean } {
  const s = session();
  if (!s) return { down: null, up: null, turtle: false };
  if (s.alt_speed_enabled) return { down: s.alt_speed_down ?? null, up: s.alt_speed_up ?? null, turtle: true };
  return {
    down: s.speed_limit_down_enabled ? (s.speed_limit_down ?? null) : null,
    up: s.speed_limit_up_enabled ? (s.speed_limit_up ?? null) : null,
    turtle: false,
  };
}

/** "/ 100 kB/s" after a speed, when a limit holds it. */
const cap = (kbps: number | null) => (kbps === null ? '' : ` / ${formatSpeed(kbps * 1000)}`);

/** The bar along the bottom: the connection, how many torrents, global speeds and the limits on them. */
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
            <Show when={limits().turtle}>
              <span class="flex items-center gap-1 text-accent" title="Slower speeds are on">
                <Icon of={Turtle} size={12} />
              </span>
            </Show>
            <span class="flex items-center gap-1" title="Download speed, and the limit on it">
              <Icon of={ArrowDown} size={12} class="text-down" />
              {formatSpeed(s().download_speed)}
              <span class="text-faint">{cap(limits().down)}</span>
            </span>
            <span class="flex items-center gap-1" title="Upload speed, and the limit on it">
              <Icon of={ArrowUp} size={12} class="text-up" />
              {formatSpeed(s().upload_speed)}
              <span class="text-faint">{cap(limits().up)}</span>
            </span>
          </div>
        )}
      </Show>
    </footer>
  );
}
