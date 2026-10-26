//! Commands behind the inspector: following one torrent, changing its options
//! and files, renaming it, moving it, and opening or revealing it here
//! (through the path mappings).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
    copies: State<'_, CopyNames>,
) -> Result<Option<String>, Problem> {
    let client = client(&poller)?;
    let fields = [TorrentField::Name, TorrentField::TorrentFile];
    let got = client.torrent_get(&Ids::hashes([hash.as_str()]), &fields).await.map_err(problem)?;
    let Some(t) = got.torrents.into_iter().next() else { return Ok(None) };

    let cached = copies.0.lock().unwrap_or_else(|e| e.into_inner()).get(&hash).cloned();
    let on_host = t.torrent_file.as_deref().and_then(|f| mappings.to_local(f));
    let from_file = match (cached, on_host) {
        (Some(name), _) => Some(name),
        // Off the main thread: it may be on a network share.
        (None, Some(local)) => {
            let name = tauri::async_runtime::spawn_blocking(move || name_in_copy(&local)).await.ok().flatten();
            if let Some(name) = &name {
                copies.0.lock().unwrap_or_else(|e| e.into_inner()).insert(hash.clone(), name.clone());
            }
            name
        }
        (None, None) => None,
    };
    let original = from_file.or_else(|| originals.get(&hash));
    Ok(original.filter(|o| Some(o) != t.name.as_ref()))
}

/// Names read from the host's .torrent copies, by hash. A copy never changes, so each is
/// read once (copies that couldn't be read are tried again next time).
#[derive(Default)]
pub struct CopyNames(Mutex<HashMap<String, String>>);

/// The name in a .torrent copy here (blocking: it may be on a network share).
fn name_in_copy(local: &Path) -> Option<String> {
    let bytes = crate::actions::read_torrent_bytes(local).ok()?;
    transmission::read_metainfo(&bytes).ok().map(|m| m.name).filter(|n| !n.is_empty())
}

/// Every renamed torrent's original name, by hash, for the table's second line: from the
/// host's .torrent copies where the mappings reach them, else from renames made here.
#[tauri::command]
pub async fn original_names(
    poller: State<'_, Poller>,
    mappings: State<'_, Mappings>,
    originals: State<'_, Originals>,
    copies: State<'_, CopyNames>,
) -> Result<HashMap<String, String>, Problem> {
    let client = client(&poller)?;
    let fields = [TorrentField::HashString, TorrentField::Name, TorrentField::TorrentFile];
    let torrents = client.torrent_get(&Ids::All, &fields).await.map_err(problem)?.torrents;

    let unread = copies.unread(&torrents, &mappings);
    if !unread.is_empty() {
        let read = tauri::async_runtime::spawn_blocking(move || read_copies(unread)).await.unwrap_or_default();
        copies.add(read);
    }
    Ok(copies.originals(torrents, &originals))
}

impl CopyNames {
    /// Torrents whose copy hasn't been read yet but can be reached here: (hash, path here).
    fn unread(&self, torrents: &[Torrent], mappings: &Mappings) -> Vec<(String, PathBuf)> {
        let known = self.0.lock().unwrap_or_else(|e| e.into_inner());
        torrents
            .iter()
            .filter_map(|t| {
                let hash = t.hash_string.as_ref().filter(|h| !known.contains_key(*h))?;
                Some((hash.clone(), mappings.to_local(t.torrent_file.as_deref()?)?))
            })
            .collect()
    }

    fn add(&self, names: Vec<(String, String)>) {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).extend(names);
    }

    /// Original names that differ from the current ones, by hash.
    fn originals(&self, torrents: Vec<Torrent>, originals: &Originals) -> HashMap<String, String> {
        let known = self.0.lock().unwrap_or_else(|e| e.into_inner());
        torrents
            .into_iter()
            .filter_map(|t| {
                let hash = t.hash_string?;
                let original = known.get(&hash).cloned().or_else(|| originals.get(&hash))?;
                (Some(&original) != t.name.as_ref()).then_some((hash, original))
            })
            .collect()
    }
}

/// Read the names in these copies (blocking); unreadable ones are left out.
fn read_copies(unread: Vec<(String, PathBuf)>) -> Vec<(String, String)> {
    unread.into_iter().filter_map(|(hash, local)| Some((hash, name_in_copy(&local)?))).collect()
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

/// Where the host keeps its copies of the .torrent files, which still hold each torrent's
/// name from before any rename, and whether the path mappings reach it from here.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TorrentsFolder {
    /// The folder on the host.
    pub host: String,
    /// Where it is here, through the mappings (null when none covers it).
    pub local: Option<String>,
    /// A torrent's copy can be read here, so original names show.
    pub readable: bool,
}

