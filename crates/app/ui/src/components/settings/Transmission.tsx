import { createResource, createSignal, For, type JSX, Show } from 'solid-js';
import CircleAlert from 'lucide-solid/icons/circle-alert';

import { api, asProblem, errorMessage, type Session, type SessionSet } from '../../api';
import { connection } from '../../connection';
import { toast } from '../../toast';
import { NumberField, TextField } from '../Fields';
import { button, Callout, Group, Heading, Row, Segmented, Toggle } from './parts';

/** Days of the week as Transmission's schedule counts them (tr_sched_day). */
const DAYS: [number, string][] = [
  [2, 'Mon'],
  [4, 'Tue'],
  [8, 'Wed'],
  [16, 'Thu'],
  [32, 'Fri'],
  [64, 'Sat'],
  [1, 'Sun'],
];

const time = (minutes: number) =>
  `${String(Math.floor(minutes / 60) % 24).padStart(2, '0')}:${String(minutes % 60).padStart(2, '0')}`;
const minutes = (hhmm: string) => {
  const [h, m] = hhmm.split(':').map(Number);
  return (h ?? 0) * 60 + (m ?? 0);
};

/**
 * Transmission's own settings, on the host: they apply to it whichever computer
 * changes them. Each change is sent as it's made.
 */
export function TransmissionSettings() {
  const [settings, { mutate }] = createResource(
    () => connection().state === 'connected',
    (connected) => (connected ? api.sessionSettings() : Promise.resolve(null)),
  );
  const s = () => settings() ?? null;

  const change = async (set: SessionSet) => {
    try {
      mutate(await api.setSession(set));
    } catch (err) {
      toast(errorMessage(err), { error: true });
    }
  };

  return (
    <>
      <Heading title="Transmission">
        Transmission’s own settings, kept on the host: changing them here changes them for every computer that uses it.
      </Heading>
      <Show
        when={s()}
        fallback={
          <Callout icon={CircleAlert} title={settings.loading ? 'Loading…' : 'Not connected'}>
            <Show when={!settings.loading}>
              Connect to Transmission (Settings › Connection) to change its settings.
            </Show>
          </Callout>
        }
      >
        {(v) => <Page s={v()} change={change} />}
      </Show>
    </>
  );
}

