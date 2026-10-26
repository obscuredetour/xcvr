import { createResource, Show } from 'solid-js';
import CircleAlert from 'lucide-solid/icons/circle-alert';

import { api, errorMessage, type AppSettings, type GeneralSettings } from '../../api';
import { isMac, keys } from '../../keys';
import { toast } from '../../toast';
import { Callout, Group, Heading, Row, Toggle } from './parts';

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
                  checked={g().settings.keepRunning}
                  onChange={(keepRunning) => change({ keepRunning })}
                />
              </Row>
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
                  <Callout icon={CircleAlert} tone="warning" class="mt-3">
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
