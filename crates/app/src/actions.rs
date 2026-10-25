//! Commands that change torrents: start, stop, verify, queue moves, remove,
//! add. Each polls straight after, so the table catches up at once.

use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;
use transmission::{
    AddOutcome, AddTorrent, Client, FreeSpace, Ids, Metainfo, QueueMove, SessionField, TorrentField, TorrentSource,
};
use ts_rs::TS;

use crate::commands::Problem;
use crate::poller::Poller;

/// Larger than any real .torrent file; a bigger file isn't one.
const MAX_TORRENT_FILE: u64 = 20 * 1024 * 1024;

fn client(poller: &Poller) -> Result<Arc<Client>, Problem> {
    poller.client().ok_or_else(|| Problem::from("Not connected to Transmission.".to_owned()))
}

fn problem(e: transmission::Error) -> Problem {
    Problem::from(&e)
}

/// What a row, bulk or keyboard action does.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum TorrentAction {
    Start,
    /// Start, skipping the queue.
    StartNow,
    Stop,
    Verify,
    /// Ask the trackers for more peers now.
    Reannounce,
    QueueTop,
    QueueUp,
    QueueDown,
    QueueBottom,
}

#[tauri::command]
pub async fn torrent_action(
    action: TorrentAction,
    hashes: Vec<String>,
    poller: State<'_, Poller>,
) -> Result<(), Problem> {
    if hashes.is_empty() {
        return Ok(());
    }
    let client = client(&poller)?;
    let ids = Ids::hashes(hashes);
    let result = match action {
        TorrentAction::Start => client.torrent_start(&ids).await,
        TorrentAction::StartNow => client.torrent_start_now(&ids).await,
        TorrentAction::Stop => client.torrent_stop(&ids).await,
        TorrentAction::Verify => client.torrent_verify(&ids).await,
        TorrentAction::Reannounce => client.torrent_reannounce(&ids).await,
        TorrentAction::QueueTop => client.queue_move(&ids, QueueMove::Top).await,
        TorrentAction::QueueUp => client.queue_move(&ids, QueueMove::Up).await,
        TorrentAction::QueueDown => client.queue_move(&ids, QueueMove::Down).await,
        TorrentAction::QueueBottom => client.queue_move(&ids, QueueMove::Bottom).await,
    };
    poller.poll_now();
    result.map_err(problem)
}

/// Remove torrents from Transmission, and with `delete_data` their files on the host.
#[tauri::command]
pub async fn remove_torrents(hashes: Vec<String>, delete_data: bool, poller: State<'_, Poller>) -> Result<(), Problem> {
    if hashes.is_empty() {
        return Ok(());
    }
    let client = client(&poller)?;
    let result = client.torrent_remove(&Ids::hashes(hashes), delete_data).await;
    poller.poll_now();
    result.map_err(problem)
}

/// The magnet links of torrents, for copying.
#[tauri::command]
pub async fn magnet_links(hashes: Vec<String>, poller: State<'_, Poller>) -> Result<Vec<String>, Problem> {
    let client = client(&poller)?;
    let got = client.torrent_get(&Ids::hashes(hashes), &[TorrentField::MagnetLink]).await.map_err(problem)?;
    Ok(got.torrents.into_iter().filter_map(|t| t.magnet_link).collect())
}

/// What a .torrent file on this machine is, for the add dialog's list.
#[tauri::command]
pub async fn read_torrent_file(path: String) -> Result<Metainfo, Problem> {
    let bytes = read_torrent_bytes(Path::new(&path))?;
    transmission::read_metainfo(&bytes).map_err(|e| Problem::from(e.to_string()))
}

/// A .torrent file's bytes, refusing anything too big to be one.
pub(crate) fn read_torrent_bytes(path: &Path) -> Result<Vec<u8>, Problem> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let size = std::fs::metadata(path).map_err(|e| Problem::from(format!("Couldn't read {name}: {e}")))?.len();
    if size > MAX_TORRENT_FILE {
        return Err(Problem::from(format!("{name} is too big to be a .torrent file.")));
    }
    std::fs::read(path).map_err(|e| Problem::from(format!("Couldn't read {name}: {e}")))
}

/// One thing to add: a magnet link or URL, or a .torrent file on this machine.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AddSource {
    Link { link: String },
    File { path: String },
}

/// Options for everything added at once.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AddOptions {
    /// A folder on the host; the session's default when None.
    pub download_dir: Option<String>,
    pub paused: bool,
    pub labels: Vec<String>,
}

/// How adding one source went.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum AddResult {
    Added {
        name: String,
        hash: String,
    },
    /// Transmission already had it; nothing changed.
    Duplicate {
        name: String,
        hash: String,
    },
    Failed {
        message: String,
    },
}

/// Add torrents, each on its own: one failing doesn't stop the rest.
#[tauri::command]
pub async fn add_torrents(
    sources: Vec<AddSource>,
    options: AddOptions,
    poller: State<'_, Poller>,
) -> Result<Vec<AddResult>, Problem> {
    let client = client(&poller)?;
    let labels = options
        .labels
        .iter()
        .map(|l| transmission::label::normalize(l).map_err(|e| Problem::from(format!("“{l}”: {e}"))))
        .collect::<Result<Vec<_>, _>>()?;
    let download_dir = options.download_dir.map(|d| d.trim().to_owned()).filter(|d| !d.is_empty());

    let mut results = Vec::with_capacity(sources.len());
    for source in sources {
        let source = match source {
            AddSource::Link { link } => Ok(TorrentSource::Link(link.trim().to_owned())),
            AddSource::File { path } => read_torrent_bytes(Path::new(&path)).map(TorrentSource::Metainfo),
        };
        let result = match source {
            Err(p) => AddResult::Failed { message: p.message },
            Ok(source) => {
                let mut add = AddTorrent::new(source);
                add.download_dir = download_dir.clone();
                add.paused = options.paused;
                add.labels = labels.clone();
                match client.torrent_add(&add).await {
                    Ok(AddOutcome::Added(t)) => AddResult::Added { name: t.name, hash: t.hash_string },
                    Ok(AddOutcome::Duplicate(t)) => AddResult::Duplicate { name: t.name, hash: t.hash_string },
                    Err(e) => AddResult::Failed { message: e.to_string() },
                }
            }
        };
        results.push(result);
    }
    poller.poll_now();
    Ok(results)
}

/// The host's defaults for adding: its download folder, and whether it starts what's added.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AddDefaults {
    pub download_dir: String,
    pub start_added: bool,
}

#[tauri::command]
pub async fn add_defaults(poller: State<'_, Poller>) -> Result<AddDefaults, Problem> {
    let client = client(&poller)?;
    let s =
        client.session_get(&[SessionField::DownloadDir, SessionField::StartAddedTorrents]).await.map_err(problem)?;
    Ok(AddDefaults {
        download_dir: s.download_dir.unwrap_or_default(),
        start_added: s.start_added_torrents.unwrap_or(true),
    })
}

/// Free space in a folder on the host. A folder that doesn't exist yet is a Problem
/// the add dialog shows quietly (Transmission makes it when it's first used).
#[tauri::command]
pub async fn free_space(path: String, poller: State<'_, Poller>) -> Result<FreeSpace, Problem> {
    let client = client(&poller)?;
    client.free_space(path.trim()).await.map_err(problem)
}