function Page(props: { s: Session; change(set: SessionSet): Promise<void> }) {
  const s = () => props.s;
  const set = (change: SessionSet) => void props.change(change);
  const kb = 'kB/s';

  return (
    <>
      <Group title="Downloading">
        <Row label="Download to" description="Where new torrents go unless you choose a folder when adding.">
          <TextField label="Download to" value={s().download_dir ?? ''} onCommit={(v) => set({ download_dir: v })} />
        </Row>
        <Row label="Keep unfinished downloads apart" description="Downloads stay here until they finish, then move.">
          <TextField
            label="Folder for unfinished downloads"
            value={s().incomplete_dir ?? ''}
            disabled={!s().incomplete_dir_enabled}
            class="w-52"
            onCommit={(v) => set({ incomplete_dir: v })}
          />
          <Toggle
            label="Keep unfinished downloads apart"
            checked={!!s().incomplete_dir_enabled}
            onChange={(on) => set({ incomplete_dir_enabled: on })}
          />
        </Row>
        <Row label="Start added torrents" description="Otherwise they’re added paused.">
          <Toggle
            label="Start added torrents"
            checked={!!s().start_added_torrents}
            onChange={(on) => set({ start_added_torrents: on })}
          />
        </Row>
        <Row label="Mark unfinished files" description="Adds .part to files until they’re complete.">
          <Toggle
            label="Mark unfinished files"
            checked={!!s().rename_partial_files}
            onChange={(on) => set({ rename_partial_files: on })}
          />
        </Row>
      </Group>

      <Group title="Speed">
        <Limit
          label="Download limit"
          on={s().speed_limit_down_enabled}
          value={s().speed_limit_down}
          unit={kb}
          onToggle={(on) => set({ speed_limit_down_enabled: on })}
          onValue={(n) => set({ speed_limit_down: n })}
        />
        <Limit
          label="Upload limit"
          on={s().speed_limit_up_enabled}
          value={s().speed_limit_up}
          unit={kb}
          onToggle={(on) => set({ speed_limit_up_enabled: on })}
          onValue={(n) => set({ speed_limit_up: n })}
        />
      </Group>

      <Group
        title="Slower speeds (the turtle)"
        description="Limits to switch to when the network’s needed for other things: from the turtle in the toolbar, or on a schedule."
      >
        <Row label="On now">
          <Toggle
            label="Slower speeds on"
            checked={!!s().alt_speed_enabled}
            onChange={(on) => set({ alt_speed_enabled: on })}
          />
        </Row>
        <Row label="Download">
          <NumberField
            label="Slower download speed"
            value={s().alt_speed_down ?? 0}
            step={10}
            onCommit={(n) => set({ alt_speed_down: Math.round(n) })}
          />
          <Unit>{kb}</Unit>
        </Row>
        <Row label="Upload">
          <NumberField
            label="Slower upload speed"
            value={s().alt_speed_up ?? 0}
            step={10}
            onCommit={(n) => set({ alt_speed_up: Math.round(n) })}
          />
          <Unit>{kb}</Unit>
        </Row>
        <Row
          label="On a schedule"
          description="Switches the turtle by the clock. Switching it by hand holds until the schedule next turns."
        >
          <Toggle
            label="Slower speeds on a schedule"
            checked={!!s().alt_speed_time_enabled}
            onChange={(on) => set({ alt_speed_time_enabled: on })}
          />
        </Row>
        <Show when={s().alt_speed_time_enabled}>
          <Row label="From">
            <input
              type="time"
              aria-label="Slower speeds from"
              class="h-7 rounded-control border border-line bg-field px-2 text-xs tabular-nums outline-none focus:border-focus"
              value={time(s().alt_speed_time_begin ?? 0)}
              onChange={(e) => set({ alt_speed_time_begin: minutes(e.currentTarget.value) })}
            />
            <span class="text-xs text-muted">to</span>
            <input
              type="time"
              aria-label="Slower speeds until"
              class="h-7 rounded-control border border-line bg-field px-2 text-xs tabular-nums outline-none focus:border-focus"
              value={time(s().alt_speed_time_end ?? 0)}
              onChange={(e) => set({ alt_speed_time_end: minutes(e.currentTarget.value) })}
            />
          </Row>
          <Row label="On">
            <div class="flex gap-1" role="group" aria-label="Days">
              <For each={DAYS}>
                {([bit, day]) => {
                  const on = () => ((s().alt_speed_time_day ?? 0) & bit) !== 0;
                  return (
                    <button
                      class="h-7 w-10 rounded-button text-xs font-medium"
                      classList={{ 'bg-accent text-on-accent': on(), 'bg-element text-muted hover:bg-hover': !on() }}
                      aria-pressed={on()}
                      onClick={() => set({ alt_speed_time_day: (s().alt_speed_time_day ?? 0) ^ bit })}
                    >
                      {day}
                    </button>
                  );
                }}
              </For>
            </div>
          </Row>
        </Show>
      </Group>

      <Group title="Queue" description="How many torrents run at once; the rest wait their turn.">
        <Limit
          label="Downloading at once"
          on={s().download_queue_enabled}
          value={s().download_queue_size}
          unit=""
          step={1}
          onToggle={(on) => set({ download_queue_enabled: on })}
          onValue={(n) => set({ download_queue_size: Math.max(1, Math.round(n)) })}
        />
        <Limit
          label="Seeding at once"
          on={s().seed_queue_enabled}
          value={s().seed_queue_size}
          unit=""
          step={1}
          onToggle={(on) => set({ seed_queue_enabled: on })}
          onValue={(n) => set({ seed_queue_size: Math.max(1, Math.round(n)) })}
        />
        <Limit
          label="Don’t count stalled torrents"
          description="Torrents idle this long don’t take a place in the queue."
          on={s().queue_stalled_enabled}
          value={s().queue_stalled_minutes}
          unit="min"
          step={1}
          onToggle={(on) => set({ queue_stalled_enabled: on })}
          onValue={(n) => set({ queue_stalled_minutes: Math.round(n) })}
        />
      </Group>

      <Group
        title="Seeding"
        description="When finished torrents stop uploading, unless a torrent has its own limits (inspector › Options)."
      >
        <Limit
          label="Stop at ratio"
          on={s().seed_ratio_limited}
          value={s().seed_ratio_limit}
          unit=""
          step={0.1}
          onToggle={(on) => set({ seed_ratio_limited: on })}
          onValue={(n) => set({ seed_ratio_limit: n })}
        />
        <Limit
          label="Stop when idle for"
          on={s().idle_seeding_limit_enabled}
          value={s().idle_seeding_limit}
          unit="min"
          step={1}
          onToggle={(on) => set({ idle_seeding_limit_enabled: on })}
          onValue={(n) => set({ idle_seeding_limit: Math.round(n) })}
        />
      </Group>

      <Network s={s()} set={set} />
    </>
  );
}

function Unit(props: { children: JSX.Element }) {
  return <span class="w-9 text-xs text-muted">{props.children}</span>;
}

