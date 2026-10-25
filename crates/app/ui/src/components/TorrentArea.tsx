import { Match, Show, Switch } from 'solid-js';
import CircleAlert from 'lucide-solid/icons/circle-alert';
import LoaderCircle from 'lucide-solid/icons/loader-circle';
import Server from 'lucide-solid/icons/server';

import { api } from '../api';
import { isFiltered, shownTorrents } from '../browse';
import { primaryButton, secondaryButton } from '../buttons';
import { connection, described } from '../connection';
import { select, selectedHashes, selectRange, toggleSelected } from '../selection';
import { torrents } from '../torrents';
import { query } from '../view';
import { Icon } from './Icon';
import { TorrentTable } from './TorrentTable';

/**
 * Where the torrents go: what the connection is doing until it's up, then the
 * table. The torrents from before a dropped connection stay showing while it
 * reconnects, rather than flashing away.
 */
export function TorrentArea(props: { onOpenSettings(): void; onOpenInspector(): void }) {
  return (
    <Switch>
      <Match when={connection().state === 'connected' || (connection().state === 'unreachable' && torrents().length)}>
        <Show when={torrents().length} fallback={<Message title="No torrents" body="The host has no torrents yet." />}>
          <Table onOpen={() => props.onOpenInspector()} />
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
              class="grid size-14 place-items-center rounded-full"
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

/** The table of what the filters and search let through; clicks select, double-click opens the inspector. */
function Table(props: { onOpen(hash: string): void }) {
  const click = (hash: string, e: MouseEvent) => {
    if (e.metaKey || e.ctrlKey) toggleSelected(hash);
    else if (e.shiftKey) selectRange(hash, shownTorrents());
    else select(hash);
  };
  const empty = () =>
    query().trim() ? `No torrents match “${query().trim()}”.` : isFiltered() ? 'No torrents here.' : 'No torrents.';
  return (
    <TorrentTable
      rows={shownTorrents()}
      selectedHashes={selectedHashes()}
      emptyMessage={empty()}
      onSelect={click}
      onOpen={(hash) => {
        select(hash);
        props.onOpen(hash);
      }}
    />
  );
}
