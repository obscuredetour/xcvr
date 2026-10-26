import { createSignal, For, Match, onMount, Switch } from 'solid-js';
import type { Component } from 'solid-js';
import type { LucideProps } from 'lucide-solid';
import AppWindow from 'lucide-solid/icons/app-window';
import Info from 'lucide-solid/icons/info';
import FolderSync from 'lucide-solid/icons/folder-sync';
import Keyboard from 'lucide-solid/icons/keyboard';
import Palette from 'lucide-solid/icons/palette';
import Server from 'lucide-solid/icons/server';
import Settings2 from 'lucide-solid/icons/settings-2';
import SlidersHorizontal from 'lucide-solid/icons/sliders-horizontal';
import X from 'lucide-solid/icons/x';

import { backdropDismiss } from '../../backdrop';
import { Icon } from '../Icon';
import { About } from './About';
import { Appearance } from './Appearance';
import { ConnectionSettings } from './Connection';
import { GeneralSettings } from './General';
import { PathSettings } from './Paths';
import { ShortcutSettings } from './Shortcuts';
import { ThemeSettings } from './Themes';
import { TransmissionSettings } from './Transmission';

export type SettingsSection =
  'general' | 'appearance' | 'themes' | 'connection' | 'transmission' | 'paths' | 'shortcuts' | 'about';

type Entry = { id: SettingsSection; label: string; icon: Component<LucideProps> };

/** The sections, in groups: the app itself and how it looks, what it talks to, the rest. */
const SECTIONS: Entry[][] = [
  [
    { id: 'general', label: 'General', icon: Settings2 },
    { id: 'appearance', label: 'Appearance', icon: AppWindow },
    { id: 'themes', label: 'Themes', icon: Palette },
  ],
  [
    { id: 'connection', label: 'Connection', icon: Server },
    { id: 'transmission', label: 'Transmission', icon: SlidersHorizontal },
    { id: 'paths', label: 'Paths', icon: FolderSync },
  ],
  [
    { id: 'shortcuts', label: 'Keyboard Shortcuts', icon: Keyboard },
    { id: 'about', label: 'About', icon: Info },
  ],
];

/**
 * The settings window (⌘,): a large dialog over the torrents, so most pages
 * fit without scrolling. Mount it to open; `onClose` unmounts it.
 */
export function SettingsDialog(props: { onClose(): void; section?: SettingsSection }) {
  let dialog!: HTMLDialogElement;
  let page!: HTMLDivElement;
  const [section, setSection] = createSignal<SettingsSection>(props.section ?? 'general');

  onMount(() => dialog.showModal());

  return (
    <dialog
      ref={dialog}
      aria-label="Settings"
      class="m-auto h-[min(720px,calc(100vh-48px))] w-[min(1000px,calc(100vw-48px))] animate-menu-in overflow-hidden rounded-dialog border border-line bg-popover p-0 text-fg shadow-lift"
      onCancel={(e) => {
        // Esc: let Solid unmount the dialog instead of the browser closing it.
        e.preventDefault();
        props.onClose();
      }}
      // Settings apply as they're made, so clicking outside always closes it.
      {...backdropDismiss(() => dialog, { close: () => props.onClose() })}
    >
      <div class="flex h-full">
        <nav
          class="flex w-52 flex-none flex-col overflow-y-auto border-r border-line bg-bg/50 p-3"
          aria-label="Settings sections"
        >
          <h2 class="px-2.5 pt-1 pb-3 font-serif text-lg font-semibold">Settings</h2>
          <For each={SECTIONS}>
            {(group, i) => (
              <div class="flex flex-col gap-0.5" classList={{ 'mt-2 border-t border-line pt-2': i() > 0 }}>
                <For each={group}>
                  {(s) => (
                    <button
                      class="group flex items-center gap-2.5 rounded-control px-2.5 py-1.5 text-left hover:bg-hover aria-[current=page]:bg-highlight aria-[current=page]:text-accent"
                      aria-current={section() === s.id ? 'page' : undefined}
                      onClick={() => {
                        setSection(s.id);
                        page.scrollTop = 0;
                      }}
                    >
                      <Icon of={s.icon} class="text-muted group-aria-[current=page]:text-accent" />
                      {s.label}
                    </button>
                  )}
                </For>
              </div>
            )}
          </For>
        </nav>

        {/* Focused on opening (autofocus), so no section button shows a focus ring until Tab. */}
        <div
          ref={page}
          tabIndex={-1}
          autofocus
          class="@container relative min-w-0 flex-1 overflow-y-auto px-8 pt-7 pb-10 outline-none select-text"
        >
          <button
            class="absolute top-4 right-4 grid size-7 place-items-center rounded-full hover:bg-hover"
            title="Close (Esc)"
            aria-label="Close settings"
            onClick={() => props.onClose()}
          >
            <Icon of={X} class="text-muted" />
          </button>
          <Switch>
            <Match when={section() === 'general'}>
              <GeneralSettings />
            </Match>
            <Match when={section() === 'appearance'}>
              <Appearance />
            </Match>
            <Match when={section() === 'themes'}>
              <ThemeSettings />
            </Match>
            <Match when={section() === 'connection'}>
              <ConnectionSettings />
            </Match>
            <Match when={section() === 'transmission'}>
              <TransmissionSettings />
            </Match>
            <Match when={section() === 'paths'}>
              <PathSettings />
            </Match>
            <Match when={section() === 'shortcuts'}>
              <ShortcutSettings />
            </Match>
            <Match when={section() === 'about'}>
              <About />
            </Match>
          </Switch>
        </div>
      </div>
    </dialog>
  );
}
