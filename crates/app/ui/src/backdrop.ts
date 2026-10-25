// Clicks on a modal <dialog>'s backdrop: close it when nothing would be lost,
// else nudge it (macOS's "can't do that" shake), so the click visibly did
// something and Save / Cancel stay the ways out.

/** Whether a pointer event is on `dialog`'s backdrop (the dialog element itself, outside its box). */
function onBackdrop(e: MouseEvent, dialog: HTMLDialogElement): boolean {
  if (e.target !== dialog) return false;
  const r = dialog.getBoundingClientRect();
  return e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom;
}

function nudge(el: HTMLElement): void {
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  el.animate(
    [0, -8, 7, -5, 3, 0].map((x) => ({ translate: `${x}px 0` })),
    { duration: 360, easing: 'ease-out' },
  );
}

/**
 * Handlers to spread on the <dialog>. Only a press and release both on the
 * backdrop count: a text selection dragged out of a field, which also ends
 * with a click on the dialog element, doesn't.
 */
export function backdropDismiss(dialog: () => HTMLDialogElement, opts: { dirty?: () => boolean; close(): void }) {
  let pressed = false;
  return {
    onPointerDown: (e: PointerEvent) => {
      pressed = onBackdrop(e, dialog());
    },
    onClick: (e: MouseEvent) => {
      if (!pressed || !onBackdrop(e, dialog())) return;
      pressed = false;
      if (opts.dirty?.()) nudge(dialog());
      else opts.close();
    },
  };
}
