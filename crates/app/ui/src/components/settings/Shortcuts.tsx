import { For } from 'solid-js';

import { keys, SHORTCUTS } from '../../keys';
import { Group, Heading } from './parts';

export function ShortcutSettings() {
  return (
    <>
      <Heading title="Keyboard Shortcuts" />
      <div class="grid items-start gap-x-6 @[720px]:grid-cols-2">
        <For each={SHORTCUTS}>
          {(g) => (
            <Group title={g.group}>
              <For each={g.items}>
                {(s) => (
                  <div class="flex items-center justify-between gap-6 border-b border-line py-2 last:border-b-0">
                    <span>{s.action}</span>
                    <span class="flex flex-none flex-wrap justify-end gap-1">
                      <For each={s.keys}>
                        {(k) => (
                          <kbd class="min-w-6 rounded-inset border border-line bg-element px-1.5 py-0.5 text-center font-sans text-xs text-fg shadow-[0_1px_0_var(--color-line)]">
                            {keys(k)}
                          </kbd>
                        )}
                      </For>
                    </span>
                  </div>
                )}
              </For>
            </Group>
          )}
        </For>
      </div>
    </>
  );
}
