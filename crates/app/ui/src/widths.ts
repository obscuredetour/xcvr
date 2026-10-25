// Width breakpoints for a component, measured here instead of with CSS
// container queries. When the page is zoomed (Settings › Interface size),
// WebKit (Tauri's webview on macOS) evaluates container queries against
// zoomed sizes: px breakpoints switched too early (the edit window went to
// three columns at 200% in a window far narrower than 960 CSS px) and rem
// ones too late (a 680px details panel stayed stacked). Element sizes read
// from script are CSS px at every zoom, in every engine.

import { onCleanup } from 'solid-js';

/**
 * Keep `data-<name>` set on `el` while its content box (what a container
 * query measures) is at least that many CSS px wide. Style with Tailwind's
 * `group-data-<name>/<group>:` variants, `el` being the `group/<group>`.
 * Use as a ref: `ref={(el) => widthFlags(el, { wide: 416 })}`.
 */
export function widthFlags(el: HTMLElement, flags: Record<string, number>): void {
  const update = () => {
    const style = getComputedStyle(el);
    const width = el.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
    for (const [name, min] of Object.entries(flags)) el.toggleAttribute(`data-${name}`, width >= min);
  };
  const observer = new ResizeObserver(update);
  observer.observe(el);
  update();
  onCleanup(() => observer.disconnect());
}
