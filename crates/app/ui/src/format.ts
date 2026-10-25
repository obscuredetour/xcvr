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
