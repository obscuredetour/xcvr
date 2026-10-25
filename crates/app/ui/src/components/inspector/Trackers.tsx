import { For, Show } from 'solid-js';

import type { Torrent } from '../../api';
import type { TrackerStat } from '../../bindings/TrackerStat';
import { relativeTime } from './parts';

/** A count from a tracker: -1 means it didn't say. */
const count = (n: number) => (n >= 0 ? String(n) : '–');

/** Each tracker, by tier: how its last announce went, and what it reports. */
export function TrackersTab(props: { t: Torrent }) {
  const stats = () => [...(props.t.tracker_stats ?? [])].sort((a, b) => a.tier - b.tier || a.id - b.id);
  return (
    <Show
      when={stats().length}
      fallback={<p class="p-5 text-xs text-muted">No trackers: it finds peers through DHT and PEX.</p>}
    >
      <ul class="divide-y divide-line">
        <For each={stats()}>{(s) => <TrackerRow s={s} />}</For>
      </ul>
    </Show>
  );
}

function TrackerRow(props: { s: TrackerStat }) {
  const s = () => props.s;
  const failing = () => s().has_announced && !s().last_announce_succeeded;
  const announce = () => {
    if (!s().has_announced) return 'Not announced yet';
    const when = relativeTime(s().last_announce_time);
    return s().last_announce_succeeded
      ? `Announced ${when}: ${s().last_announce_peer_count} peers`
      : `Failed ${when}: ${s().last_announce_result || 'no answer'}`;
  };
  return (
    <li class="px-5 py-3 text-xs">
      <div class="flex items-baseline justify-between gap-2">
        <span class="truncate font-medium">{s().sitename || s().host}</span>
        <span class="flex-none text-[11px] text-muted">
          Tier {s().tier + 1}
          {s().is_backup ? ' · backup' : ''}
        </span>
      </div>
      <div class="truncate font-mono text-[10px] text-faint" title={s().announce}>
        {s().announce}
      </div>
      <div class="mt-1.5" classList={{ 'text-warning': failing(), 'text-muted': !failing() }}>
        {announce()}
      </div>
      <Show when={s().next_announce_time > 0 && !s().is_backup}>
        <div class="text-muted">Next {relativeTime(s().next_announce_time)}</div>
      </Show>
      <div class="mt-1 flex gap-3 text-muted tabular-nums">
        <span>Seeders {count(s().seeder_count)}</span>
        <span>Leechers {count(s().leecher_count)}</span>
        <span>Downloaded {count(s().download_count)}</span>
      </div>
    </li>
  );
}
