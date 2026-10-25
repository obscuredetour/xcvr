import X from 'lucide-solid/icons/x';

import { INSPECTOR_WIDTH, inspectorWidth, setInspectorWidth } from '../view';
import { widthFlags } from '../widths';
import { Icon } from './Icon';

/**
 * The right sidebar: the selected torrent's details, files, peers, trackers
 * and options (phase 6). Resizable from its left edge, like Magnitide's
 * details panel.
 */
export function Inspector(props: { onClose(): void }) {
  return (
    <aside
      ref={(el) => widthFlags(el, { wide: 416 })}
      class="group/inspector relative flex-none animate-slide-in border-l border-line bg-panel select-text max-[760px]:absolute max-[760px]:inset-y-0 max-[760px]:right-0 max-[760px]:w-[min(28rem,100%-2rem)]! max-[760px]:max-w-full! max-[760px]:shadow-lift"
      // The table keeps at least 320px beside it.
      style={{ width: `${inspectorWidth()}px`, 'max-width': 'calc(100% - 320px)' }}
      aria-label="Inspector"
    >
      <ResizeHandle />
      <button
        class="absolute top-2.5 right-2.5 z-1 grid size-7 place-items-center rounded-full hover:bg-hover"
        title="Close"
        aria-label="Close inspector"
        onClick={() => props.onClose()}
      >
        <Icon of={X} class="text-muted" />
      </button>
      <div class="grid h-full place-items-center p-6 text-center text-sm text-muted">Select a torrent</div>
    </aside>
  );
}

/**
 * The inspector's left edge: drag to resize, double-click for the default
 * width, or focus it and use ←/→ (Shift for bigger steps). Hidden while the
 * inspector overlays a narrow window.
 */
function ResizeHandle() {
  const step = (e: KeyboardEvent) => (e.shiftKey ? 64 : 16);
  return (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize the inspector"
      aria-valuenow={inspectorWidth()}
      aria-valuemin={INSPECTOR_WIDTH.min}
      aria-valuemax={INSPECTOR_WIDTH.max}
      tabIndex={0}
      title="Drag to resize · double-click to reset"
      class="group/resize absolute inset-y-0 -left-1 z-2 w-2 cursor-col-resize outline-none max-[760px]:hidden"
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        const handle = e.currentTarget;
        handle.setPointerCapture(e.pointerId);
        handle.dataset.dragging = '';
        const startX = e.clientX;
        const startW = inspectorWidth();
        // Dragging left widens the inspector.
        const move = (ev: PointerEvent) => setInspectorWidth(startW + startX - ev.clientX, false);
        const end = () => {
          delete handle.dataset.dragging;
          setInspectorWidth(inspectorWidth());
          handle.removeEventListener('pointermove', move);
          handle.removeEventListener('pointerup', end);
          handle.removeEventListener('pointercancel', end);
        };
        handle.addEventListener('pointermove', move);
        handle.addEventListener('pointerup', end);
        handle.addEventListener('pointercancel', end);
      }}
      onDblClick={() => setInspectorWidth(INSPECTOR_WIDTH.initial)}
      onKeyDown={(e) => {
        if (e.key === 'ArrowLeft') setInspectorWidth(inspectorWidth() + step(e));
        else if (e.key === 'ArrowRight') setInspectorWidth(inspectorWidth() - step(e));
        else if (e.key === 'Home') setInspectorWidth(INSPECTOR_WIDTH.min);
        else if (e.key === 'End') setInspectorWidth(INSPECTOR_WIDTH.max);
        else return;
        e.preventDefault();
        e.stopPropagation();
      }}
    >
      {/* The visible line: on hover, while dragging, and when focused from the keyboard. */}
      <div class="mx-auto h-full w-0.5 bg-accent opacity-0 transition-opacity group-hover/resize:opacity-60 group-focus-visible/resize:opacity-100 group-data-dragging/resize:opacity-100" />
    </div>
  );
}
