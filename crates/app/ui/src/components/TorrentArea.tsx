import { For, Match, Show, Switch } from 'solid-js';
import CircleAlert from 'lucide-solid/icons/circle-alert';
import LoaderCircle from 'lucide-solid/icons/loader-circle';
import Server from 'lucide-solid/icons/server';

import { api } from '../api';
import { primaryButton, secondaryButton } from '../buttons';
import { connection, described } from '../connection';
import { formatPercent, formatSize, formatSpeed, statusLabel } from '../format';
import { torrents } from '../torrents';
import { Icon } from './Icon';

/**
 * Where the torrents go: what the connection is doing until it's up, then the
 * torrents. The list here is a stand-in for the table (phase 4).
 */
export function TorrentArea(props: { onOpenSettings(): void }) {
  return (
    <Switch>
      <Match when={connection().state === 'connected'}>
        <Show when={torrents().length} fallback={<Message title="No torrents" body="The host has no torrents yet." />}>
          <QuickList />
        </Show>
      </Match>
      <Match when={connection().state === 'notConfigured'}>
        <Message
          icon={Server}
          title="Not connected"
          body="Connect to the Transmission host to see its torrents here."
          actions={
            <button class={primaryButton} onClick={() => props.onOpenSettings()}>
              Connection Settings…
            </button>
          }
        />
      </Match>
      <Match when={connection().state === 'connecting'}>
        <Message icon={LoaderCircle} spin title={`Connecting to ${described().host}…`} />
      </Match>
      <Match when={described().tone === 'bad'}>
        <Message
          icon={CircleAlert}
          danger
          title={described().short.split(' · ')[0]!}
          body={described().detail ?? undefined}
          hint={described().hint ?? undefined}
          actions={
            <>
              <button class={primaryButton} onClick={() => void api.pollNow()}>
                Try Again
              </button>
              <button class={secondaryButton} onClick={() => props.onOpenSettings()}>
                Connection Settings…
              </button>
            </>
          }
        />
      </Match>
    </Switch>
  );
}

function Message(props: {
  icon?: typeof Server;
  spin?: boolean;
  danger?: boolean;
  title: string;
  body?: string;
  hint?: string;
  actions?: import('solid-js').JSX.Element;
}) {
  return (
    <div class="grid flex-1 place-items-center p-8">
      <div class="flex max-w-md flex-col items-center text-center select-text">
        <Show when={props.icon}>
          {(icon) => (
            <span
              class="grid size-14 place-items-center rounded-card"
              classList={{ 'bg-highlight text-accent': !props.danger, 'bg-danger/12 text-danger': props.danger }}
            >
              <Icon of={icon()} size={20} class={props.spin ? 'animate-spin' : ''} />
            </span>
          )}
        </Show>
        <h2 class="mt-4 text-lg font-semibold">{props.title}</h2>
        <Show when={props.body}>{(body) => <p class="mt-1.5 text-muted">{body()}</p>}</Show>
        <Show when={props.hint}>{(hint) => <p class="mt-2 text-[12.5px] text-muted">{hint()}</p>}</Show>
        <Show when={props.actions}>
          <div class="mt-5 flex flex-wrap justify-center gap-2">{props.actions}</div>
        </Show>
      </div>
    </div>
  );
}

/** Name, status, progress and speeds, live. Replaced by the virtualized table in phase 4. */
function QuickList() {
  return (
    <div class="min-h-0 flex-1 overflow-y-auto">
      <For each={torrents()}>
        {(t) => (
          <div class="flex h-10 items-center gap-4 border-b border-line px-4">
            <div class="min-w-0 flex-1">
              <div class="truncate font-medium">{t.name}</div>
              <div class="truncate text-xs text-muted">
                {statusLabel(t)} · {formatPercent(t.percent_done ?? 0)} of {formatSize(t.size_when_done ?? 0)}
                {t.labels?.length ? ` · ${t.labels.join(', ')}` : ''}
              </div>
            </div>
            <div class="h-1.5 w-28 flex-none overflow-hidden rounded-full bg-element">
              <div
                class="h-full rounded-full"
                classList={{
                  'bg-accent': t.status !== 6 && t.status !== 5,
                  'bg-success': t.status === 6 || t.status === 5,
                }}
                style={{ width: `${(t.percent_done ?? 0) * 100}%` }}
              />
            </div>
            <div class="w-36 flex-none text-right text-xs text-muted tabular-nums">
              ↓ {formatSpeed(t.rate_download ?? 0)} ↑ {formatSpeed(t.rate_upload ?? 0)}
            </div>
          </div>
        )}
      </For>
    </div>
  );
}
