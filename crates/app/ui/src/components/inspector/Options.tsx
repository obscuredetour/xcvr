import { createEffect, createSignal, on, Show } from 'solid-js';

import { api, errorMessage, type Torrent, type TorrentSet } from '../../api';
import { toast } from '../../toast';
import { Segmented, Toggle } from '../settings/parts';
import { Section } from './parts';

const select =
  'h-7 rounded-control border border-line bg-field px-2 text-xs outline-none focus:border-focus disabled:opacity-50';

/** This torrent's own limits and priorities. Each change applies at once. */
export function OptionsTab(props: { t: Torrent }) {
  const t = () => props.t;
  const set = async (change: TorrentSet) => {
    try {
      await api.setTorrents([t().hash_string!], change);
    } catch (err) {
      toast(errorMessage(err), { error: true });
    }
  };

  return (
    <>
      <Section title="Bandwidth">
        <Row label="Priority">
          <Segmented
            label="Priority"
            value={String(t().bandwidth_priority ?? 0) as '-1' | '0' | '1'}
            options={[
              ['-1', 'Low'],
              ['0', 'Normal'],
              ['1', 'High'],
            ]}
            onChange={(v) => void set({ bandwidth_priority: Number(v) })}
          />
        </Row>
        <LimitRow
          label="Download limit"
          on={!!t().download_limited}
          value={t().download_limit ?? 0}
          unit="kB/s"
          onToggle={(on) => void set({ download_limited: on })}
          onValue={(n) => void set({ download_limit: n, download_limited: true })}
        />
        <LimitRow
          label="Upload limit"
          on={!!t().upload_limited}
          value={t().upload_limit ?? 0}
          unit="kB/s"
          onToggle={(on) => void set({ upload_limited: on })}
          onValue={(n) => void set({ upload_limit: n, upload_limited: true })}
        />
        <Row label="Follow the global limits" hint="Transmission’s own speed limits apply to it too.">
          <Toggle
            label="Follow the global limits"
            checked={!!t().honors_session_limits}
            onChange={(on) => void set({ honors_session_limits: on })}
          />
        </Row>
      </Section>

      <Section title="Seeding">
        <ModeRow
          label="Stop at ratio"
          mode={t().seed_ratio_mode ?? 0}
          value={t().seed_ratio_limit ?? 2}
          step={0.1}
          unit=""
          labels={['As Transmission is set', 'At this ratio', 'Never']}
          onMode={(m) => void set({ seed_ratio_mode: m })}
          onValue={(n) => void set({ seed_ratio_limit: n, seed_ratio_mode: 1 })}
        />
        <ModeRow
          label="Stop when idle"
          mode={t().seed_idle_mode ?? 0}
          value={t().seed_idle_limit ?? 30}
          step={1}
          unit="min"
          labels={['As Transmission is set', 'After this long', 'Never']}
          onMode={(m) => void set({ seed_idle_mode: m })}
          onValue={(n) => void set({ seed_idle_limit: Math.round(n), seed_idle_mode: 1 })}
        />
      </Section>

      <Section title="Peers and pieces">
        <Row label="Most peers">
          <NumberField
            label="Most peers"
            value={t().peer_limit ?? 50}
            step={1}
            onCommit={(n) => void set({ peer_limit: Math.max(1, Math.round(n)) })}
          />
        </Row>
        <Row label="Download in order" hint="Pieces from start to end, so video can play while it downloads.">
          <Toggle
            label="Download in order"
            checked={!!t().sequential_download}
            onChange={(on) => void set({ sequential_download: on })}
          />
        </Row>
      </Section>
    </>
  );
}

function Row(props: { label: string; hint?: string; children: import('solid-js').JSX.Element }) {
  return (
    <div class="flex items-center justify-between gap-3 py-1.5">
      <div class="min-w-0">
        <div class="text-xs">{props.label}</div>
        <Show when={props.hint}>
          <div class="text-[11px] text-muted">{props.hint}</div>
        </Show>
      </div>
      <div class="flex flex-none items-center gap-2">{props.children}</div>
    </div>
  );
}

/** A number that's sent on Enter or leaving the field, when it changed and makes sense. */
function NumberField(props: {
  label: string;
  value: number;
  step: number;
  disabled?: boolean;
  onCommit(n: number): void;
}) {
  const [text, setText] = createSignal(String(props.value));
  // Follow the torrent while this isn't being typed in.
  let editing = false;
  createEffect(
    on(
      () => props.value,
      (v) => !editing && setText(String(v)),
    ),
  );
  const commit = () => {
    editing = false;
    const n = Number(text());
    if (text().trim() !== '' && Number.isFinite(n) && n >= 0 && n !== props.value) props.onCommit(n);
    else setText(String(props.value));
  };
  return (
    <input
      type="number"
      min="0"
      step={props.step}
      aria-label={props.label}
      disabled={props.disabled}
      class="h-7 w-20 rounded-control border border-line bg-field px-2 text-right text-xs tabular-nums outline-none focus:border-focus disabled:opacity-50"
      value={text()}
      onFocus={() => (editing = true)}
      onInput={(e) => setText(e.currentTarget.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === 'Enter') e.currentTarget.blur();
        else if (e.key === 'Escape') {
          setText(String(props.value));
          e.currentTarget.blur();
        }
      }}
    />
  );
}

/** A speed limit: on or off, and how much. */
function LimitRow(props: {
  label: string;
  on: boolean;
  value: number;
  unit: string;
  onToggle(on: boolean): void;
  onValue(n: number): void;
}) {
  return (
    <Row label={props.label}>
      <NumberField label={props.label} value={props.value} step={10} disabled={!props.on} onCommit={props.onValue} />
      <span class="w-8 text-[11px] text-muted">{props.unit}</span>
      <Toggle label={`${props.label} on`} checked={props.on} onChange={props.onToggle} />
    </Row>
  );
}

/** A seeding limit: Transmission's default, this torrent's own value, or never. */
function ModeRow(props: {
  label: string;
  mode: number;
  value: number;
  step: number;
  unit: string;
  labels: [string, string, string];
  onMode(m: number): void;
  onValue(n: number): void;
}) {
  return (
    <div class="py-1.5">
      <div class="flex items-center justify-between gap-3">
        <div class="text-xs">{props.label}</div>
        <select
          class={select}
          aria-label={props.label}
          value={String(props.mode)}
          onChange={(e) => props.onMode(Number(e.currentTarget.value))}
        >
          {props.labels.map((label, i) => (
            <option value={String(i)}>{label}</option>
          ))}
        </select>
      </div>
      <Show when={props.mode === 1}>
        <div class="mt-1.5 flex items-center justify-end gap-2">
          <NumberField label={`${props.label}: value`} value={props.value} step={props.step} onCommit={props.onValue} />
          <span class="w-8 text-[11px] text-muted">{props.unit}</span>
        </div>
      </Show>
    </div>
  );
}
