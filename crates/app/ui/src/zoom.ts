// Interface size: the whole window zoomed in steps (Settings › Appearance,
// ⌘+ / ⌘− / ⌘0). The webview's own zoom, so everything scales together,
// pixel sizes included. Remembered per device.

import { LogicalSize } from '@tauri-apps/api/dpi';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { createSignal } from 'solid-js';

import { confirm } from './confirm';

import { storage } from './storage';

export const ZOOM_STEPS = [0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2] as const;

const nearest = (z: number) =>
  ZOOM_STEPS.reduce((best, s) => (Math.abs(s - z) < Math.abs(best - z) ? s : best), 1 as number);

const stored = Number(storage.get('zoom'));
const [zoom, setZoomSignal] = createSignal<number>(stored ? nearest(stored) : 1);
export { zoom };

/** The window's minimum size at 100%, as in tauri.conf.json. */
const MIN_WINDOW = { width: 640, height: 440 };

function apply(z: number): void {
  // CSS reads it to keep the macOS title bar the native height (see styles.css).
  document.documentElement.style.setProperty('--zoom', String(z));
  // Outside the app (the UI harness, a browser) there's no webview to zoom.
  try {
    void getCurrentWebview()
      .setZoom(z)
      .catch(() => {});
    // Keep the layout at least as wide as at 100%'s minimum, so zooming in
    // can't squeeze the toolbar; never more than the screen, so it still fits.
    const min = new LogicalSize(
      Math.min(MIN_WINDOW.width * z, screen.availWidth),
      Math.min(MIN_WINDOW.height * z, screen.availHeight),
    );
    void getCurrentWindow()
      .setMinSize(min)
      .catch(() => {});
  } catch {
    // Not running in Tauri.
  }
}

export function setZoom(z: number): void {
  const step = nearest(z);
  setZoomSignal(step);
  storage.set('zoom', String(step));
  apply(step);
}

/** One step larger (+1) or smaller (−1). */
export function stepZoom(by: 1 | -1): void {
  const i = ZOOM_STEPS.indexOf(zoom() as (typeof ZOOM_STEPS)[number]);
  setZoom(ZOOM_STEPS[Math.min(ZOOM_STEPS.length - 1, Math.max(0, i + by))]!);
}

export const resetZoom = () => setZoom(1);

/**
 * Try a size from Settings: applied at once, then kept only if the user says
 * so within 15 seconds (as with a display resolution), since a size that's
 * too large can make the controls hard to reach. The shortcuts step straight
 * away; they're one step at a time and ⌘0 undoes them.
 */
export async function tryZoom(z: number): Promise<void> {
  const step = nearest(z);
  const before = zoom();
  if (step === before) return;
  setZoomSignal(step);
  apply(step);
  const keep = await confirm({
    title: 'Keep this interface size?',
    message: () => `The interface is now ${zoomLabel(step)}. It goes back to ${zoomLabel(before)} unless you keep it.`,
    confirm: 'Keep',
    cancel: 'Revert',
    timeout: 15,
  });
  if (keep) setZoom(step);
  else if (zoom() === step) setZoom(before);
}

export const zoomLabel = (z: number) => `${Math.round(z * 100)}%`;

// Before the first render, so the window doesn't flash at 100%.
apply(zoom());
