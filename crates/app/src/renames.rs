//! Renaming a torrent's folder and files for a TV library (naming.rs): the plan
//! for the add dialog and the Smart Rename dialog to show, and carrying it out.
//!
//! Transmission can't name things as it adds them, so a torrent is renamed just
//! after it's added. The plan's made again from the names Transmission reports
//! (they can differ from a .torrent's: it tidies some characters), files first,
//! then the torrent's folder, whose name is remembered as its original.

use std::collections::BTreeSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;
use transmission::{Ids, TorrentField};
use ts_rs::TS;

use crate::commands::Problem;
use crate::naming::{self, NameOptions, NamePlan, TorrentFile};
use crate::originals::Originals;
use crate::poller::Poller;
use crate::secrets::Secrets;
use crate::tmdb::{self, Tmdb};

/// Whose names to plan: a .torrent file here (before it's added), or a torrent in Transmission.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PlanSource {
    File { path: String },
    Torrent { hash: String },
}

/// How renaming went.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct NamesApplied {
    /// Files and folders renamed.
    pub renamed: u32,
    /// What couldn't be, and why.
    pub failures: Vec<String>,
    /// The folder a one-file movie was moved into.
    pub moved_to: Option<String>,
}

fn client(poller: &Poller) -> Result<Arc<transmission::Client>, Problem> {
    poller.client().ok_or_else(|| Problem::from("Not connected to Transmission.".to_owned()))
}

/// A torrent's name, files and download folder, as Transmission has them.
async fn torrent_names(poller: &Poller, hash: &str) -> Result<(String, Vec<TorrentFile>, String), Problem> {
    let fields =
        [TorrentField::Name, TorrentField::Files, TorrentField::MetadataPercentComplete, TorrentField::DownloadDir];
    let got = client(poller)?.torrent_get(&Ids::hashes([hash]), &fields).await.map_err(|e| Problem::from(&e))?;
    let t = got
        .torrents
        .into_iter()
        .next()
        .ok_or_else(|| Problem::from("That torrent isn't there any more.".to_owned()))?;
    let files: Vec<TorrentFile> =
        t.files.unwrap_or_default().into_iter().map(|f| TorrentFile { path: f.name, size: f.length }).collect();
    if t.metadata_percent_complete.is_some_and(|m| m < 1.0) || files.is_empty() {
        return Err(Problem::from(
            "Transmission doesn't know this torrent's files yet (it's still fetching them for a magnet link). Try again once it does."
                .to_owned(),
        ));
    }
    Ok((t.name.unwrap_or_default(), files, t.download_dir.unwrap_or_default()))
}

/// The plan, with TMDB's titles when a show's chosen there. TMDB failing isn't fatal:
/// the titles in the file names stand in, and the plan says why.
async fn make_plan(
    name: &str,
    files: &[TorrentFile],
    options: &NameOptions,
    secrets: &Arc<Secrets>,
    tmdb: &Tmdb,
) -> Result<NamePlan, Problem> {
    let mut titles = Default::default();
    let mut problem = None;
    if let Some(show) = options.episodes.as_ref().and_then(|e| e.tmdb) {
        let seasons: BTreeSet<u32> = naming::episodes(files).into_iter().flatten().map(|e| e.season).collect();
        match tmdb::key(secrets).await? {
            None => problem = Some("No TMDB key (Settings › General), so titles come from the file names.".to_owned()),
            Some(key) => match tmdb.titles(&key, show, seasons).await {
                Ok(found) => titles = found,
                Err(why) => problem = Some(format!("{why} Titles come from the file names.")),
            },
        }
    }
    let mut plan = naming::plan(name, files, options, &titles);
    plan.problem = problem;
    Ok(plan)
}

/// What renaming would do, to show before it's done.
#[tauri::command]
pub async fn name_plan(
    source: PlanSource,
    options: NameOptions,
    poller: State<'_, Poller>,
    secrets: State<'_, Arc<Secrets>>,
    tmdb: State<'_, Tmdb>,
) -> Result<NamePlan, Problem> {
    let (name, files) = match source {
        PlanSource::File { path } => {
            let bytes = crate::actions::read_torrent_bytes(std::path::Path::new(&path))?;
            let meta = transmission::read_metainfo(&bytes).map_err(|e| Problem::from(e.to_string()))?;
            let files = meta.files.into_iter().map(|f| TorrentFile { path: f.path, size: f.length }).collect();
            (meta.name, files)
        }
        PlanSource::Torrent { hash } => {
            let (name, files, _) = torrent_names(&poller, &hash).await?;
            (name, files)
        }
    };
    make_plan(&name, &files, &options, &secrets, &tmdb).await
}

/// Rename a torrent's files, then its folders, then the torrent itself, as planned;
/// then move a one-file movie into a folder of its own.
#[tauri::command]
pub async fn apply_names(
    hash: String,
    options: NameOptions,
    poller: State<'_, Poller>,
    secrets: State<'_, Arc<Secrets>>,
    tmdb: State<'_, Tmdb>,
    originals: State<'_, Originals>,
) -> Result<NamesApplied, Problem> {
    let (name, files, download_dir) = torrent_names(&poller, &hash).await?;
    let plan = make_plan(&name, &files, &options, &secrets, &tmdb).await?;
    let client = client(&poller)?;
    let ids = Ids::hashes([hash.as_str()]);
    let mut applied = NamesApplied::default();
    for file in &plan.files {
        let Some(to) = &file.to else { continue };
        match client.torrent_rename_path(&ids, &file.path, to).await {
            Ok(_) => applied.renamed += 1,
            Err(e) => applied.failures.push(format!("{}: {e}", file.path.rsplit('/').next().unwrap_or(&file.path))),
        }
    }
    for folder in &plan.folders {
        let Some(to) = &folder.to else { continue };
        match client.torrent_rename_path(&ids, &folder.path, to).await {
            Ok(_) => applied.renamed += 1,
            Err(e) => applied.failures.push(format!("{}: {e}", folder.path.rsplit('/').next().unwrap_or(&folder.path))),
        }
    }
    if let Some(top) = &plan.top {
        match client.torrent_rename_path(&ids, &plan.name, top).await {
            Ok(_) => {
                applied.renamed += 1;
                originals.remember(&hash, &plan.name);
            }
            Err(e) => applied.failures.push(format!("{}: {e}", plan.name)),
        }
    }
    if let Some(folder) = &plan.own_folder {
        let location = format!("{}/{folder}", download_dir.trim_end_matches('/'));
        match client.torrent_set_location(&ids, &location, true).await {
            Ok(()) => applied.moved_to = Some(location),
            Err(e) => applied.failures.push(format!("Moving it into {folder}: {e}")),
        }
    }
    poller.poll_now();
    Ok(applied)
}
