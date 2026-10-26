import { createResource, Show } from 'solid-js';
import Copy from 'lucide-solid/icons/copy';

import { renameTorrent, setLocation } from '../../actions';
import { api, type Torrent } from '../../api';
import {
  formatDateTime,
  formatDuration,
  formatEta,
  formatPercent,
  formatRatio,
  formatSize,
  formatSpeed,
  progressOf,
  statusTone,
} from '../../format';
import { toast } from '../../toast';
import { Icon } from '../Icon';
import { Facts, Section } from './parts';

const TONE_BG = { download: 'bg-down', seed: 'bg-up', idle: 'bg-faint', check: 'bg-info', error: 'bg-danger' };
const link = 'cursor-pointer text-link hover:underline';

async function copy(text: string, what: string) {
  try {
    await navigator.clipboard.writeText(text);
    toast(`${what} copied`);
  } catch {
    toast(`Couldn't copy the ${what.toLowerCase()}.`, { error: true });
  }
}

/** How it's doing, and what it is. */
export function GeneralTab(props: { t: Torrent }) {
  const t = () => props.t;
  // What it was called before a rename; asked again when its name changes.
  const [original] = createResource(
    () => [t().hash_string!, t().name] as const,
    ([hash]) => api.originalName(hash).catch(() => null),
  );
  const speed = (bps: number | undefined, peers: number | undefined, dir: string) =>
    bps ? `${formatSpeed(bps)} ${dir} ${peers ?? 0} ${peers === 1 ? 'peer' : 'peers'}` : '–';

  return (
    <>
      <Section>
        {/* What's done, then the bar: read like a caption, not a footnote. */}
        <div class="mb-1.5 flex justify-between text-xs text-muted tabular-nums">
          <span>
            {formatSize(Math.max(0, (t().size_when_done ?? 0) - (t().left_until_done ?? 0)))} of{' '}
            {formatSize(t().size_when_done ?? 0)} ({formatPercent(t().percent_done ?? 0)})
          </span>
          <Show when={formatEta(t())}>{(eta) => <span>{eta()} left</span>}</Show>
        </div>
        <div class="h-2 overflow-hidden rounded-full bg-element">
          <div
            class={`h-full rounded-full ${TONE_BG[statusTone(t())]}`}
            style={{ width: `${progressOf(t()) * 100}%` }}
          />
        </div>
      </Section>

      <Section title="Transfer">
        <Facts
          rows={[
            ['Downloading', speed(t().rate_download, t().peers_sending_to_us, 'from')],
            ['Uploading', speed(t().rate_upload, t().peers_getting_from_us, 'to')],
            ['Downloaded', formatSize(t().downloaded_ever ?? 0)],
            ['Uploaded', formatSize(t().uploaded_ever ?? 0)],
            ['Ratio', formatRatio(t().upload_ratio)],
            ['Corrupt', (t().corrupt_ever ?? 0) > 0 ? `${formatSize(t().corrupt_ever!)} discarded` : null],
            ['Downloading for', t().seconds_downloading ? formatDuration(t().seconds_downloading!) : null],
            ['Seeding for', t().seconds_seeding ? formatDuration(t().seconds_seeding!) : null],
          ]}
        />
      </Section>

      <Section title="Details">
        <Facts
          rows={[
            [
              'Name',
              <span>
                {t().name}{' '}
                <button class={link} onClick={() => renameTorrent(t().hash_string!)}>
                  Change…
                </button>
                <Show when={original()}>
                  {(o) => (
                    <span class="mt-0.5 block text-muted" title="Its name before it was renamed">
                      Originally {o()}
                    </span>
                  )}
                </Show>
              </span>,
            ],
            [
              'Size',
              `${formatSize(t().total_size ?? 0)}${
                t().piece_count ? ` (${t().piece_count} pieces of ${formatSize(t().piece_size ?? 0)})` : ''
              }`,
            ],
            [
              'Location',
              <span>
                {t().download_dir}{' '}
                <button class={link} onClick={() => setLocation([t().hash_string!])}>
                  Change…
                </button>
              </span>,
            ],
            [
              'Hash',
              <span class="inline-flex max-w-full items-center gap-1">
                <span class="truncate font-mono text-[11px]">{t().hash_string}</span>
                <button
                  class="flex-none text-muted hover:text-fg"
                  title="Copy the hash"
                  aria-label="Copy the hash"
                  onClick={() => void copy(t().hash_string!, 'Hash')}
                >
                  <Icon of={Copy} size={12} />
                </button>
              </span>,
            ],
            ['Added', formatDateTime(t().added_date)],
            ['Completed', formatDateTime(t().done_date) || null],
            [
              'Created',
              [formatDateTime(t().date_created), t().creator && `by ${t().creator}`].filter(Boolean).join(' '),
            ],
            ['Private', t().is_private ? 'Yes: its tracker only, no DHT or PEX' : null],
            ['Comment', t().comment ? <span class="whitespace-pre-wrap">{t().comment}</span> : null],
          ]}
        />
      </Section>
    </>
  );
}
