import { createEffect, createResource, createSignal, on, Show } from 'solid-js';
import CircleCheck from 'lucide-solid/icons/circle-check';
import CircleAlert from 'lucide-solid/icons/circle-alert';
import KeyRound from 'lucide-solid/icons/key-round';

import { api, asProblem, errorMessage, type Problem, type Profile, type TestResult } from '../../api';
import { primaryButton, secondaryButton } from '../../buttons';
import { confirm } from '../../confirm';
import { described } from '../../connection';
import { toast } from '../../toast';
import { Icon } from '../Icon';
import { StatusDot } from '../StatusDot';
import { Group, Heading, linkButton, Row } from './parts';

const DEFAULT_PORT = 9091;
const DEFAULT_PATH = '/transmission/rpc';

const field =
  'h-7 rounded-control border border-line bg-field px-2 text-xs outline-none select-text focus:border-focus disabled:opacity-60';

type Outcome = { ok: TestResult } | { problem: Problem };

/** Host, port and sign-in for the Transmission host; Test Connection; Save & Connect. */
export function ConnectionSettings() {
  // A failure (a locked keychain, say) shows as a toast and an empty form, not a broken page.
  const [settings, { mutate }] = createResource(() =>
    api.connectionSettings().catch((err: unknown) => {
      toast(errorMessage(err), { error: true });
      return { profile: null, hasPassword: false, keyStorage: 'the system keychain' };
    }),
  );
  const [host, setHost] = createSignal('');
  const [port, setPort] = createSignal(String(DEFAULT_PORT));
  const [path, setPath] = createSignal(DEFAULT_PATH);
  const [username, setUsername] = createSignal('');
  const [password, setPassword] = createSignal('');
  /** Remove the saved password on save. */
  const [forget, setForget] = createSignal(false);
  const [busy, setBusy] = createSignal<'test' | 'save' | 'remove' | null>(null);
  const [outcome, setOutcome] = createSignal<Outcome | null>(null);

  // Fill the form from what's saved.
  createEffect(
    on(settings, (s) => {
      const p = s?.profile;
      if (!p) return;
      setHost(p.host);
      setPort(String(p.port));
      setPath(p.path);
      setUsername(p.username);
    }),
  );

  const profile = (): Profile => ({
    id: settings()?.profile?.id ?? '',
    host: host(),
    port: Number(port()) || 0,
    path: path(),
    username: username(),
  });
  /** What to send for the password: typed (replace), '' (remove), or null (keep the saved one). */
  const passwordArg = () => (password() ? password() : forget() ? '' : null);

  const test = async () => {
    setBusy('test');
    setOutcome(null);
    try {
      setOutcome({ ok: await api.testConnection(profile(), passwordArg()) });
    } catch (err) {
      setOutcome({ problem: asProblem(err) });
    } finally {
      setBusy(null);
    }
  };

  const save = async () => {
    setBusy('save');
    setOutcome(null);
    try {
      mutate(await api.saveConnection(profile(), passwordArg()));
      setPassword('');
      setForget(false);
      toast('Saved. Connecting…');
    } catch (err) {
      setOutcome({ problem: asProblem(err) });
    } finally {
      setBusy(null);
    }
  };

  const remove = async () => {
    const host = settings()?.profile?.host ?? 'this host';
    const ok = await confirm({
      title: 'Remove this connection?',
      message: () => (
        <p>
          Transceiver disconnects from {host} and forgets it, password included. Nothing changes on the host: its
          torrents and settings stay as they are.
        </p>
      ),
      confirm: 'Remove Connection',
      danger: true,
    });
    if (!ok) return;
    setBusy('remove');
    setOutcome(null);
    try {
      mutate(await api.removeConnection());
      setHost('');
      setPort(String(DEFAULT_PORT));
      setPath(DEFAULT_PATH);
      setUsername('');
      setPassword('');
      setForget(false);
      toast('Connection removed.');
    } catch (err) {
      setOutcome({ problem: asProblem(err) });
    } finally {
      setBusy(null);
    }
  };

  const onEnter = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !busy()) void save();
  };

  const passwordPlaceholder = () =>
    forget() ? 'Will be removed' : settings()?.hasPassword ? 'Saved · type to replace' : 'None';

  return (
    <>
      <Heading title="Connection">
        The Transmission host to manage: its Remote settings (Transmission › Settings › Remote) give the port, and the
        username and password if it asks for one.
      </Heading>

      <CurrentState />

      <Group title="Host">
        <Row label="Address" description="Its name on the network (mac-mini.local) or IP address, and the RPC port.">
          <input
            class={`${field} w-52`}
            aria-label="Host"
            placeholder="mac-mini.local"
            autocomplete="off"
            spellcheck={false}
            value={host()}
            onInput={(e) => setHost(e.currentTarget.value)}
            onKeyDown={onEnter}
          />
          <span class="text-muted">:</span>
          <input
            class={`${field} w-20 tabular-nums`}
            type="number"
            min="1"
            max="65535"
            aria-label="Port"
            value={port()}
            onInput={(e) => setPort(e.currentTarget.value)}
            onKeyDown={onEnter}
          />
        </Row>
        <Row label="Username" description="Leave empty if the host doesn’t ask for a password.">
          <input
            class={`${field} w-52`}
            aria-label="Username"
            autocomplete="off"
            spellcheck={false}
            value={username()}
            onInput={(e) => setUsername(e.currentTarget.value)}
            onKeyDown={onEnter}
          />
        </Row>
        <Row
          label="Password"
          description={
            <>
              Kept in {settings()?.keyStorage ?? 'the system keychain'}.{' '}
              <Show when={settings()?.hasPassword && !password()}>
                <button class={linkButton} onClick={() => setForget(!forget())}>
                  {forget() ? 'Keep it' : 'Remove it'}
                </button>
              </Show>
            </>
          }
        >
          <input
            class={`${field} w-52`}
            type="password"
            autocomplete="new-password"
            aria-label="Password"
            placeholder={passwordPlaceholder()}
            disabled={!username().trim()}
            value={password()}
            onInput={(e) => setPassword(e.currentTarget.value)}
            onKeyDown={onEnter}
          />
        </Row>
        <Row label="RPC path" description="Only if the host changed it.">
          <input
            class={`${field} w-52`}
            aria-label="RPC path"
            spellcheck={false}
            value={path()}
            onInput={(e) => setPath(e.currentTarget.value)}
            onKeyDown={onEnter}
          />
        </Row>
      </Group>

      <div class="flex flex-wrap items-center gap-2">
        <button class={primaryButton} disabled={!!busy() || !host().trim()} onClick={() => void save()}>
          {busy() === 'save' ? 'Saving…' : 'Save & Connect'}
        </button>
        <button class={secondaryButton} disabled={!!busy() || !host().trim()} onClick={() => void test()}>
          {busy() === 'test' ? 'Testing…' : 'Test Connection'}
        </button>
        <Show when={settings()?.profile}>
          <button class={`ml-auto ${secondaryButton} text-danger`} disabled={!!busy()} onClick={() => void remove()}>
            {busy() === 'remove' ? 'Removing…' : 'Remove Connection'}
          </button>
        </Show>
      </div>

      <Show when={outcome()}>{(o) => <OutcomeNote outcome={o()} />}</Show>
    </>
  );
}

