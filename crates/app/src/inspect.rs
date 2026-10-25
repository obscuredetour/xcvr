//! Commands behind the inspector: following one torrent, changing its options
//! and files, renaming it, moving it, and opening or revealing it here
//! (through the path mappings).

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use transmission::{Ids, Renamed, Torrent, TorrentField, TorrentSet};
use ts_rs::TS;

use crate::commands::Problem;
use crate::mappings::{Mappings, PathMapping};
use crate::originals::Originals;
use crate::poller::Poller;

fn client(poller: &Poller) -> Result<std::sync::Arc<transmission::Client>, Problem> {
    poller.client().ok_or_else(|| Problem::from("Not connected to Transmission.".to_owned()))
}

fn problem(e: transmission::Error) -> Problem {
    Problem::from(&e)
}

/// Show a torrent in the inspector: its details now, then `details` events as they
/// change. None (or a torrent that's gone) stops following.
#[tauri::command]
pub async fn inspect(hash: Option<String>, poller: State<'_, Poller>) -> Result<Option<Torrent>, Problem> {
    let Some(hash) = hash else {
        poller.follow(None, None);
        return Ok(None);
    };
    let client = client(&poller)?;
    let got = client.torrent_get(&Ids::hashes([hash.as_str()]), TorrentField::DETAIL).await.map_err(problem)?;
    let details = got.torrents.into_iter().next();
    poller.follow(details.as_ref().map(|_| hash), details.clone());
    Ok(details)
}

/// Change torrents' options, files or labels.
#[tauri::command]
pub async fn set_torrents(hashes: Vec<String>, set: TorrentSet, poller: State<'_, Poller>) -> Result<(), Problem> {
    if hashes.is_empty() {
        return Ok(());
    }
    let client = client(&poller)?;
    if let Some(labels) = &set.labels {
        for l in labels {
            transmission::label::normalize(l).map_err(|e| Problem::from(format!("“{l}”: {e}")))?;
        }
    }
    let result = client.torrent_set(&Ids::hashes(hashes), &set.without_empty_file_lists()).await;
    poller.poll_now();
    result.map_err(problem)
}

/// Rename a torrent's top folder or file (`path` is its current name), or a file or
/// folder inside it (`path` as in its files). `name` is the new last part.
#[tauri::command]
pub async fn rename_torrent(
    hash: String,
    path: String,
    name: String,
    poller: State<'_, Poller>,
    originals: State<'_, Originals>,
) -> Result<Renamed, Problem> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Problem::from("Enter a name.".to_owned()));
    }
    if name.contains('/') || name == "." || name == ".." {
        return Err(Problem::from("A name can't contain “/”.".to_owned()));
    }
    let client = client(&poller)?;
    let result = client.torrent_rename_path(&Ids::hashes([hash.as_str()]), &path, name).await;
    poller.poll_now();
    let renamed = result.map_err(problem)?;
    // The torrent itself (its top folder or file): Transmission won't remember what it was.
    if !path.contains('/') {
        originals.remember(&hash, &path);
    }
    Ok(renamed)
}

/// What a torrent was called before it was renamed, when that's known and differs:
/// from Transmission's own copy of its .torrent file, when a path mapping reaches it
/// (the original's always there), else from a rename made in Transceiver here.
#[tauri::command]
pub async fn original_name(
    hash: String,
    poller: State<'_, Poller>,
    mappings: State<'_, Mappings>,
    originals: State<'_, Originals>,
) -> Result<Option<String>, Problem> {
    let client = client(&poller)?;
    let fields = [TorrentField::Name, TorrentField::TorrentFile];
    let got = client.torrent_get(&Ids::hashes([hash.as_str()]), &fields).await.map_err(problem)?;
    let Some(t) = got.torrents.into_iter().next() else { return Ok(None) };

    let on_host = t.torrent_file.as_deref().and_then(|f| mappings.to_local(f));
    let from_file = match on_host {
        // Off the main thread: it may be on a network share.
        Some(local) => tauri::async_runtime::spawn_blocking(move || {
            crate::actions::read_torrent_bytes(&local).ok().and_then(|b| transmission::read_metainfo(&b).ok())
        })
        .await
        .ok()
        .flatten()
        .map(|m| m.name)
        .filter(|n| !n.is_empty()),
        None => None,
    };
    let original = from_file.or_else(|| originals.get(&hash));
    Ok(original.filter(|o| Some(o) != t.name.as_ref()))
}