/// The folder holding a torrent's copy (`torrent_file`), as it maps here.
fn folder_of(torrent_file: &str, mappings: &Mappings) -> Option<(TorrentsFolder, Option<std::path::PathBuf>)> {
    let host = Path::new(torrent_file).parent()?.to_string_lossy().into_owned();
    let local = mappings.to_local(&host).map(|p| p.to_string_lossy().into_owned());
    Some((TorrentsFolder { host, local, readable: false }, mappings.to_local(torrent_file)))
}

/// The host's torrents folder, from one of its torrents (null with no torrents).
#[tauri::command]
pub async fn torrents_folder(
    poller: State<'_, Poller>,
    mappings: State<'_, Mappings>,
) -> Result<Option<TorrentsFolder>, Problem> {
    let Some(hash) = poller.torrents().into_iter().find_map(|t| t.hash_string) else { return Ok(None) };
    let client = client(&poller)?;
    let got = client.torrent_get(&Ids::hashes([hash.as_str()]), &[TorrentField::TorrentFile]).await.map_err(problem)?;
    let Some(file) = got.torrents.into_iter().next().and_then(|t| t.torrent_file) else { return Ok(None) };
    let Some((mut folder, local_file)) = folder_of(&file, &mappings) else { return Ok(None) };
    if let Some(path) = local_file {
        // Off the main thread: it may be on a network share.
        folder.readable = tauri::async_runtime::spawn_blocking(move || path.is_file()).await.unwrap_or(false);
    }
    Ok(Some(folder))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_torrents_folder_maps_like_any_other() {
        let path = std::env::temp_dir().join(format!("xcvr-folder-test-{}.json", std::process::id()));
        let mappings = Mappings::load(path.clone());
        let file = "/Users/alice/Library/Application Support/Transmission/Torrents/abc.torrent";

        let (folder, local_file) = folder_of(file, &mappings).unwrap();
        assert_eq!(folder.host, "/Users/alice/Library/Application Support/Transmission/Torrents");
        assert_eq!((folder.local, local_file), (None, None));

        mappings
            .set(vec![PathMapping {
                host: "/Users/alice/Library/Application Support/Transmission".into(),
                local: "/Volumes/Transmission".into(),
            }])
            .unwrap();
        let (folder, local_file) = folder_of(file, &mappings).unwrap();
        assert_eq!(folder.local.as_deref(), Some("/Volumes/Transmission/Torrents"));
        assert_eq!(local_file.as_deref(), Some(Path::new("/Volumes/Transmission/Torrents/abc.torrent")));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn original_names_come_from_the_copies_then_from_renames_here() {
        let dir = std::env::temp_dir().join(format!("xcvr-copies-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("torrents")).unwrap();
        // A minimal .torrent whose name is "Original Name".
        let info = b"d6:lengthi1e4:name13:Original Name12:piece lengthi16384e6:pieces20:aaaaaaaaaaaaaaaaaaaae";
        let mut copy = b"d4:info".to_vec();
        copy.extend_from_slice(info);
        copy.push(b'e');
        std::fs::write(dir.join("torrents/a.torrent"), &copy).unwrap();

        let mappings = Mappings::load(dir.join("paths.json"));
        mappings.set(vec![PathMapping { host: "/config".into(), local: dir.to_string_lossy().into() }]).unwrap();
        let originals = Originals::load(dir.join("originals.json"));
        originals.remember("c", "Renamed Here From");

        let t = |hash: &str, name: &str, file: Option<&str>| Torrent {
            hash_string: Some(hash.into()),
            name: Some(name.into()),
            torrent_file: file.map(Into::into),
            ..Default::default()
        };
        let torrents = vec![
            t("a", "New Name", Some("/config/torrents/a.torrent")),
            t("b", "Not Renamed", Some("/elsewhere/b.torrent")),
            t("c", "Renamed Here", None),
        ];

        let copies = CopyNames::default();
        let unread = copies.unread(&torrents, &mappings);
        assert_eq!(unread.len(), 1, "only a's copy is reachable");
        copies.add(read_copies(unread));
        assert!(copies.unread(&torrents, &mappings).is_empty(), "read once");

        let names = copies.originals(torrents.clone(), &originals);
        assert_eq!(names.get("a").map(String::as_str), Some("Original Name"));
        assert_eq!(names.get("c").map(String::as_str), Some("Renamed Here From"));
        assert_eq!(names.len(), 2);

        // Renamed back: no original to show.
        let back = vec![t("a", "Original Name", Some("/config/torrents/a.torrent"))];
        assert!(copies.originals(back, &originals).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
