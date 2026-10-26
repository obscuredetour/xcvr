// Fields that send their value when it's done (Enter, or leaving the field),
// not on every keystroke, and follow the value they show while not being typed
// in (it can change underneath them: a poll, another computer).

import { createEffect, createSignal, on } from 'solid-js';

const base =
  'h-7 rounded-control border border-line bg-field px-2 text-xs outline-none select-text focus:border-focus disabled:opacity-50';

/** A number that's sent when it changed and makes sense (finite, not negative); otherwise put back. */
export function NumberField(props: {
  label: string;
  value: number;
  step: number;
  min?: number;
  disabled?: boolean;
  class?: string;
  onCommit(n: number): void;
}) {
  const [text, setText] = createSignal(String(props.value));
  let editing = false;
  createEffect(
    on(
      () => props.value,
      (v) => !editing && setText(String(v)),
    ),
  );
  const commit = () => {
    editing = false;
    const n = Number(text());
    if (text().trim() !== '' && Number.isFinite(n) && n >= (props.min ?? 0) && n !== props.value) props.onCommit(n);
    else setText(String(props.value));
  };
  return (
    <input
      type="number"
      min={props.min ?? 0}
      step={props.step}
      aria-label={props.label}
      disabled={props.disabled}
      class={`${base} text-right tabular-nums ${props.class ?? 'w-20'}`}
      value={text()}
      onFocus={() => (editing = true)}
      onInput={(e) => setText(e.currentTarget.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        e.stopPropagation();
        if (e.key === 'Enter') e.currentTarget.blur();
        else if (e.key === 'Escape') {
          setText(String(props.value));
          e.currentTarget.blur();
        }
      }}
    />
  );
}

/** A line of text (a folder, a URL) that's sent when it changed. */
export function TextField(props: {
  label: string;
  value: string;
  placeholder?: string;
  disabled?: boolean;
  class?: string;
  onCommit(text: string): void;
}) {
  const [text, setText] = createSignal(props.value);
  let editing = false;
  createEffect(
    on(
      () => props.value,
      (v) => !editing && setText(v),
    ),
  );
  const commit = () => {
    editing = false;
    if (text().trim() !== props.value) props.onCommit(text().trim());
  };
  return (
    <input
      aria-label={props.label}
      placeholder={props.placeholder}
      spellcheck={false}
      autocomplete="off"
      disabled={props.disabled}
      class={`${base} ${props.class ?? 'w-64'}`}
      value={text()}
      onFocus={() => (editing = true)}
      onInput={(e) => setText(e.currentTarget.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        e.stopPropagation();
        if (e.key === 'Enter') e.currentTarget.blur();
        else if (e.key === 'Escape') {
          setText(props.value);
          e.currentTarget.blur();
        }
      }}
    />
  );
}
