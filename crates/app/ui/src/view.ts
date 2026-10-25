// View preferences (per device).

import { createSignal } from 'solid-js';

import { storage } from './storage';

/** Alternating row colours in the torrent table; on unless turned off. */
const [zebraRows, setZebraRowsSignal] = createSignal(storage.get('tableZebra') !== 'off');
export { zebraRows };

export function setZebraRows(on: boolean): void {
  setZebraRowsSignal(on);
  storage.set('tableZebra', on ? 'on' : 'off');
}

/** Width of the inspector in CSS px (it also never squeezes the table below 320px). */
export const INSPECTOR_WIDTH = { min: 300, max: 680, initial: 380 } as const;

const clampInspector = (n: number) => Math.min(INSPECTOR_WIDTH.max, Math.max(INSPECTOR_WIDTH.min, Math.round(n)));
const storedInspector = Number(storage.get('inspectorWidth'));
const [inspectorWidth, setInspectorWidthSignal] = createSignal(
  storedInspector ? clampInspector(storedInspector) : INSPECTOR_WIDTH.initial,
);
export { inspectorWidth };

/** Set the inspector's width; `save` stores it (at the end of a drag, not on every move). */
export function setInspectorWidth(n: number, save = true): void {
  const w = clampInspector(n);
  setInspectorWidthSignal(w);
  if (save) storage.set('inspectorWidth', String(w));
}

/** The inspector; shown unless turned off. */
const [inspectorOpen, setInspectorOpenSignal] = createSignal(storage.get('inspector') !== 'off');
export { inspectorOpen };

export function setInspectorOpen(open: boolean): void {
  setInspectorOpenSignal(open);
  storage.set('inspector', open ? 'on' : 'off');
}

/** The filters sidebar; shown unless turned off. */
const [sidebarOpen, setSidebarOpenSignal] = createSignal(storage.get('sidebar') !== 'off');
export { sidebarOpen };

export function setSidebarOpen(open: boolean): void {
  setSidebarOpenSignal(open);
  storage.set('sidebar', open ? 'on' : 'off');
}
