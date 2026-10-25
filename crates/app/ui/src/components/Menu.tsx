import {
  createContext,
  createEffect,
  createSignal,
  type JSX,
  onCleanup,
  onMount,
  type ParentProps,
  Show,
  untrack,
  useContext,
} from 'solid-js';
import { Portal } from 'solid-js/web';
import Check from 'lucide-solid/icons/check';
import ChevronRight from 'lucide-solid/icons/chevron-right';
import { Icon } from './Icon';

const MenuContext = createContext<{ close(): void }>();

const ITEMS = '[role^="menuitem"]:not([disabled])';
const PANEL = 'z-30 min-w-56 animate-menu-in rounded-menu border border-line bg-popover p-1.5 text-fg shadow-lift';
const ITEM_BASE =
  'flex w-full cursor-default items-center gap-2.5 rounded-control px-2.5 py-1.5 text-left outline-none disabled:opacity-45 disabled:hover:bg-transparent';
const ITEM = `${ITEM_BASE} hover:bg-highlight focus:bg-highlight`;
const DANGER_ITEM = `${ITEM_BASE} text-danger hover:bg-danger/15 focus:bg-danger/15`;

type PanelProps = {
  label: string;
  class?: string;
  style?: JSX.CSSProperties;
  ref?: (el: HTMLDivElement) => void;
  /** Closes the whole menu (an item was chosen). */
  close(): void;
  /** Esc (and, in a submenu, ←). */
  onEscape(): void;
  /** Tab away. */
  onTab?(): void;
  /** Focus an item when shown: the checked one, else the first. */
  autofocus?: boolean;
  children: JSX.Element;
};

/**
 * The panel of a menu: arrow keys move between its own items (not a
 * submenu's), and keys pressed inside don't reach the app's global shortcuts.
 */
