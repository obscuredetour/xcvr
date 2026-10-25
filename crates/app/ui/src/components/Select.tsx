import { createEffect, type JSX, splitProps } from 'solid-js';
import ChevronDown from 'lucide-solid/icons/chevron-down';

import { Icon } from './Icon';

/**
 * A native `<select>` with the app's own arrow. WebKit draws a different one
 * in a styled select on each macOS (a ↕ stepper on macOS 15), so its native
 * look is turned off; the menu that opens is still the system's. `class` is
 * for size, text and left padding: the right padding makes room for the arrow.
 * `wide` fills the container's width, like the text inputs beside it.
 *
 * `value` is applied once the options are there (and again when it
 * changes): set before them, as a spread does, a select shows its first
 * option whatever the value.
 */
export function Select(props: JSX.SelectHTMLAttributes<HTMLSelectElement> & { wide?: boolean }) {
  const [local, rest] = splitProps(props, ['class', 'children', 'wide']);
  let select!: HTMLSelectElement;
  createEffect(() => {
    if (rest.value !== undefined) select.value = String(rest.value);
  });
  return (
    <span class="relative items-center" classList={{ 'inline-flex': !local.wide, 'flex w-full': local.wide }}>
      <select
        ref={select}
        {...rest}
        class={`appearance-none rounded-control border border-line bg-field pr-6 ${local.wide ? 'w-full' : ''} ${local.class ?? ''}`}
      >
        {local.children}
      </select>
      <Icon of={ChevronDown} size={12} class="pointer-events-none absolute right-1.5 text-muted" />
    </span>
  );
}
