// Pieces the inspector's tabs share: a titled section of label/value rows.

import { For, type JSX, type ParentProps, Show } from 'solid-js';

export function Section(props: ParentProps<{ title?: string; aside?: JSX.Element }>) {
  return (
    <section class="px-5 py-4 [&+&]:border-t [&+&]:border-line">
      <Show when={props.title || props.aside}>
        <div class="mb-2 flex items-center justify-between gap-2">
          <h3 class="text-[11px] font-semibold tracking-[.06em] text-muted uppercase">{props.title}</h3>
          {props.aside}
        </div>
      </Show>
      {props.children}
    </section>
  );
}

/** Label/value rows; rows whose value is empty are left out. */
export function Facts(props: { rows: [string, JSX.Element | string | null | undefined][] }) {
  return (
    <dl class="grid grid-cols-[minmax(84px,auto)_1fr] gap-x-4 gap-y-1.5 text-xs">
      <For each={props.rows.filter(([, v]) => v !== null && v !== undefined && v !== '')}>
        {([label, value]) => (
          <>
            <dt class="text-muted">{label}</dt>
            <dd class="min-w-0 break-words tabular-nums">{value}</dd>
          </>
        )}
      </For>
    </dl>
  );
}

/** "3 minutes ago", "in 12 minutes", from Unix seconds; blank for 0. */
export function relativeTime(unix: number | undefined): string {
  if (!unix) return '';
  const seconds = Math.round(unix - Date.now() / 1000);
  const fmt = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' });
  const abs = Math.abs(seconds);
  if (abs < 60) return fmt.format(seconds, 'second');
  if (abs < 3600) return fmt.format(Math.round(seconds / 60), 'minute');
  if (abs < 86400) return fmt.format(Math.round(seconds / 3600), 'hour');
  return fmt.format(Math.round(seconds / 86400), 'day');
}
