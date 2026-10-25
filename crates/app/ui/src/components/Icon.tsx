import type { LucideProps } from 'lucide-solid';
import type { Component } from 'solid-js';
import { Dynamic } from 'solid-js/web';

/**
 * A Lucide icon (https://lucide.dev) at the app's sizes, with the same
 * stroke at every size. Import icons one by one (`lucide-solid/icons/x`) so
 * only those used are bundled. Colour comes from the text colour.
 */
export function Icon(props: {
  of: Component<LucideProps>;
  /** Pixels: 12 in badges and chips, 14 inline and in menus, 16 on toolbar buttons. */
  size?: 12 | 14 | 16 | 20 | 40;
  class?: string;
}) {
  return (
    <Dynamic
      component={props.of}
      size={props.size ?? 14}
      strokeWidth={1.75}
      absoluteStrokeWidth
      class={`flex-none ${props.class ?? ''}`}
      aria-hidden="true"
    />
  );
}
