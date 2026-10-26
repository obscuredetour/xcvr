import { createSignal, For, Show } from 'solid-js';
import X from 'lucide-solid/icons/x';

import { allLabels, editLabels, labelCounts, splitLabels } from '../labels';
import { Icon } from './Icon';

let nextId = 0;

/**
 * The labels of one torrent or several: a chip per label (× takes it off them
 * all; one only some have shows how many, and a click puts it on the rest), and
 * a field to add more, suggesting labels already in use.
 */
export function LabelEditor(props: { hashes: string[] }) {
  const listId = `label-suggestions-${nextId++}`;
  const [text, setText] = createSignal('');
  const counts = () => [...labelCounts(props.hashes)].sort(([a], [b]) => a.localeCompare(b));
  const total = () => props.hashes.length;
  const suggestions = () => allLabels().filter(([l]) => (labelCounts(props.hashes).get(l) ?? 0) < total());

  const add = () => {
    const labels = splitLabels(text());
    setText('');
    void editLabels(props.hashes, labels);
  };

  return (
    <div class="flex flex-wrap items-center gap-1">
      <For each={counts()}>
        {([label, n]) => (
          <span
            class="inline-flex h-6 items-center gap-1 rounded-chip bg-element pr-1 pl-2 text-[11px] font-medium"
            classList={{ 'border border-dashed border-line bg-transparent text-muted': n < total() }}
          >
            <Show when={n < total()} fallback={label}>
              <button
                class="cursor-pointer hover:text-fg"
                title={`On ${n} of ${total()}: click to put it on them all`}
                onClick={() => void editLabels(props.hashes, [label])}
              >
                {label}{' '}
                <span class="tabular-nums">
                  {n}/{total()}
                </span>
              </button>
            </Show>
            <button
              class="grid size-4 place-items-center rounded-full text-muted hover:bg-hover hover:text-fg"
              aria-label={`Remove “${label}”`}
              title={`Remove “${label}”`}
              onClick={() => void editLabels(props.hashes, [], [label])}
            >
              <Icon of={X} size={12} />
            </button>
          </span>
        )}
      </For>
      <input
        class="h-6 w-28 min-w-0 flex-1 rounded-chip border border-transparent bg-transparent px-2 text-[11px] outline-none placeholder:text-faint hover:border-line focus:border-focus"
        placeholder={counts().length ? 'Add…' : 'Add a label…'}
        aria-label="Add a label"
        spellcheck={false}
        autocomplete="off"
        list={listId}
        value={text()}
        onInput={(e) => setText(e.currentTarget.value)}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.key === 'Enter' && text().trim()) {
            e.preventDefault();
            add();
          } else if (e.key === 'Escape') {
            setText('');
            e.currentTarget.blur();
          }
        }}
        onBlur={() => text().trim() && add()}
      />
      <datalist id={listId}>
        <For each={suggestions()}>{([l]) => <option value={l} />}</For>
      </datalist>
    </div>
  );
}
