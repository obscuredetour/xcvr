// The pieces Settings pages are made of: a page heading, groups of rows,
// switches, segmented choices, callouts.

import type { Component } from 'solid-js';
import { For, type JSX, type ParentProps, Show } from 'solid-js';
import type { LucideProps } from 'lucide-solid';

import { smallButton } from '../../buttons';
import { Icon } from '../Icon';

export const button = smallButton;

/** Help text under a label, and the paragraphs that introduce a page or group. */
export const help = 'text-[12.5px]/[1.5] text-muted';

/** A page's title, and what the page is for (one short paragraph each). */
export function Heading(props: ParentProps<{ title: string }>) {
  return (
    <header class="mb-6">
      <h3 class="text-lg font-semibold">{props.title}</h3>
      <Show when={props.children}>
        <div class={`mt-1.5 max-w-[62ch] space-y-1.5 ${help}`}>{props.children}</div>
      </Show>
    </header>
  );
}

/**
 * Related settings in a box: rows inside, divided by lines; above it, an
 * optional title and introduction, and `aside` (buttons) at the right.
 */
export function Group(
  props: ParentProps<{ title?: string; description?: JSX.Element; aside?: JSX.Element; class?: string }>,
) {
  return (
    <section class={`mb-7 ${props.class ?? ''}`}>
      <Show when={props.title || props.aside}>
        <div class="mb-2 flex items-end justify-between gap-4 px-1">
          <div class="min-w-0">
            <Show when={props.title}>
              <h4 class="font-semibold">{props.title}</h4>
            </Show>
            <Show when={props.description}>
              <div class={`mt-0.5 max-w-[62ch] space-y-1 ${help}`}>{props.description}</div>
            </Show>
          </div>
          <Show when={props.aside}>
            <div class="flex flex-none items-center gap-2">{props.aside}</div>
          </Show>
        </div>
      </Show>
      <div class="rounded-card border border-line bg-bg/40 px-4">{props.children}</div>
    </section>
  );
}

/**
 * A labelled setting: name and help text on the left, its control on the
 * right; `stacked` puts the help text under both, for narrow columns.
 * `joined` drops the line under it, for a row continued by what follows (an
 * API key's field).
 */
export function Row(
  props: ParentProps<{ label: JSX.Element; description?: JSX.Element; joined?: boolean; stacked?: boolean }>,
) {
  const description = () => (
    <Show when={props.description}>
      <div class={`mt-0.5 max-w-[62ch] ${help}`}>{props.description}</div>
    </Show>
  );
  return (
    <div class="py-3" classList={{ 'border-b border-line last:border-b-0': !props.joined }}>
      <div class="flex items-center justify-between gap-6">
        <div class="min-w-0">
          <div class="font-medium" classList={{ 'whitespace-nowrap': props.stacked }}>
            {props.label}
          </div>
          <Show when={!props.stacked}>{description()}</Show>
        </div>
        <Show when={props.children}>
          <div class="flex flex-none items-center gap-2">{props.children}</div>
        </Show>
      </div>
      <Show when={props.stacked}>{description()}</Show>
    </div>
  );
}

/** An on/off switch: a checkbox drawn as a track and knob. */
export function Toggle(props: { checked: boolean; label: string; disabled?: boolean; onChange(on: boolean): void }) {
  return (
    <label class="relative inline-flex flex-none cursor-pointer has-disabled:cursor-default has-disabled:opacity-50">
      <input
        type="checkbox"
        role="switch"
        class="peer sr-only"
        aria-label={props.label}
        checked={props.checked}
        disabled={props.disabled}
        onChange={(e) => props.onChange(e.currentTarget.checked)}
      />
      <span class="h-[18px] w-8 rounded-full bg-element-hover shadow-[inset_0_0_0_1px_var(--color-line)] transition-colors peer-checked:bg-accent peer-checked:shadow-none peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-focus" />
      <span class="pointer-events-none absolute top-0.5 left-0.5 size-3.5 rounded-full bg-white shadow-[0_1px_2px_rgb(0_0_0/0.3)] transition-transform peer-checked:translate-x-3.5" />
    </label>
  );
}

/** One of a few choices, as a segmented control. */
export function Segmented<T extends string>(props: {
  label: string;
  value: T;
  options: readonly (readonly [T, string])[];
  onChange(value: T): void;
}) {
  return (
    <div class="flex rounded-control bg-element p-0.5" role="group" aria-label={props.label}>
      <For each={props.options}>
        {([value, label]) => (
          <button
            class="rounded-inset px-3 py-1 text-xs font-medium text-muted aria-pressed:bg-popover aria-pressed:text-fg aria-pressed:shadow-card"
            aria-pressed={props.value === value}
            onClick={() => props.onChange(value)}
          >
            {label}
          </button>
        )}
      </For>
    </div>
  );
}

/** A box that stands apart from the settings around it: what's stored where, a warning. */
export function Callout(
  props: ParentProps<{ icon: Component<LucideProps>; title?: string; tone?: 'info' | 'warning'; class?: string }>,
) {
  return (
    <div
      class={`flex gap-3 rounded-card border p-4 ${props.tone === 'warning' ? 'border-warning/30 bg-warning/8' : 'border-line bg-element/60'} ${props.class ?? ''}`}
    >
      <span
        class="grid size-8 flex-none place-items-center rounded-control"
        classList={{
          'bg-warning/15 text-warning': props.tone === 'warning',
          'bg-highlight text-accent': props.tone !== 'warning',
        }}
      >
        <Icon of={props.icon} size={16} />
      </span>
      <div class="min-w-0 flex-1">
        <Show when={props.title}>
          <div class="font-semibold">{props.title}</div>
        </Show>
        <div class={`mt-0.5 space-y-1.5 ${help}`}>{props.children}</div>
      </div>
    </div>
  );
}

/** A small uppercase label over a list (Installed, From plugins…). */
export function Label(props: ParentProps<{ class?: string }>) {
  return (
    <h5 class={`mb-1.5 px-1 text-[11px] font-semibold tracking-[.06em] text-muted uppercase ${props.class ?? ''}`}>
      {props.children}
    </h5>
  );
}

/** A plugin's or an action's log, as Settings shows it. */
export function Log(props: { lines: string[]; class?: string }) {
  return (
    <pre
      class={`max-h-64 overflow-auto rounded-control border border-line bg-bg/50 p-2 text-[11px] whitespace-pre-wrap select-text ${props.class ?? ''}`}
    >
      {props.lines.length ? props.lines.join('\n') : 'Nothing logged yet.'}
    </pre>
  );
}

/** A link-styled button, for actions inside help text. */
export const linkButton = 'cursor-pointer text-link hover:underline disabled:cursor-default disabled:opacity-50';

/** A description as a sentence (a full stop added when it has none). */
export function sentence(s: string): string {
  const t = s.trim();
  return t ? t.replace(/[^.!?]$/, '$&.') : '';
}
