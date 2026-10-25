import { createMemo, createSignal, For, Show } from 'solid-js';

type Props = {
  value: string;
  onInput(value: string): void;
  /** Names to suggest. */
  options: readonly string[];
  /**
   * For fields holding several names ("A & B", "x, y"): the separator. The
   * name being typed (after the last separator) is completed, and names
   * already entered aren't suggested again. Omit for single-value fields.
   */
  separator?: '&' | ',';
  class?: string;
  id?: string;
  placeholder?: string;
};

const MAX = 8;

/** A text input that suggests existing names as you type. */
export function Autocomplete(props: Props) {
  const [open, setOpen] = createSignal(false);
  const [active, setActive] = createSignal(0);

  const parts = () => (props.separator ? props.value.split(props.separator) : [props.value]);
  const current = () => (parts().at(-1) ?? '').trim();

  const matches = createMemo(() => {
    const q = current().toLowerCase();
    if (!q) return [];
    const taken = new Set(
      parts()
        .slice(0, -1)
        .map((p) => p.trim().toLowerCase()),
    );
    const usable = props.options.filter((o) => !taken.has(o.toLowerCase()) && o.toLowerCase() !== q);
    // Names starting with the text first, then names containing it.
    const starts = usable.filter((o) => o.toLowerCase().startsWith(q));
    const contains = usable.filter((o) => !o.toLowerCase().startsWith(q) && o.toLowerCase().includes(q));
    return [...starts, ...contains].slice(0, MAX);
  });
  const showing = () => open() && matches().length > 0;

  const accept = (name: string) => {
    if (props.separator) {
      const done = parts()
        .slice(0, -1)
        .map((p) => p.trim());
      props.onInput([...done, name].join(props.separator === '&' ? ' & ' : ', '));
    } else {
      props.onInput(name);
    }
    setOpen(false);
  };

  return (
    <div class="relative">
      <input
        id={props.id}
        class={props.class}
        placeholder={props.placeholder}
        autocomplete="off"
        role="combobox"
        aria-autocomplete="list"
        aria-expanded={showing()}
        value={props.value}
        onInput={(e) => {
          props.onInput(e.currentTarget.value);
          setActive(0);
          setOpen(true);
        }}
        onBlur={() => setOpen(false)}
        // Native listener: these keys mustn't reach the dialog (Esc would close it).
        on:keydown={(e) => {
          if (!showing()) return;
          const n = matches().length;
          if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
            e.preventDefault();
            setActive((i) => (i + (e.key === 'ArrowDown' ? 1 : n - 1)) % n);
          } else if (e.key === 'Enter' || e.key === 'Tab') {
            const pick = matches()[active()];
            if (pick) {
              e.preventDefault();
              accept(pick);
            }
          } else if (e.key === 'Escape') {
            e.preventDefault();
            e.stopPropagation();
            setOpen(false);
          }
        }}
      />
      <Show when={showing()}>
        <ul
          role="listbox"
          class="absolute top-full right-0 left-0 z-50 mt-1 max-h-56 overflow-y-auto rounded-menu border border-line bg-popover p-1.5 shadow-lift"
        >
          <For each={matches()}>
            {(name, i) => (
              <li
                role="option"
                aria-selected={i() === active()}
                class="cursor-default truncate rounded-control px-2.5 py-1.5 aria-selected:bg-highlight aria-selected:text-accent"
                // mousedown (not click), so the input keeps focus.
                onMouseDown={(e) => {
                  e.preventDefault();
                  accept(name);
                }}
                onMouseEnter={() => setActive(i())}
              >
                {name}
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}
