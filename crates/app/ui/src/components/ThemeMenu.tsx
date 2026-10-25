import { For } from 'solid-js';
import Contrast from 'lucide-solid/icons/contrast';

import { setTheme, themeAttrs, themeChoice, themes } from '../theme';
import { AccentPicker } from './AccentPicker';
import { Menu, MenuItem, MenuLabel, MenuSeparator } from './Menu';
import { Icon } from './Icon';

/**
 * A theme's colours, rendered by scoping `data-theme` to the swatch itself;
 * themes.css selectors match any element, not just <html>.
 */
function Swatch(props: { theme: string }) {
  return (
    <span
      {...themeAttrs(props.theme)}
      class="relative size-5 flex-none overflow-hidden rounded-full border border-line bg-bg"
      aria-hidden="true"
    >
      <span class="absolute inset-y-0 right-0 w-1/2 bg-element" />
      <span class="absolute right-[3px] bottom-[3px] size-2 rounded-full bg-accent" />
    </span>
  );
}

export function ThemeMenu() {
  return (
    <Menu
      label="Theme"
      iconOnly
      align="end"
      buttonClass="grid size-8 place-items-center rounded-button hover:bg-hover aria-expanded:bg-hover"
      button={<Icon of={Contrast} size={16} class="text-muted" />}
    >
      <MenuLabel>Theme</MenuLabel>
      <MenuItem checked={themeChoice() === 'system'} onSelect={() => setTheme('system')}>
        <span class="relative size-5 flex-none overflow-hidden rounded-full border border-line" aria-hidden="true">
          <span {...themeAttrs('light')} class="absolute inset-y-0 left-0 w-1/2 bg-bg" />
          <span {...themeAttrs('dark')} class="absolute inset-y-0 right-0 w-1/2 bg-bg" />
        </span>
        Match System
      </MenuItem>
      <For each={themes()}>
        {(t) => (
          <MenuItem checked={themeChoice() === t.id} onSelect={() => setTheme(t.id)}>
            <Swatch theme={t.id} />
            {t.label}
          </MenuItem>
        )}
      </For>
      <MenuSeparator />
      <MenuLabel>Accent</MenuLabel>
      <AccentPicker menu />
    </Menu>
  );
}