/** What Test Connection (or a failed save) found. */
function OutcomeNote(props: { outcome: Outcome }) {
  return (
    <div class="mt-4 max-w-[62ch] text-[12.5px]/[1.5]" role="status">
      <Show
        when={'ok' in props.outcome && props.outcome.ok}
        fallback={
          <Show when={'problem' in props.outcome && props.outcome.problem}>
            {(p) => (
              <div class="flex gap-2 text-danger">
                <Icon of={CircleAlert} class="mt-0.5 flex-none" />
                <div>
                  <p>{p().message}</p>
                  <Show when={p().hint}>{(h) => <p class="mt-1 text-muted">{h()}</p>}</Show>
                </div>
              </div>
            )}
          </Show>
        }
      >
        {(ok) => (
          <div class="flex items-center gap-2 text-success">
            <Icon of={CircleCheck} class="flex-none" />
            Transmission {ok().version.split(' ')[0]} answered, with {ok().torrentCount}{' '}
            {ok().torrentCount === 1 ? 'torrent' : 'torrents'}.
          </div>
        )}
      </Show>
    </div>
  );
}

/** The live connection, above the form. */
function CurrentState() {
  const d = described;
  return (
    <Show when={d().host}>
      <div class="mb-6 flex items-start gap-3 rounded-card border border-line bg-element/60 p-4">
        <span class="grid size-8 flex-none place-items-center rounded-full bg-highlight text-accent">
          <Icon of={KeyRound} size={16} />
        </span>
        <div class="min-w-0 flex-1 text-[12.5px]/[1.5]">
          <div class="flex items-center gap-2 font-semibold">
            <StatusDot tone={d().tone} />
            {d().host}
          </div>
          <div class="text-muted">
            {d().tone === 'bad' ? d().detail : [d().short, d().detail].filter(Boolean).join(' · ')}
          </div>
          <Show when={d().hint}>{(hint) => <div class="mt-1 text-muted">{hint()}</div>}</Show>
          <Show when={d().retryable}>
            <button class={`mt-1.5 ${linkButton}`} onClick={() => void api.pollNow()}>
              Try again now
            </button>
          </Show>
        </div>
      </div>
    </Show>
  );
}
