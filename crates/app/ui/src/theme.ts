// Theme and accent selection. Themes live in themes/ (see themes/index.css)
// and are registered here. From Magnitide, with Transceiver's own Light, Dark
// and Sepia, and without add-on theme files.

import { createSignal } from 'solid-js';

import { storage } from './storage';

/** A palette accent: its variable name (`--mauve`) and how a theme labels it. */
export type Accent = { name: string; label: string };

/** Catppuccin's accents, in its order. */
const CATPPUCCIN_ACCENTS: Accent[] = [
  'Rosewater',
  'Flamingo',
  'Pink',
  'Mauve',
  'Red',
  'Maroon',
  'Peach',
  'Yellow',
  'Green',
  'Teal',
  'Sky',
  'Sapphire',
  'Blue',
  'Lavender',
].map((label) => ({ name: label.toLowerCase(), label }));

/** Light and Dark's accents: the icon's teal and amber first (each theme tunes them; transceiver.css). */
const TRANSCEIVER_ACCENTS: Accent[] = [
  { name: 'teal', label: 'Teal' },
  { name: 'amber', label: 'Amber' },
  { name: 'blue', label: 'Blue' },
  { name: 'purple', label: 'Purple' },
];
/** Sepia's: its own peach, then the icon's. */
const SEPIA_ACCENTS: Accent[] = [{ name: 'peach', label: 'Peach' }, ...TRANSCEIVER_ACCENTS];

/** Dracula's accents, by its own names (mapped onto the palette's in dracula.css). */
const DRACULA_ACCENTS: Accent[] = [
  { name: 'red', label: 'Red' },
  { name: 'peach', label: 'Orange' },
  { name: 'yellow', label: 'Yellow' },
  { name: 'green', label: 'Green' },
  { name: 'sky', label: 'Cyan' },
  { name: 'mauve', label: 'Purple' },
  { name: 'pink', label: 'Pink' },
];

/** A theme the app can switch to. */
export type Theme = {
  id: string;
  label: string;
  appearance: 'light' | 'dark';
  /** What the accent picker offers. */
  accents: readonly Accent[];
  /** The theme's own accent (what `--accent` is set to), if it's one of `accents`. */
  own?: string;
};

export const BUILTIN_THEMES: readonly Theme[] = [
  { id: 'light', label: 'Light', appearance: 'light', accents: TRANSCEIVER_ACCENTS, own: 'teal' },
  { id: 'dark', label: 'Dark', appearance: 'dark', accents: TRANSCEIVER_ACCENTS, own: 'teal' },
  { id: 'sepia', label: 'Sepia', appearance: 'light', accents: SEPIA_ACCENTS, own: 'peach' },
  { id: 'catppuccin-latte', label: 'Catppuccin Latte', appearance: 'light', accents: CATPPUCCIN_ACCENTS, own: 'mauve' },
  {
    id: 'catppuccin-frappe',
    label: 'Catppuccin Frappé',
    appearance: 'dark',
    accents: CATPPUCCIN_ACCENTS,
    own: 'mauve',
  },
  {
    id: 'catppuccin-macchiato',
    label: 'Catppuccin Macchiato',
    appearance: 'dark',
    accents: CATPPUCCIN_ACCENTS,
    own: 'mauve',
  },
  { id: 'catppuccin-mocha', label: 'Catppuccin Mocha', appearance: 'dark', accents: CATPPUCCIN_ACCENTS, own: 'mauve' },
  { id: 'dracula', label: 'Dracula', appearance: 'dark', accents: DRACULA_ACCENTS, own: 'mauve' },
  { id: 'alucard', label: 'Alucard', appearance: 'light', accents: DRACULA_ACCENTS, own: 'mauve' },
];

const SAFE_NAME = /^[a-z][a-z0-9-]*$/;

/** Every theme. */
export const themes = (): readonly Theme[] => BUILTIN_THEMES;

// ---------- Choosing ----------

/** `system` follows the OS light/dark setting. */
export type ThemeChoice = string;

/** The attributes that scope an element to a theme (for swatches and previews). */
export function themeAttrs(id: string): { 'data-theme': string; 'data-appearance': string } {
  const t = themes().find((x) => x.id === id) ?? BUILTIN_THEMES[0]!;
  return { 'data-theme': t.id, 'data-appearance': t.appearance };
}

const systemDark = window.matchMedia('(prefers-color-scheme: dark)');
const [choice, setChoice] = createSignal<ThemeChoice>(storage.get('theme') ?? 'system');
const [systemIsDark, setSystemIsDark] = createSignal(systemDark.matches);

/** Each theme's chosen accent (a palette name); a theme without one uses its own. */
function loadAccents(): Record<string, string> {
  try {
    const parsed: unknown = JSON.parse(storage.get('accents') ?? '{}');
    return parsed && typeof parsed === 'object' ? (parsed as Record<string, string>) : {};
  } catch {
    return {};
  }
}
const [accents, setAccents] = createSignal<Record<string, string>>(loadAccents());

/** The theme in use: `system` resolved, and an unknown id falls back to it. */
export const currentTheme = (): string => {
  const c = choice();
  if (c !== 'system' && themes().some((t) => t.id === c)) return c;
  return systemIsDark() ? 'dark' : 'light';
};

const current = (): Theme | undefined => themes().find((t) => t.id === currentTheme());

/** The theme in use's own accent, when it's one the picker would offer. */
export const ownAccent = (): Accent | undefined => {
  const t = current();
  return t?.accents.find((a) => a.name === t.own);
};

/** The accents the theme in use offers besides its own (which the picker shows first, as the theme's). */
export const currentAccents = (): readonly Accent[] => {
  const t = current();
  return t?.accents.filter((a) => a.name !== t.own) ?? [];
};

/** The accent chosen for the theme in use, or null for the theme's own. */
export const accentChoice = (): string | null => {
  const a = accents()[currentTheme()];
  return a && currentAccents().some((x) => x.name === a) ? a : null;
};

function apply(): void {
  const root = document.documentElement;
  Object.assign(root.dataset, { theme: currentTheme(), appearance: themeAttrs(currentTheme())['data-appearance'] });
  // The chosen accent, on <html> alone: elements scoped to a theme (swatches) keep their own.
  const a = accentChoice();
  if (a && SAFE_NAME.test(a)) {
    root.dataset.accent = a;
    root.style.setProperty('--accent', `var(--${a})`);
  } else {
    delete root.dataset.accent;
    root.style.removeProperty('--accent');
  }
}

systemDark.addEventListener('change', () => {
  setSystemIsDark(systemDark.matches);
  apply();
});
apply(); // at import time, before the first render

/** What the user chose ('system' or a theme id). */
export const themeChoice = (): ThemeChoice => {
  const c = choice();
  return c === 'system' || themes().some((t) => t.id === c) ? c : 'system';
};

export function setTheme(c: ThemeChoice): void {
  setChoice(c);
  storage.set('theme', c);
  apply();
}

/** Choose the theme in use's accent (a palette name), or null for its own. */
export function setAccent(name: string | null): void {
  const next = { ...accents() };
  if (name) next[currentTheme()] = name;
  else delete next[currentTheme()];
  setAccents(next);
  storage.set('accents', JSON.stringify(next));
  apply();
}