/** A limit: on or off, and how much. */
function Limit(props: {
  label: string;
  description?: string;
  on: boolean | undefined;
  value: number | undefined;
  unit: string;
  step?: number;
  onToggle(on: boolean): void;
  onValue(n: number): void;
}) {
  return (
    <Row label={props.label} description={props.description}>
      <NumberField
        label={props.label}
        value={props.value ?? 0}
        step={props.step ?? 10}
        disabled={!props.on}
        onCommit={props.onValue}
      />
      <Unit>{props.unit}</Unit>
      <Toggle label={`${props.label} on`} checked={!!props.on} onChange={props.onToggle} />
    </Row>
  );
}

/** Peers, the port, encryption and the blocklist. */
function Network(props: { s: Session; set(change: SessionSet): void }) {
  const s = () => props.s;
  const [port, setPort] = createSignal<string | null>(null);
  const [blocklist, setBlocklist] = createSignal<string | null>(null);

  const testPort = async () => {
    setPort('Checking…');
    try {
      const r = await api.testPort();
      setPort(r.port_is_open ? 'Open: peers can reach it.' : 'Closed: peers can’t reach it from the internet.');
    } catch (err) {
      setPort(asProblem(err).message);
    }
  };
  const updateBlocklist = async () => {
    setBlocklist('Updating…');
    try {
      const n = await api.updateBlocklist();
      setBlocklist(`Updated: ${n.toLocaleString()} rules.`);
    } catch (err) {
      setBlocklist(asProblem(err).message);
    }
  };

  return (
    <>
      <Group title="Network">
        <Row label="Peer port" description={port() ?? 'The port other peers connect to.'}>
          <NumberField
            label="Peer port"
            value={s().peer_port ?? 51413}
            step={1}
            min={1}
            onCommit={(n) => props.set({ peer_port: Math.round(n) })}
          />
          <button class={button} onClick={() => void testPort()}>
            Test Port
          </button>
        </Row>
        <Row label="Pick a random port at start">
          <Toggle
            label="Random port at start"
            checked={!!s().peer_port_random_on_start}
            onChange={(on) => props.set({ peer_port_random_on_start: on })}
          />
        </Row>
        <Row label="Forward the port on the router" description="With UPnP or NAT-PMP, where the router allows it.">
          <Toggle
            label="Port forwarding"
            checked={!!s().port_forwarding_enabled}
            onChange={(on) => props.set({ port_forwarding_enabled: on })}
          />
        </Row>
        <Row label="Encryption">
          <Segmented
            label="Encryption"
            value={
              (s().encryption === 'unknown' ? 'preferred' : (s().encryption ?? 'preferred')) as
                'required' | 'preferred' | 'allowed'
            }
            options={[
              ['required', 'Required'],
              ['preferred', 'Preferred'],
              ['allowed', 'Allowed'],
            ]}
            onChange={(v) => props.set({ encryption: v })}
          />
        </Row>
        <Row label="Most peers, overall">
          <NumberField
            label="Most peers overall"
            value={s().peer_limit_global ?? 200}
            step={10}
            min={1}
            onCommit={(n) => props.set({ peer_limit_global: Math.round(n) })}
          />
        </Row>
        <Row label="Most peers, per torrent">
          <NumberField
            label="Most peers per torrent"
            value={s().peer_limit_per_torrent ?? 50}
            step={5}
            min={1}
            onCommit={(n) => props.set({ peer_limit_per_torrent: Math.round(n) })}
          />
        </Row>
        <Row label="Find peers through DHT" description="For public torrents.">
          <Toggle label="DHT" checked={!!s().dht_enabled} onChange={(on) => props.set({ dht_enabled: on })} />
        </Row>
        <Row label="Find peers through PEX" description="Peers telling each other about more peers.">
          <Toggle label="PEX" checked={!!s().pex_enabled} onChange={(on) => props.set({ pex_enabled: on })} />
        </Row>
        <Row label="Find peers on the local network">
          <Toggle
            label="Local peer discovery"
            checked={!!s().lpd_enabled}
            onChange={(on) => props.set({ lpd_enabled: on })}
          />
        </Row>
      </Group>

      <Group title="Blocklist" description="Addresses Transmission won’t connect to, from a list it downloads.">
        <Row
          label="Use a blocklist"
          description={blocklist() ?? `${(s().blocklist_size ?? 0).toLocaleString()} rules.`}
        >
          <button class={button} disabled={!s().blocklist_enabled} onClick={() => void updateBlocklist()}>
            Update Now
          </button>
          <Toggle
            label="Use a blocklist"
            checked={!!s().blocklist_enabled}
            onChange={(on) => props.set({ blocklist_enabled: on })}
          />
        </Row>
        <Row label="From">
          <TextField
            label="Blocklist URL"
            value={s().blocklist_url ?? ''}
            placeholder="https://…"
            class="w-80"
            onCommit={(v) => props.set({ blocklist_url: v })}
          />
        </Row>
      </Group>

      <p class="px-1 text-xs text-faint">Transmission {s().version}</p>
    </>
  );
}
