import { getVersion } from '@tauri-apps/api/app';
import { createResource, createSignal, Show } from 'solid-js';

import { api, errorMessage } from '../../api';
import { toast } from '../../toast';
import { available, checkForUpdate, installing, installUpdate } from '../../updates';
import { button, Group, linkButton, Row, Toggle } from './parts';

export function About() {
  const [version] = createResource(() => getVersion().catch(() => null));
  return (
    <>
      <div class="flex flex-col items-center pt-10 text-center">
        <img src="/icon.png" alt="" width="112" height="112" />
        <h3 class="mt-4 font-serif text-2xl font-semibold">Transceiver</h3>
        <Show when={version()}>{(v) => <div class="mt-1 text-xs text-muted">Version {v()}</div>}</Show>
        <p class="mt-4 max-w-sm text-sm text-muted">A desktop client for Transmission.</p>
        <p class="mt-4 text-xs text-faint">Released under the MIT License.</p>
      </div>
      <Updates />
    </>
  );
}

/** Checking for and installing a newer version, where this copy can update itself. */
function Updates() {
  const [status, { mutate }] = createResource(() => api.updateStatus().catch(() => undefined));
  const [checking, setChecking] = createSignal(false);
  /** After a check that found nothing. */
  const [upToDate, setUpToDate] = createSignal(false);

  const setAuto = async (on: boolean) => {
    try {
      mutate(await api.setAutoUpdate(on));
    } catch (err) {
      toast(errorMessage(err), { error: true });
    }
  };
  const check = async () => {
    setChecking(true);
    setUpToDate(false);
    try {
      setUpToDate(!(await checkForUpdate()));
    } catch (err) {
      toast(errorMessage(err), { error: true });
    } finally {
      setChecking(false);
    }
  };

  return (
    <Show when={status()}>
      {(s) => (
        <Group title="Updates" class="mx-auto mt-10 max-w-xl">
          <Show when={s().supported} fallback={<p class="py-3 text-[12.5px]/[1.5] text-muted">{s().unsupported}</p>}>
            <Row label="Check for updates automatically" description="Soon after Transceiver starts, then once a day.">
              <Toggle
                label="Check for updates automatically"
                checked={s().autoCheck}
                onChange={(on) => void setAuto(on)}
              />
            </Row>
            <Row
              label={available() ? `Transceiver ${available()!.version} is available` : 'This version'}
              description={
                installing() ??
                (available() ? (
                  <button class={linkButton} onClick={() => void api.openReleaseNotes(available()!.version)}>
                    What's new
                  </button>
                ) : upToDate() ? (
                  'Transceiver is up to date.'
                ) : undefined)
              }
            >
              <Show
                when={available()}
                fallback={
                  <button class={button} disabled={checking()} onClick={() => void check()}>
                    {checking() ? 'Checking…' : 'Check for Updates'}
                  </button>
                }
              >
                <button class={button} disabled={!!installing()} onClick={() => void installUpdate()}>
                  Update and Restart
                </button>
              </Show>
            </Row>
          </Show>
        </Group>
      )}
    </Show>
  );
}
