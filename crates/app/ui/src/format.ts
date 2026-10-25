// Numbers for people. Transmission counts in thousands (kB = 1000 bytes) unless
// the host is set otherwise; so do these.

import type { Torrent } from './api';

/** Bytes: "4.2 GB", "612 kB", "38 B". */
export function formatSize(bytes: number): string {
  if (bytes >= 1e12) return `${(bytes / 1e12).toFixed(2)} TB`;
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(1)} MB`;
  if (bytes >= 1e3) return `${Math.round(bytes / 1e3)} kB`;
  return `${bytes} B`;
}

/** Bytes per second: "1.4 MB/s", "86 kB/s", "0 kB/s". */
export function formatSpeed(bytesPerSecond: number): string {
  if (bytesPerSecond >= 1e6) return `${(bytesPerSecond / 1e6).toFixed(1)} MB/s`;
  return `${Math.round(bytesPerSecond / 1e3)} kB/s`;
}

/** 0–1 as a percentage: "42%", "99.5%" near the end, "100%". */
export function formatPercent(fraction: number): string {
  const p = fraction * 100;
  if (p >= 100) return '100%';
  return p >= 99 ? `${p.toFixed(1)}%` : `${Math.floor(p)}%`;
}

/** A duration in seconds, two units at most: "45s", "12m", "3h 12m", "2d 4h", "5w". */
export function formatDuration(seconds: number): string {
  const s = Math.max(0, Math.round(seconds));
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  if (h < 24) return m % 60 ? `${h}h ${m % 60}m` : `${h}h`;
  const d = Math.floor(h / 24);
  if (d < 14) return h % 24 ? `${d}d ${h % 24}h` : `${d}d`;
  return `${Math.floor(d / 7)}w`;
}

/** Time left: blank when not downloading, "∞" when it never will (Transmission's -2), "" when unknown (-1). */
export function formatEta(t: Torrent): string {
  if (t.status !== 4 || t.eta === undefined) return '';
  if (t.eta === -2) return '∞';
  if (t.eta < 0) return '';
  return formatDuration(t.eta);
}

/** Upload ratio: "1.42", "∞" for infinite (-2), "–" when there's nothing to compare against (-1). */
export function formatRatio(ratio: number | undefined): string {
  if (ratio === undefined || ratio === -1) return '–';
  if (ratio === -2) return '∞';
  return ratio < 10 ? ratio.toFixed(2) : ratio.toFixed(1);
}

const dateFormat = new Intl.DateTimeFormat(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
const dateTimeFormat = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' });

/** Unix seconds as a date ("Oct 25, 2026"); blank for 0 (never). */
export function formatDate(unix: number | undefined): string {
  return unix ? dateFormat.format(new Date(unix * 1000)) : '';
}

/** Unix seconds with the time, for tooltips. */
export function formatDateTime(unix: number | undefined): string {
  return unix ? dateTimeFormat.format(new Date(unix * 1000)) : '';
}

/** How a torrent's state colours its progress bar and status dot. */
export type Tone = 'download' | 'seed' | 'idle' | 'check' | 'error';

export function statusTone(t: Torrent): Tone {
  if ((t.error ?? 0) >= 2) return 'error';
  switch (t.status) {
    case 1:
    case 2:
      return 'check';
    case 3:
    case 4:
      return 'download';
    case 5:
    case 6:
      return 'seed';
    default:
      return 'idle';
  }
}

/** Progress as the bar shows it: the verify's progress while verifying, else how much is done. */
export function progressOf(t: Torrent): number {
  if (t.status === 2) return t.recheck_progress ?? 0;
  if ((t.metadata_percent_complete ?? 1) < 1) return t.metadata_percent_complete ?? 0;
  return t.percent_done ?? 0;
}

/** Status numbers (transmission's tr_torrent_activity), as the web UI words them. */
export function statusLabel(t: Torrent): string {
  if (t.error && t.error >= 2) return 'Error';
  switch (t.status) {
    case 0:
      return t.is_finished ? 'Finished' : 'Paused';
    case 1:
      return 'Queued to verify';
    case 2:
      return 'Verifying';
    case 3:
      return 'Queued';
    case 4:
      return (t.metadata_percent_complete ?? 1) < 1 ? 'Fetching metadata' : 'Downloading';
    case 5:
      return 'Queued to seed';
    case 6:
      return 'Seeding';
    default:
      return 'Unknown';
  }
}

/** The sidebar's status filters. Error: a tracker error or a local one, not a tracker warning. */
export const FILTERS = {
  all: () => true,
  active: (t: Torrent) => (t.rate_download ?? 0) > 0 || (t.rate_upload ?? 0) > 0 || t.status === 2,
  downloading: (t: Torrent) => t.status === 3 || t.status === 4,
  seeding: (t: Torrent) => t.status === 5 || t.status === 6,
  paused: (t: Torrent) => t.status === 0 && !t.is_finished,
  finished: (t: Torrent) => !!t.is_finished,
  error: (t: Torrent) => (t.error ?? 0) >= 2,
} satisfies Record<string, (t: Torrent) => boolean>;

export type FilterId = keyof typeof FILTERS;
