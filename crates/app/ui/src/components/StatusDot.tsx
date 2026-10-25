import type { Described } from '../connection';

/** The connection's colour: grey idle, pulsing amber connecting, green connected, red failing. */
export function StatusDot(props: { tone: Described['tone'] }) {
  return (
    <span
      class="size-2 flex-none rounded-full"
      classList={{
        'bg-faint': props.tone === 'idle',
        'animate-pulse bg-warning': props.tone === 'busy',
        'bg-success': props.tone === 'good',
        'bg-danger': props.tone === 'bad',
      }}
      aria-hidden="true"
    />
  );
}
