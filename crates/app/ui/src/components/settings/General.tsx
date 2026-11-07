import { createResource, createSignal, Show } from 'solid-js';
import CircleAlert from 'lucide-solid/icons/circle-alert';

import { api, errorMessage, type AppSettings, type GeneralSettings } from '../../api';
import { isMac, keys } from '../../keys';
import { toast } from '../../toast';
import { button, Callout, Group, Heading, Row, Toggle } from './parts';

/** Where the icon lives, as people call it on each platform. */
const trayName = isMac ? 'menu bar' : 'system tray';

/** This app's own settings: how it starts, closes and tells you things. Per computer. */
export function GeneralSettings() {
  const [general, { mutate }] = createResource(() => api.generalSettings());

  const save = async (run: () => Promise<GeneralSettings>) => {
    try {
      mutate(await run());
    } catch (err) {
      toast(errorMessage(err), { error: true });
    }
  };
  const change = (patch: Partial<AppSettings>) => {
    const g = general();
    if (g) void save(() => api.setGeneralSettings({ ...g.settings, ...patch }));
  };

  return (
    <>
      <Heading title="General">How Transceiver starts and closes, on this computer.</Heading>
      <Show when={general()}>
        {(g) => (
          <>
            <Group title="Starting and closing">
              <Row
                label="Launch at login"
                description={`Starts in the ${trayName} when you log in, with no window, to notify you of finished downloads.`}
              >
                <Toggle
                  label="Launch at login"
                  checked={g().launchAtLogin}
                  onChange={(on) => void save(() => api.setLaunchAtLogin(on))}
                />
              </Row>
              <Row
                label={`Keep running in the ${trayName}`}
                description={
                  g().settings.keepRunning
                    ? `Closing the window leaves Transceiver in the ${trayName}. Quit from there${isMac ? ` or with ${keys('mod+Q')}` : ''}.`
                    : 'Closing the window quits Transceiver.'
                }
              >
                <Toggle
                  label={`Keep running in the ${trayName}`}
                  checked={g().settings.keepRunning && !g().trayNote}
                  disabled={!!g().trayNote}
                  onChange={(keepRunning) => change({ keepRunning })}
                />
              </Row>
              <Show when={g().trayNote}>
                {(note) => (
                  <Callout icon={CircleAlert} tone="warning" class="my-3">
                    {note()}
                  </Callout>
                )}
              </Show>
            </Group>

            <Group title="Notifications">
              <Row
                label="When a download finishes"
                description="A notification for each torrent that finishes downloading while Transceiver is running."
              >
                <Toggle
                  label="Notify when a download finishes"
                  checked={g().settings.notifyFinished}
                  onChange={(notifyFinished) => change({ notifyFinished })}
                />
              </Row>
            </Group>

            <TmdbKey />

            <Group title="Shortcut">
              <Row
                label="Show Transceiver"
                description={`${keys(g().shortcut)} brings the window forward from any app.`}
              >
                <Toggle
                  label="Show Transceiver shortcut"
                  checked={g().settings.shortcutEnabled}
                  onChange={(shortcutEnabled) => change({ shortcutEnabled })}
                />
              </Row>
              <Show when={g().shortcutNote}>
                {(note) => (
                  <Callout icon={CircleAlert} tone="warning" class="my-3">
                    {note()}
                  </Callout>
                )}
              </Show>
            </Group>
          </>
        )}
      </Show>
    </>
  );
}

const field =
  'h-7 w-56 rounded-control border border-line bg-field px-2 text-xs outline-none select-text focus:border-focus disabled:opacity-60';

/** The TMDB key, for episode titles when renaming for a TV library (Smart Rename). */
function TmdbKey() {
  const [status, { mutate }] = createResource(() => api.tmdbStatus().catch(() => null));
  const [key, setKey] = createSignal('');
  const [busy, setBusy] = createSignal(false);

  const save = async (next: string | null) => {
    setBusy(true);
    try {
      mutate(await api.setTmdbKey(next));
      setKey('');
      toast(next ? 'TMDB key saved.' : 'TMDB key removed.');
    } catch (err) {
      toast(errorMessage(err), { error: true });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Group
      title="TV episode names"
      description="Renaming a torrent for a TV library names each episode with its title. With a TMDB key, titles come from TMDB (themoviedb.org); without one, from the file names."
    >
      <Row
        label="TMDB key"
        description={
          <>
            An API key or API read access token, free with a TMDB account (Settings › API on themoviedb.org). Kept in{' '}
            {status()?.keyStorage ?? 'the system keychain'}. Only the show you look up, and its seasons, are sent to
            TMDB. This product uses the TMDB API but is not endorsed or certified by TMDB.
          </>
        }
      >
        <Show
          when={!status()?.hasKey}
          fallback={
            <button class={button} disabled={busy()} onClick={() => void save(null)}>
              Remove Key
            </button>
          }
        >
          <input
            class={field}
            type="password"
            aria-label="TMDB key"
            autocomplete="off"
            spellcheck={false}
            placeholder="Paste a key"
            value={key()}
            onInput={(e) => setKey(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && key().trim()) void save(key().trim());
            }}
          />
          <button class={button} disabled={busy() || !key().trim()} onClick={() => void save(key().trim())}>
            {busy() ? 'Checking…' : 'Save'}
          </button>
        </Show>
      </Row>
    </Group>
  );
}
