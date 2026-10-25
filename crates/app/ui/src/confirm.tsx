// A modal "Are you sure?" for destructive actions. `confirm(...)` resolves
// to true when the user confirms; <ConfirmHost/> (mounted once in App) shows it.

import { createSignal, type JSX, onCleanup, onMount, Show } from 'solid-js';

import { dangerButton, primaryButton, secondaryButton } from './buttons';

type Request = {
  title: string;
  /** Rendered inside the dialog (so it can hold its own controls). */
  message: () => JSX.Element;
  /** The confirm button's label, e.g. "Remove 3 Books"; a function when it follows choices in the message. */
  confirm: string | (() => string);
  /** Red confirm button. */
  danger?: boolean;
  /** The cancel button's label (default "Cancel"). */
  cancel?: string;
  /** Seconds until it cancels by itself, counted down on the cancel button
      (for changes that could leave the app hard to use, like display settings). */
  timeout?: number;
  resolve(ok: boolean): void;
};

const [request, setRequest] = createSignal<Request | null>(null);

export function confirm(options: Omit<Request, 'resolve'>): Promise<boolean> {
  request()?.resolve(false);
  return new Promise((resolve) => setRequest({ ...options, resolve }));
}

export function ConfirmHost() {
  return <Show when={request()}>{(r) => <ConfirmDialog request={r()} />}</Show>;
}

function ConfirmDialog(props: { request: Request }) {
  let dialog!: HTMLDialogElement;
  let confirmButton!: HTMLButtonElement;
  const done = (ok: boolean) => {
    props.request.resolve(ok);
    setRequest(null);
  };
  const [left, setLeft] = createSignal(props.request.timeout);
  if (props.request.timeout) {
    const tick = setInterval(() => {
      const n = (left() ?? 0) - 1;
      setLeft(n);
      if (n <= 0) done(false);
    }, 1000);
    onCleanup(() => clearInterval(tick));
  }
  onMount(() => {
    dialog.showModal();
    // A field in the message (marked autofocus) takes focus; else the confirm button.
    (dialog.querySelector<HTMLElement>('[autofocus]') ?? confirmButton).focus();
  });
  return (
    <dialog
      ref={dialog}
      aria-label={props.request.title}
      class="m-auto w-[min(440px,calc(100vw-48px))] animate-menu-in rounded-dialog border border-line bg-popover p-0 text-fg shadow-lift"
      onCancel={(e) => {
        e.preventDefault();
        done(false);
      }}
    >
      <div class="px-6 pt-5 pb-4">
        <h2 class="mb-2 text-base font-semibold">{props.request.title}</h2>
        <div class="text-sm text-muted select-text">{props.request.message()}</div>
      </div>
      <div class="flex justify-end gap-2 border-t border-line px-6 py-3">
        <button class={`${secondaryButton} tabular-nums`} onClick={() => done(false)}>
          {props.request.cancel ?? 'Cancel'}
          <Show when={left()}>{(n) => <> ({n()})</>}</Show>
        </button>
        <button
          ref={confirmButton}
          data-confirm
          class={props.request.danger ? dangerButton : primaryButton}
          onClick={() => done(true)}
        >
          {typeof props.request.confirm === 'function' ? props.request.confirm() : props.request.confirm}
        </button>
      </div>
    </dialog>
  );
}
