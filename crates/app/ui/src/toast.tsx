import { createSignal, Show } from 'solid-js';

type ToastState = { message: string; error: boolean };

const [current, setCurrent] = createSignal<ToastState | null>(null);
/** Something sits at the bottom of the window (the selection bar): show toasts above it. */
const [lifted, setLifted] = createSignal(false);
export { setLifted as liftToasts };
let timer: ReturnType<typeof setTimeout> | undefined;

/** Show a short message. `duration` (ms) defaults to longer for errors; Infinity keeps it up until replaced. */
export function toast(message: string, { error = false, duration }: { error?: boolean; duration?: number } = {}): void {
  clearTimeout(timer);
  setCurrent({ message, error });
  const ms = duration ?? (error ? 6000 : 1800);
  if (Number.isFinite(ms)) timer = setTimeout(() => setCurrent(null), ms);
}

export function Toast() {
  return (
    <Show when={current()}>
      {(t) => (
        <div
          class="fixed left-1/2 z-20 max-w-[min(560px,calc(100%-32px))] -translate-x-1/2 animate-toast-in rounded-menu px-4 py-[9px] shadow-lift select-text"
          classList={{
            'bottom-10': !lifted(),
            'bottom-24': lifted(),
            'bg-inverse/95 text-on-inverse': !t().error,
            'bg-danger text-on-status': t().error,
          }}
          role="status"
          aria-live="polite"
        >
          {t().message}
        </div>
      )}
    </Show>
  );
}
