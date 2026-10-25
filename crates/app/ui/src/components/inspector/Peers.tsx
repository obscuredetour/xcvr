import { For, Show } from 'solid-js';
import ArrowDownLeft from 'lucide-solid/icons/arrow-down-left';
import Lock from 'lucide-solid/icons/lock';

import type { Torrent } from '../../api';
import type { Peer } from '../../bindings/Peer';
import { formatPercent, formatSpeed } from '../../format';
import { Icon } from '../Icon';
import { Section } from './parts';

const SOURCES: [keyof NonNullable<Torrent['peers_from']>, string][] = [
  ['from_tracker', 'Tracker'],
  ['from_dht', 'DHT'],
  ['from_pex', 'PEX'],
  ['from_lpd', 'Local'],
  ['from_incoming', 'Incoming'],
  ['from_ltep', 'LTEP'],
  ['from_cache', 'Cache'],
];

/** Who it's connected to, busiest first. */
export function PeersTab(props: { t: Torrent }) {
  const peers = () =>
    [...(props.t.peers ?? [])].sort(
      (a, b) =>
        b.rate_to_client + b.rate_to_peer - (a.rate_to_client + a.rate_to_peer) || a.address.localeCompare(b.address),
    );
  const from = () => SOURCES.filter(([k]) => (props.t.peers_from?.[k] ?? 0) > 0);

  return (
    <>
      <Section>
        <p class="text-xs text-muted">
          Connected to {props.t.peers_connected ?? 0} · downloading from {props.t.peers_sending_to_us ?? 0} · uploading
          to {props.t.peers_getting_from_us ?? 0}
        </p>
        <Show when={from().length}>
          <div class="mt-2 flex flex-wrap gap-1">
            <For each={from()}>
              {([key, label]) => (
                <span class="rounded-chip bg-element px-2 py-0.5 text-[11px] text-muted tabular-nums">
                  {label} {props.t.peers_from?.[key]}
                </span>
              )}
            </For>
          </div>
        </Show>
      </Section>
      <Show when={peers().length} fallback={<p class="px-5 text-xs text-muted">No peers right now.</p>}>
        <ul class="divide-y divide-line border-t border-line">
          <For each={peers()}>{(p) => <PeerRow p={p} />}</For>
        </ul>
      </Show>
    </>
  );
}

function PeerRow(props: { p: Peer }) {
  const p = () => props.p;
  return (
    <li class="flex items-center gap-3 px-5 py-2">
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-1 text-xs">
          <span class="truncate font-mono text-[11px]">{p().address}</span>
          <Show when={p().is_encrypted}>
            <span title="Encrypted">
              <Icon of={Lock} size={12} class="flex-none text-muted" />
            </span>
          </Show>
          <Show when={p().is_incoming}>
            <span title="They connected to us">
              <Icon of={ArrowDownLeft} size={12} class="flex-none text-muted" />
            </span>
          </Show>
          <Show when={p().is_utp}>
            <span class="text-[10px] text-muted" title="Over µTP">
              µTP
            </span>
          </Show>
        </div>
        <div class="truncate text-[11px] text-muted" title={p().flag_str}>
          {p().client_name || 'Unknown client'} · has {formatPercent(p().progress)}
        </div>
      </div>
      <div class="flex-none text-right text-[11px] tabular-nums">
        <div classList={{ 'text-down': p().rate_to_client > 0, 'text-faint': !p().rate_to_client }}>
          ↓ {p().rate_to_client ? formatSpeed(p().rate_to_client) : '–'}
        </div>
        <div classList={{ 'text-up': p().rate_to_peer > 0, 'text-faint': !p().rate_to_peer }}>
          ↑ {p().rate_to_peer ? formatSpeed(p().rate_to_peer) : '–'}
        </div>
      </div>
    </li>
  );
}
