import { For, type ParentProps } from 'solid-js';

import { setTheme, themeAttrs, themeChoice, themes } from '../../theme';
import { AccentPicker } from '../AccentPicker';
import { Group, Heading, Row } from './parts';

/** Miniature of a theme: page, a card, an accent. Scoped via data-theme. */
function ThemePreview(props: { theme: string }) {
  return (
    <div {...themeAttrs(props.theme)} class="flex h-full w-full flex-col gap-1.5 bg-bg p-2">
      <div class="h-1.5 w-1/2 rounded-full bg-fg/70" />
      <div class="flex flex-1 gap-1.5">
        <div class="flex-1 rounded bg-panel shadow-card" />
        <div class="flex-1 rounded bg-element" />
      </div>
      <div class="h-1.5 w-1/3 rounded-full bg-accent" />
    </div>
  );
}

function ThemeCard(props: ParentProps<{ selected: boolean; label: string; onSelect(): void }>) {
  return (
    <button
      class="group flex cursor-pointer flex-col gap-1.5 text-left"
      role="radio"
      aria-checked={props.selected}
      onClick={() => props.onSelect()}
    >
      <div class="h-[72px] w-full overflow-hidden rounded-card border border-line group-aria-checked:outline-2 group-aria-checked:outline-offset-2 group-aria-checked:outline-accent">
        {props.children}
      </div>
      <span class="text-xs group-aria-checked:font-semibold">{props.label}</span>
    </button>
  );
}

export function ThemeSettings() {
  return (
    <>
      <Heading title="Themes">The app’s colours. The toolbar’s theme menu switches them too.</Heading>

      <section class="mb-7">
        <div
          class="mb-3 grid grid-cols-[repeat(auto-fill,minmax(120px,1fr))] gap-4"
          role="radiogroup"
          aria-label="Theme"
        >
          <ThemeCard selected={themeChoice() === 'system'} label="Match System" onSelect={() => setTheme('system')}>
            <div class="flex h-full">
              <div class="w-1/2 overflow-hidden">
                <ThemePreview theme="light" />
              </div>
              <div class="w-1/2 overflow-hidden">
                <ThemePreview theme="dark" />
              </div>
            </div>
          </ThemeCard>
          <For each={themes()}>
            {(t) => (
              <ThemeCard selected={themeChoice() === t.id} label={t.label} onSelect={() => setTheme(t.id)}>
                <ThemePreview theme={t.id} />
              </ThemeCard>
            )}
          </For>
        </div>
      </section>

      <Group>
        <Row label="Accent" description="Buttons, selection and highlights. Remembered for each theme.">
          <div class="max-w-[300px]">
            <AccentPicker />
          </div>
        </Row>
      </Group>
    </>
  );
}
