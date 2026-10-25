import { Show } from 'solid-js';

import {
  act,
  closeTorrentMenu,
  copyMagnets,
  isRunning,
  openHere,
  remove,
  renameTorrent,
  setLocation,
  torrentMenu,
} from '../actions';
import { deleteKeys, isMac, keys } from '../keys';
import { torrent } from '../torrents';
import { ContextMenu, MenuItem, MenuSeparator, SubMenu } from './Menu';

/** The right-click menu for torrents (the selection, or the one clicked). Mounted once in App. */
export function TorrentMenuHost() {
  return (
    <Show when={torrentMenu()} keyed>
      {(m) => <TorrentMenu x={m.x} y={m.y} hashes={m.hashes} />}
    </Show>
  );
}

function TorrentMenu(props: { x: number; y: number; hashes: string[] }) {
  const ts = () => props.hashes.map((h) => torrent(h));
  const anyRunning = () => ts().some(isRunning);
  const anyStopped = () => ts().some((t) => t?.status === 0);
  const run = (fn: () => Promise<void>) => () => void fn();

  return (
    <ContextMenu
      label={props.hashes.length === 1 ? 'Torrent' : `${props.hashes.length} torrents`}
      x={props.x}
      y={props.y}
      onClose={closeTorrentMenu}
    >
      <Show when={props.hashes.length === 1}>
        <MenuItem shortcut={keys('mod+ArrowDown')} onSelect={run(() => openHere(props.hashes[0]!, null, false))}>
          Open
        </MenuItem>
        <MenuItem shortcut={keys('mod+R')} onSelect={run(() => openHere(props.hashes[0]!, null, true))}>
          {isMac ? 'Show in Finder' : 'Show in File Manager'}
        </MenuItem>
        <MenuSeparator />
      </Show>
      <MenuItem disabled={!anyStopped()} shortcut={keys('Space')} onSelect={run(() => act('start', props.hashes))}>
        Resume
      </MenuItem>
      <MenuItem disabled={!anyRunning()} shortcut={keys('Space')} onSelect={run(() => act('stop', props.hashes))}>
        Pause
      </MenuItem>
      <MenuItem
        title="Start now, ahead of anything waiting in the queue"
        onSelect={run(() => act('startNow', props.hashes))}
      >
        Start Now
      </MenuItem>
      <MenuSeparator />
      <MenuItem onSelect={run(() => act('verify', props.hashes))}>Verify Local Data</MenuItem>
      <MenuItem title="Ask the trackers for more peers now" onSelect={run(() => act('reannounce', props.hashes))}>
        Ask Trackers for More Peers
      </MenuItem>
      <SubMenu label="Queue">
        <MenuItem onSelect={run(() => act('queueTop', props.hashes))}>Move to Top</MenuItem>
        <MenuItem onSelect={run(() => act('queueUp', props.hashes))}>Move Up</MenuItem>
        <MenuItem onSelect={run(() => act('queueDown', props.hashes))}>Move Down</MenuItem>
        <MenuItem onSelect={run(() => act('queueBottom', props.hashes))}>Move to Bottom</MenuItem>
      </SubMenu>
      <MenuSeparator />
      <Show when={props.hashes.length === 1}>
        <MenuItem shortcut={keys('F2')} onSelect={() => renameTorrent(props.hashes[0])}>
          Rename…
        </MenuItem>
      </Show>
      <MenuItem shortcut={keys('mod+L')} onSelect={() => setLocation(props.hashes)}>
        Set Location…
      </MenuItem>
      <MenuItem onSelect={run(() => copyMagnets(props.hashes))}>
        {props.hashes.length === 1 ? 'Copy Magnet Link' : 'Copy Magnet Links'}
      </MenuItem>
      <MenuSeparator />
      <MenuItem danger shortcut={keys(deleteKeys)} onSelect={run(() => remove(false, props.hashes))}>
        Remove…
      </MenuItem>
      <MenuItem danger shortcut={keys(`shift+${deleteKeys}`)} onSelect={run(() => remove(true, props.hashes))}>
        Remove and Delete Files…
      </MenuItem>
    </ContextMenu>
  );
}
