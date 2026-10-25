import { For, Show } from 'solid-js';

import { accentChoice, currentAccents, currentTheme, ownAccent, setAccent, themeAttrs } from '../theme';

// A ring, not an outline: `outline-none` would keep any outline from showing.
const SWATCH =
  'grid size-6 cursor-pointer place-items-center rounded-full outline-none focus-visible:ring-2 focus-visible:ring-focus';

/** The checked mark: a dot that's dark on light colours and light on dark ones. */
function Dot(props: { on: string }) {
  return (
    <span
      class="size-2 rounded-full"
      style={{ background: `oklch(from ${props.on} clamp(0.2, (0.64 - l) * 1000, 0.99) 0.02 h)` }}
    />
  );
}

/**
 * The theme in use's own accent, then the others it offers, as a row of
 * swatches (its own isn't repeated among them). `menu` makes them menu items
 * (inside a Menu) rather than radio buttons.
 */
export function AccentPicker(props: { menu?: boolean }) {
  const role = props.menu ? 'menuitemradio' : 'radio';
  return (
    <div
      class="flex flex-wrap gap-1.5"
      classList={{ 'max-w-[244px] px-2 pt-0.5 pb-1.5': props.menu }}
      role={props.menu ? 'group' : 'radiogroup'}
      aria-label="Accent colour"
    >
      <button
        role={role}
        aria-checked={accentChoice() === null}
        tabIndex={props.menu ? -1 : 0}
        class={SWATCH}
        title={ownAccent() ? `${ownAccent()!.label} (the theme's own)` : "The theme's own"}
        aria-label={ownAccent() ? `${ownAccent()!.label}, the theme's own accent` : "The theme's own accent"}
        onClick={() => setAccent(null)}
      >
        {/* Scoped to the theme alone, so it shows the theme's accent, not the chosen one. */}
        <span
          data-theme={currentTheme()}
          data-appearance={themeAttrs(currentTheme())['data-appearance']}
          class="grid size-5 place-items-center rounded-full bg-accent"
        >
          <Show when={accentChoice() === null}>
            <Dot on="var(--accent)" />
          </Show>
        </span>
      </button>
      <For each={currentAccents()}>
        {(a) => (
          <button
            role={role}
            aria-checked={accentChoice() === a.name}
            tabIndex={props.menu ? -1 : 0}
            class={SWATCH}
            title={a.label}
            aria-label={a.label}
            onClick={() => setAccent(a.name)}
          >
            <span class="grid size-5 place-items-center rounded-full" style={{ background: `var(--${a.name})` }}>
              <Show when={accentChoice() === a.name}>
                <Dot on={`var(--${a.name})`} />
              </Show>
            </span>
          </button>
        )}
      </For>
    </div>
  );
}
