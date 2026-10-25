import { getVersion } from '@tauri-apps/api/app';
import { createResource, Show } from 'solid-js';

export function About() {
  const [version] = createResource(() => getVersion().catch(() => null));
  return (
    <div class="flex flex-col items-center pt-10 text-center">
      <img src="/icon.png" alt="" width="112" height="112" />
      <h3 class="mt-4 font-serif text-2xl font-semibold">Transceiver</h3>
      <Show when={version()}>{(v) => <div class="mt-1 text-xs text-muted">Version {v()}</div>}</Show>
      <p class="mt-4 max-w-sm text-sm text-muted">A desktop client for Transmission.</p>
      <p class="mt-4 text-xs text-faint">Licensed under the GNU GPL v3 or later.</p>
    </div>
  );
}
