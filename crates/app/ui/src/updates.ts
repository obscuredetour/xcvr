// Updates: the newer version a check found, and installing it. Checked soon
// after launch and once a day while Transceiver runs, unless that's turned off
// in Settings › About; a toast says when one's found, and Settings › About
// installs it (updates.rs does the work).

import { createSignal } from 'solid-js';

import { api, errorMessage, onUpdateProgress, type UpdateInfo } from './api';
import { toast } from './toast';

const DAY = 24 * 60 * 60 * 1000;
/** After launch, so the check doesn't compete with connecting. */
const FIRST_CHECK = 10_000;

const [available, setAvailable] = createSignal<UpdateInfo | null>(null);
/** While installing: what it's doing ("Downloading … 12 MB of 18 MB"). */
const [installing, setInstalling] = createSignal<string | null>(null);
export { available, installing };

/** The version a toast already announced, so a daily check doesn't repeat it. */
let announced: string | null = null;

/** Look for a newer version: null when this is the latest. Throws when it can't look. */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  const found = await api.checkForUpdate();
  setAvailable(found);
  return found;
}

/** Check soon, then once a day, whenever automatic checks are on at the time. Call once, at launch. */
export function scheduleUpdateChecks(): void {
  const check = async () => {
    try {
      const s = await api.updateStatus();
      if (!s.supported || !s.autoCheck || installing()) return;
      const found = await checkForUpdate();
      if (found && found.version !== announced) {
        announced = found.version;
        toast(`Transceiver ${found.version} is available: Settings › About.`, { duration: 8000 });
      }
    } catch {
      // Offline, or GitHub unreachable: tomorrow's check may do better.
    }
  };
  setTimeout(() => {
    void check();
    setInterval(() => void check(), DAY);
  }, FIRST_CHECK);
}

const mb = (bytes: number) => `${Math.max(1, Math.round(bytes / 1e6))} MB`;

/** Download and install the update found, then restart into it. */
export async function installUpdate(): Promise<void> {
  const u = available();
  if (!u || installing()) return;
  const name = `Transceiver ${u.version}`;
  setInstalling(`Downloading ${name}…`);
  const unlisten = await onUpdateProgress((p) => {
    if (p.phase === 'downloading') {
      setInstalling(p.total ? `Downloading ${name}: ${mb(p.done)} of ${mb(p.total)}` : `Downloading ${name}…`);
    } else setInstalling(`Installing ${name}…`);
  });
  try {
    await api.installUpdate();
    // Transceiver restarts into the new version from here.
    setInstalling(`Restarting into ${name}…`);
  } catch (err) {
    toast(errorMessage(err), { error: true });
    setInstalling(null);
  } finally {
    unlisten();
  }
}
