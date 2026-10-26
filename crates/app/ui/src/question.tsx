// A short question: choices as buttons, or a line of text (a label's name).
// `askUser(...)` resolves to the choice's index, the text, or null when
// dismissed; <QuestionHost/> (mounted once in App) shows it.

import { createSignal, onMount, Show, For } from 'solid-js';

import { primaryButton, secondaryButton } from './buttons';

type Request = {
  /** Who's asking. */
  title: string;
  question: string;
  choices: string[];
  /** Set for a text question. */
  placeholder: string | null;
  /** A text question's starting text (selected, to type over). */
  initial?: string;
  resolve(answer: { choice: number } | { text: string } | null): void;
};

const [request, setRequest] = createSignal<Request | null>(null);

export function askUser(options: Omit<Request, 'resolve'>): Promise<{ choice: number } | { text: string } | null> {
  request()?.resolve(null);
  return new Promise((resolve) => setRequest({ ...options, resolve }));
}

export function QuestionHost() {
  return <Show when={request()}>{(r) => <QuestionDialog request={r()} />}</Show>;
}

function QuestionDialog(props: { request: Request }) {
  let dialog!: HTMLDialogElement;
  let first!: HTMLElement;
  const [text, setText] = createSignal(props.request.initial ?? '');
  const done = (answer: { choice: number } | { text: string } | null) => {
    props.request.resolve(answer);
    setRequest(null);
  };
  onMount(() => {
    dialog.showModal();
    first.focus();
    if (first instanceof HTMLInputElement) first.select();
  });
  return (
    <dialog
      ref={dialog}
      aria-label={props.request.title}
      class="m-auto w-[min(440px,calc(100vw-48px))] animate-menu-in rounded-dialog border border-line bg-popover p-0 text-fg shadow-lift"
      onCancel={(e) => {
        e.preventDefault();
        done(null);
      }}
    >
      <form
        class="px-6 pt-5 pb-4"
        onSubmit={(e) => {
          e.preventDefault();
          if (props.request.placeholder !== null) done({ text: text() });
        }}
      >
        <h2 class="mb-2 text-base font-semibold">{props.request.title}</h2>
        <p class="text-sm text-muted select-text">{props.request.question}</p>
        <Show when={props.request.placeholder !== null}>
          <input
            ref={(el) => (first = el)}
            class="mt-3 h-8 w-full rounded-control border border-line bg-field px-2.5 text-sm outline-none focus:border-focus focus:ring-3 focus:ring-focus/20"
            aria-label={props.request.question}
            placeholder={props.request.placeholder ?? ''}
            value={text()}
            onInput={(e) => setText(e.currentTarget.value)}
          />
        </Show>
      </form>
      <div class="flex flex-wrap justify-end gap-2 border-t border-line px-6 py-3">
        <button type="button" class={secondaryButton} onClick={() => done(null)}>
          Cancel
        </button>
        <Show
          when={props.request.placeholder === null}
          fallback={
            <button type="button" class={primaryButton} onClick={() => done({ text: text() })}>
              OK
            </button>
          }
        >
          <For each={props.request.choices}>
            {(choice, i) => (
              <button
                type="button"
                ref={(el) => {
                  if (i() === 0) first = el;
                }}
                class={i() === 0 ? primaryButton : secondaryButton}
                onClick={() => done({ choice: i() })}
              >
                {choice}
              </button>
            )}
          </For>
        </Show>
      </div>
    </dialog>
  );
}
