// The connection to Transmission, the session's speeds, and the turtle and
// speed limits, as the poller reports them (crates/app/src/poller.rs).
// Read-only here: Settings changes them through the api.

import { createSignal } from 'solid-js';

import {
  api,
  type ConnectionState,
  errorMessage,
  onConnection,
  onSession,
  onStats,
  type Session,
  type SessionStats,
} from './api';
import { toast } from './toast';

const [connection, setConnection] = createSignal<ConnectionState>({ state: 'notConfigured' });
const [stats, setStats] = createSignal<SessionStats | null>(null);
/** The turtle (alternative speeds) and the speed limits. */
const [session, setSession] = createSignal<Session | null>(null);
export { connection, session, stats };

/** Switch the turtle; with a schedule on, this holds until the schedule next turns. */
export async function setTurtle(on: boolean): Promise<void> {
  setSession((s) => (s ? { ...s, alt_speed_enabled: on } : s));
  try {
    setSession(await api.setSession({ alt_speed_enabled: on }));
  } catch (err) {
    toast(errorMessage(err), { error: true });
  }
}

export const isConnected = () => connection().state === 'connected';

/** How the connection shows: its tone (the status dot), a short line, the details, a hint. */
export type Described = {
  tone: 'idle' | 'busy' | 'good' | 'bad';
  host: string | null;
  /** For the status bar: "Connected", "Connecting…", "Can't reach host". */
  short: string;
  /** The full message, when there's more to say. */
  detail: string | null;
  hint: string | null;
  /** Whether Try Again makes sense. */
  retryable: boolean;
};

export function describe(s: ConnectionState): Described {
  const base = { host: null, detail: null, hint: null, retryable: false };
  switch (s.state) {
    case 'notConfigured':
      return { ...base, tone: 'idle', short: 'Not connected' };
    case 'connecting':
      return { ...base, tone: 'busy', host: s.host, short: 'Connecting…' };
    case 'connected':
      return {
        ...base,
        tone: 'good',
        host: s.host,
        short: 'Connected',
        detail: `Transmission ${s.version.split(' ')[0]}`,
      };
    case 'authFailed':
      return { ...base, tone: 'bad', host: s.host, short: 'Sign-in failed', detail: s.message, retryable: true };
    case 'failed':
      return { ...base, tone: 'bad', host: s.host, short: 'Can’t connect', detail: s.message, retryable: true };
    case 'unreachable':
      return {
        tone: 'bad',
        host: s.host,
        short: `Can’t reach host · retrying in ${s.retry}s`,
        detail: s.message,
        hint: s.hint,
        retryable: true,
      };
  }
}

export const described = () => describe(connection());

/** Follow the poller: events first, then the current state, so nothing is missed in between. */
export async function watchConnection(): Promise<() => void> {
  const offs = await Promise.all([onConnection(setConnection), onStats(setStats), onSession(setSession)]);
  const [state, current, brief] = await Promise.all([api.connectionState(), api.sessionStats(), api.sessionBrief()]);
  setConnection(state);
  setStats(current);
  setSession(brief);
  return () => offs.forEach((off) => off());
}