function MenuPanel(props: PanelProps) {
  let panel!: HTMLDivElement;
  const items = () =>
    [...panel.querySelectorAll<HTMLElement>(ITEMS)].filter((el) => el.closest('[role="menu"]') === panel);

  onMount(() => {
    if (props.autofocus === false) return;
    queueMicrotask(() => {
      const all = items();
      (all.find((el) => el.getAttribute('aria-checked') === 'true') ?? all[0])?.focus();
    });
  });

  const onKeyDown = (e: KeyboardEvent) => {
    e.stopPropagation();
    const all = items();
    const i = all.indexOf(document.activeElement as HTMLElement);
    const focusAt = (n: number) => all[(n + all.length) % all.length]?.focus();
    switch (e.key) {
      case 'Escape':
        e.preventDefault();
        props.onEscape();
        break;
      case 'Tab':
        props.onTab?.();
        break;
      case 'ArrowDown':
        e.preventDefault();
        focusAt(i + 1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        focusAt(i < 0 ? -1 : i - 1);
        break;
      case 'Home':
        e.preventDefault();
        focusAt(0);
        break;
      case 'End':
        e.preventDefault();
        focusAt(-1);
        break;
    }
  };

  return (
    <MenuContext.Provider value={{ close: props.close }}>
      <div
        ref={(el) => {
          panel = el;
          props.ref?.(el);
        }}
        role="menu"
        aria-label={props.label}
        class={`${PANEL} ${props.class ?? ''}`}
        style={props.style}
        // Native (not delegated) so stopPropagation keeps keys from the app's
        // document-level shortcuts.
        on:keydown={onKeyDown}
      >
        {props.children}
      </div>
    </MenuContext.Provider>
  );
}

type MenuProps = {
  /** Accessible name for the menu (and the button, when `iconOnly`). */
  label: string;
  /** The button shows only an icon, so it needs `label` as its name and tooltip. */
  iconOnly?: boolean;
  /** Tooltip for a button with visible text. */
  title?: string;
  /** Classes for the wrapper, e.g. grid placement. */
  class?: string;
  /** Contents of the trigger button. */
  button: JSX.Element;
  buttonClass?: string;
  /** Which edge of the button the menu lines up with. */
  align?: 'start' | 'end';
  /** Extra classes for the panel, e.g. a width. */
  panelClass?: string;
  /** Called when it opens. */
  onOpen?(): void;
  children: JSX.Element;
};

/** A button with a dropdown menu. Esc or a click outside closes it. */
export function Menu(props: MenuProps) {
  const [open, setOpen] = createSignal(false);
  let root!: HTMLDivElement;
  let button!: HTMLButtonElement;

  const close = (refocus = true) => {
    setOpen(false);
    if (refocus) button.focus();
  };

  createEffect(() => {
    if (!open()) return;
    untrack(() => props.onOpen?.());
    const onPointerDown = (e: PointerEvent) => {
      if (!root.contains(e.target as Node)) close(false);
    };
    document.addEventListener('pointerdown', onPointerDown, true);
    onCleanup(() => document.removeEventListener('pointerdown', onPointerDown, true));
  });

  return (
    <div ref={root} class={`relative ${props.class ?? ''}`}>
      <button
        ref={button}
        class={props.buttonClass}
        aria-haspopup="menu"
        aria-expanded={open()}
        aria-label={props.iconOnly ? props.label : undefined}
        title={props.iconOnly ? props.label : props.title}
        onClick={() => setOpen(!open())}
        onKeyDown={(e) => {
          if (e.key === 'ArrowDown' && !open()) {
            e.preventDefault();
            setOpen(true);
          }
        }}
      >
        {props.button}
      </button>
      <Show when={open()}>
        <MenuPanel
          label={props.label}
          class={`absolute top-full mt-1.5 ${props.align === 'end' ? 'right-0' : 'left-0'} ${props.panelClass ?? ''}`}
          close={() => close()}
          onEscape={() => close()}
          onTab={() => close(false)}
        >
          {props.children}
        </MenuPanel>
      </Show>
    </div>
  );
}

type ContextMenuProps = {
  label: string;
  /** Where it was asked for, in viewport pixels; it moves to stay on screen. */
  x: number;
  y: number;
  onClose(): void;
  children: JSX.Element;
};

/** A menu at a point, e.g. on right-click. A click outside, Esc, scrolling or resizing closes it. */
export function ContextMenu(props: ContextMenuProps) {
  let panel!: HTMLDivElement;
  const previous = document.activeElement as HTMLElement | null;
  const [pos, setPos] = createSignal({ x: props.x, y: props.y });
  const close = (refocus = true) => {
    props.onClose();
    if (refocus) previous?.focus();
  };

  onMount(() => {
    const r = panel.getBoundingClientRect();
    const margin = 8;
    setPos({
      x: Math.max(margin, Math.min(props.x, innerWidth - r.width - margin)),
      y: Math.max(margin, props.y + r.height > innerHeight - margin ? props.y - r.height : props.y),
    });
    const onPointerDown = (e: PointerEvent) => {
      if (!panel.contains(e.target as Node)) close(false);
    };
    const dismiss = () => close(false);
    document.addEventListener('pointerdown', onPointerDown, true);
    addEventListener('resize', dismiss);
    addEventListener('blur', dismiss);
    document.addEventListener('scroll', dismiss, true);
    onCleanup(() => {
      document.removeEventListener('pointerdown', onPointerDown, true);
      removeEventListener('resize', dismiss);
      removeEventListener('blur', dismiss);
      document.removeEventListener('scroll', dismiss, true);
    });
  });

  return (
    <Portal>
      <MenuPanel
        ref={(el) => (panel = el)}
        label={props.label}
        class="fixed"
        style={{ left: `${pos().x}px`, top: `${pos().y}px` }}
        close={() => close()}
        onEscape={() => close()}
        onTab={() => close(false)}
      >
        {props.children}
      </MenuPanel>
    </Portal>
  );
}

/** An item that opens a further menu to its side (on hover, →, Enter or Space). */
export function SubMenu(props: { label: string; disabled?: boolean; children: JSX.Element }) {
  const menu = useContext(MenuContext);
  const [open, setOpen] = createSignal<null | 'mouse' | 'keyboard'>(null);
  const [leftward, setLeftward] = createSignal(false);
  let trigger!: HTMLButtonElement;
  const show = (how: 'mouse' | 'keyboard') => {
    // Open toward the side with room.
    setLeftward(trigger.getBoundingClientRect().right + 240 > innerWidth);
    setOpen(how);
  };
  return (
    <div class="relative" onMouseEnter={() => !props.disabled && show('mouse')} onMouseLeave={() => setOpen(null)}>
      <button
        ref={trigger}
        role="menuitem"
        aria-haspopup="menu"
        aria-expanded={open() !== null}
        disabled={props.disabled}
        tabIndex={-1}
        class={ITEM}
        classList={{ 'bg-highlight': open() !== null }}
        onPointerMove={(e) => e.currentTarget.focus()}
        onClick={() => show('keyboard')}
        // Native, like the panel's handler, which would otherwise stop the key first.
        on:keydown={(e: KeyboardEvent) => {
          if (e.key === 'ArrowRight' || e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            e.stopPropagation();
            show('keyboard');
          }
        }}
      >
        {props.label}
        <Icon of={ChevronRight} class="ml-auto text-muted" />
      </button>
      <Show when={open()}>
        {(how) => (
          // Padding, not margin, bridges the gap so moving the pointer across keeps it open.
          <div
            ref={(el) =>
              // Shift up if it would run off the bottom of the window.
              requestAnimationFrame(() => {
                const over = el.getBoundingClientRect().bottom - (innerHeight - 8);
                if (over > 0) el.style.top = `${-6 - over}px`;
              })
            }
            class={`absolute -top-1.5 ${leftward() ? 'right-full pr-1' : 'left-full pl-1'}`}
          >
            <MenuPanel
              label={props.label}
              close={() => menu?.close()}
              onEscape={() => {
                setOpen(null);
                trigger.focus();
              }}
              autofocus={how() === 'keyboard'}
            >
              <div
                on:keydown={(e: KeyboardEvent) => {
                  if (e.key === 'ArrowLeft') {
                    e.preventDefault();
                    setOpen(null);
                    trigger.focus();
                  }
                }}
              >
                {props.children}
              </div>
            </MenuPanel>
          </div>
        )}
      </Show>
    </div>
  );
}

type ItemProps = ParentProps<{
  onSelect(): void;
  /** Makes this a radio item (with a check mark when true). */
  checked?: boolean;
  /** With `checked`: an independent on/off item rather than one of a set. */
  checkbox?: boolean;
  /** Leave the menu open after selecting, e.g. to toggle several checkboxes. */
  keepOpen?: boolean;
  disabled?: boolean;
  title?: string;
  /** A keyboard shortcut shown at the right, e.g. "⌘⌫". */
  shortcut?: string;
  /** Red, for destructive items. */
  danger?: boolean;
}>;

export function MenuItem(props: ItemProps) {
  const menu = useContext(MenuContext);
  const isRadio = () => props.checked !== undefined;
  return (
    <button
      role={isRadio() ? (props.checkbox ? 'menuitemcheckbox' : 'menuitemradio') : 'menuitem'}
      aria-checked={isRadio() ? props.checked : undefined}
      disabled={props.disabled}
      title={props.title}
      tabIndex={-1}
      class={props.danger ? DANGER_ITEM : ITEM}
      // Hover moves the keyboard focus too, so only one item looks active.
      onPointerMove={(e) => e.currentTarget.focus()}
      onClick={() => {
        if (!props.keepOpen) menu?.close();
        props.onSelect();
      }}
    >
      {props.children}
      <Show when={props.shortcut}>
        <span class="ml-auto pl-4 text-xs text-faint">{props.shortcut}</span>
      </Show>
      <Show when={isRadio()}>
        <Icon of={Check} class={`ml-auto text-accent ${props.checked ? '' : 'invisible'}`} />
      </Show>
    </button>
  );
}

export function MenuSeparator() {
  return <div role="separator" class="mx-1 my-1.5 h-px bg-line" />;
}

/** Non-interactive heading text inside a menu. */
export function MenuLabel(props: ParentProps) {
  return (
    <div class="px-2.5 pt-1 pb-1.5 text-[11px] font-semibold tracking-[.06em] text-muted uppercase">
      {props.children}
    </div>
  );
}
