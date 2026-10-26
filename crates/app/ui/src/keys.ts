// Keyboard shortcuts as people see them: macOS symbols (⌘E, ⌥←) there,
// spelled out elsewhere (Ctrl+E, Alt+←). The handlers accept ⌘ or Ctrl
// (`e.metaKey || e.ctrlKey`), so "mod" means whichever the platform uses.
// SHORTCUTS lists them all, for Settings › Keyboard Shortcuts.

export const isMac = /Mac/.test(navigator.userAgent);

const MAC: Record<string, string> = {
  mod: '⌘',
  alt: '⌥',
  shift: '⇧',
  ctrl: '⌃',
  Backspace: '⌫',
  Enter: '↩',
  Escape: 'Esc',
};
const OTHER: Record<string, string> = { mod: 'Ctrl', alt: 'Alt', shift: 'Shift', ctrl: 'Ctrl', Escape: 'Esc' };
const BOTH: Record<string, string> = { ArrowLeft: '←', ArrowRight: '→', ArrowUp: '↑', ArrowDown: '↓', '-': '−' };

/** "mod+E" → "⌘E" on macOS, "Ctrl+E" elsewhere. Parts are joined with "+"; "mod++" is ⌘+ / Ctrl++. */
export function keys(combo: string): string {
  const split = combo.endsWith('++') ? [...combo.slice(0, -2).split('+'), '+'] : combo.split('+');
  // Modifiers in the platform's order: ⌃⌥⇧⌘ on macOS, Ctrl+Alt+Shift elsewhere.
  const order = isMac ? ['ctrl', 'alt', 'shift', 'mod'] : ['mod', 'ctrl', 'alt', 'shift'];
  const rank = (p: string) => (order.includes(p) ? order.indexOf(p) : order.length);
  const parts = split.map((p, i) => [p, i] as const).sort(([a, i], [b, j]) => rank(a) - rank(b) || i - j);
  const names = parts.map(([p]) => (isMac ? MAC : OTHER)[p] ?? BOTH[p] ?? p);
  // "⌘-click", as macOS writes it ("Ctrl+click" elsewhere).
  if (isMac && names.at(-1) === 'click') return `${names.slice(0, -1).join('')}-click`;
  return isMac ? names.join('') : names.join('+');
}

/** The key that removes the selected torrents. */
export const deleteKeys = isMac ? 'mod+Backspace' : 'Delete';

type Shortcut = { keys: string[]; action: string };

/** Every shortcut, grouped, for Settings. Click modifiers are written "mod+click". Add each one as it's built. */
export const SHORTCUTS: { group: string; items: Shortcut[] }[] = [
  {
    group: 'General',
    items: [
      { keys: ['mod+,'], action: 'Settings' },
      { keys: ['mod+alt+X'], action: 'Show Transceiver (from any app)' },
      { keys: ['mod+F', '/'], action: 'Search' },
      { keys: ['mod++', 'mod+-'], action: 'Larger / smaller interface' },
      { keys: ['mod+0'], action: 'Actual size' },
    ],
  },
  {
    group: 'Panels',
    items: [
      { keys: ['mod+alt+S'], action: 'Show / hide the sidebar' },
      { keys: ['mod+I'], action: 'Show / hide the inspector' },
      { keys: ['mod+shift+T'], action: 'Slower speeds (the turtle) on / off' },
      { keys: ['ArrowLeft', 'ArrowRight'], action: 'Resize the inspector (its edge focused)' },
    ],
  },
  {
    group: 'Adding',
    items: [
      { keys: ['mod+N'], action: 'Add torrents' },
      { keys: ['mod+O'], action: 'Choose .torrent files' },
      { keys: ['mod+V'], action: 'Add a copied magnet link' },
      { keys: ['mod+Enter'], action: 'Add (in the add dialog)' },
    ],
  },
  {
    group: 'Torrents',
    items: [
      { keys: ['ArrowUp', 'ArrowDown'], action: 'Move the selection' },
      { keys: ['shift+ArrowUp', 'shift+ArrowDown'], action: 'Extend the selection' },
      { keys: ['mod+click'], action: 'Add to or remove from the selection' },
      { keys: ['shift+click'], action: 'Select a range' },
      { keys: ['mod+A'], action: 'Select all' },
      { keys: ['Escape'], action: 'Back to one torrent, then none' },
      { keys: ['Enter'], action: 'Show in the inspector' },
      { keys: ['Space'], action: 'Pause or resume' },
      { keys: [deleteKeys], action: 'Remove' },
      { keys: [`shift+${deleteKeys}`], action: 'Remove and delete files' },
      { keys: ['mod+ArrowDown'], action: 'Open (through Settings › Paths)' },
      { keys: ['mod+R'], action: isMac ? 'Show in Finder' : 'Show in the file manager' },
      { keys: ['F2'], action: 'Rename' },
      { keys: ['mod+L'], action: 'Set location' },
      { keys: ['shift+F10'], action: 'Torrent menu (also the menu key)' },
      { keys: ['alt+ArrowLeft', 'alt+ArrowRight'], action: 'Move the focused column' },
    ],
  },
];
