// Shared button styles, so buttons side by side are the same height. Each
// has a fixed height and a border (transparent where it isn't seen): a border
// on only some of them made those 2px taller.

const base =
  'inline-flex cursor-pointer items-center justify-center gap-1.5 rounded-button border whitespace-nowrap disabled:cursor-default disabled:opacity-50';

/** The main action of a dialog: Save, Send, Import… */
export const primaryButton = `${base} h-8 border-transparent bg-accent px-4 text-sm font-semibold text-on-accent hover:brightness-110 disabled:hover:brightness-100`;

/** Other dialog actions: Cancel, Previous, Search… */
export const secondaryButton = `${base} h-8 border-line bg-element px-4 text-sm font-medium hover:bg-hover`;

/** A destructive main action: Move to Trash, Remove… */
export const dangerButton = `${base} h-8 border-transparent bg-danger px-4 text-sm font-semibold text-on-status hover:brightness-110`;

/** Smaller actions inside forms and settings (as tall as their h-7 inputs). */
export const smallButton = `${base} h-7 border-line bg-element px-2.5 text-xs font-medium hover:bg-hover`;