/// Point torrents at a folder on the host, moving their data there or not.
#[tauri::command]
pub async fn set_location(
    hashes: Vec<String>,
    location: String,
    move_data: bool,
    poller: State<'_, Poller>,
) -> Result<(), Problem> {
    let location = location.trim();
    if !location.starts_with('/') {
        return Err(Problem::from("Enter a full path on the host, starting with “/”.".to_owned()));
    }
    let client = client(&poller)?;
    let result = client.torrent_set_location(&Ids::hashes(hashes), location, move_data).await;
    poller.poll_now();
    result.map_err(problem)
}

/// A mapping as Settings shows it: whether its folder here exists (the share is mounted).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MappingView {
    pub host: String,
    pub local: String,
    pub local_exists: bool,
}

fn views(list: Vec<PathMapping>) -> Vec<MappingView> {
    list.into_iter()
        .map(|m| MappingView { local_exists: Path::new(&m.local).is_dir(), host: m.host, local: m.local })
        .collect()
}

#[tauri::command]
pub fn path_mappings(mappings: State<'_, Mappings>) -> Vec<MappingView> {
    views(mappings.list())
}

#[tauri::command]
pub fn set_path_mappings(list: Vec<PathMapping>, mappings: State<'_, Mappings>) -> Result<Vec<MappingView>, Problem> {
    Ok(views(mappings.set(list)?))
}

/// Open a torrent (or one of its files, by index) here, or reveal it in Finder or the
/// file manager, through the path mappings.
#[tauri::command]
pub async fn open_torrent(
    hash: String,
    file: Option<usize>,
    reveal: bool,
    app: AppHandle,
    poller: State<'_, Poller>,
    mappings: State<'_, Mappings>,
) -> Result<(), Problem> {
    let client = client(&poller)?;
    let fields = [TorrentField::DownloadDir, TorrentField::Name, TorrentField::Files];
    let got = client.torrent_get(&Ids::hashes([hash]), &fields).await.map_err(problem)?;
    let t = got.torrents.into_iter().next().ok_or_else(|| Problem::from("That torrent is gone.".to_owned()))?;
    let dir = t.download_dir.unwrap_or_default();
    let inside = match file {
        Some(i) => t.files.unwrap_or_default().get(i).map(|f| f.name.clone()),
        None => t.name,
    }
    .ok_or_else(|| Problem::from("That file isn't in the torrent.".to_owned()))?;
    let host_path = format!("{}/{inside}", dir.trim_end_matches('/'));

    let Some(local) = mappings.to_local(&host_path) else {
        return Err(Problem {
            message: format!("No path mapping covers {dir} on the host."),
            hint: Some("Add one in Settings › Paths: that folder on the host, and where it's mounted here.".into()),
        });
    };
    // An unfinished file may have Transmission's .part on the end.
    let part = local.with_file_name(format!(
        "{}.part",
        local.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    ));
    let found = if local.exists() {
        local
    } else if part.exists() {
        part
    } else {
        return Err(Problem {
            message: format!("Not found here: {}", local.display()),
            hint: Some("Is the share mounted? If it hasn't downloaded yet, there's nothing to open.".into()),
        });
    };

    let opener = app.opener();
    let result = if reveal {
        opener.reveal_item_in_dir(&found)
    } else {
        opener.open_path(found.to_string_lossy(), None::<&str>)
    };
    result.map_err(|e| Problem::from(format!("Couldn't open {}: {e}", found.display())))
}
