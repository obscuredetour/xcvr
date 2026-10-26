//! Labels across torrents. Transmission's `torrent_set` replaces a torrent's
//! whole label list, so changing one label on several torrents means reading
//! what each has, working out each new list, and sending one `torrent_set` per
//! distinct result (torrents that end up alike go together).

use std::collections::BTreeMap;

use serde::Serialize;
use tauri::State;
use transmission::{Client, Ids, TorrentField, TorrentSet};
use ts_rs::TS;

use crate::commands::Problem;
use crate::poller::Poller;

/// Labels as Transmission will store them (trimmed, no blanks, no repeats, order kept),
/// or what's wrong with one.
fn normalized(labels: &[String]) -> Result<Vec<String>, Problem> {
    let mut out: Vec<String> = Vec::new();
    for l in labels {
        let n = transmission::label::normalize(l).map_err(|e| Problem::from(format!("“{}”: {e}", l.trim())))?;
        if !out.contains(&n) {
            out.push(n);
        }
    }
    Ok(out)
}

/// Each torrent's new labels, given what it has: `change` returns the new list.
/// Only torrents whose labels change are kept, grouped by their new list.
fn plan(
    current: Vec<(String, Vec<String>)>,
    change: impl Fn(&[String]) -> Vec<String>,
) -> BTreeMap<Vec<String>, Vec<String>> {
    let mut groups: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
    for (hash, labels) in current {
        let next = change(&labels);
        if next != labels {
            groups.entry(next).or_default().push(hash);
        }
    }
    groups
}

/// Add `add` (at the end, in order) and take away `remove`.
fn edited(labels: &[String], add: &[String], remove: &[String]) -> Vec<String> {
    let mut out: Vec<String> = labels.iter().filter(|l| !remove.contains(l)).cloned().collect();
    for a in add {
        if !out.contains(a) && !remove.contains(a) {
            out.push(a.clone());
        }
    }
    out
}

/// `from` becomes `to` where it stands; merges when the torrent already has `to`.
fn renamed(labels: &[String], from: &str, to: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in labels {
        let l = if l == from { to } else { l.as_str() };
        if !out.iter().any(|o| o == l) {
            out.push(l.to_owned());
        }
    }
    out
}

/// The labels of torrents (all of them for `Ids::All`), by hash.
async fn current(client: &Client, ids: &Ids) -> Result<Vec<(String, Vec<String>)>, transmission::Error> {
    let got = client.torrent_get(ids, &[TorrentField::HashString, TorrentField::Labels]).await?;
    Ok(got.torrents.into_iter().filter_map(|t| Some((t.hash_string?, t.labels.unwrap_or_default()))).collect())
}

/// Send each group's labels. Returns how many torrents changed.
async fn apply(client: &Client, groups: BTreeMap<Vec<String>, Vec<String>>) -> Result<usize, transmission::Error> {
    let mut changed = 0;
    for (labels, hashes) in groups {
        changed += hashes.len();
        client.torrent_set(&Ids::hashes(hashes), &TorrentSet { labels: Some(labels), ..Default::default() }).await?;
    }
    Ok(changed)
}

pub async fn edit(
    client: &Client,
    hashes: Vec<String>,
    add: &[String],
    remove: &[String],
) -> transmission::Result<usize> {
    let have = current(client, &Ids::hashes(hashes)).await?;
    apply(client, plan(have, |l| edited(l, add, remove))).await
}

pub async fn rename(client: &Client, from: &str, to: &str) -> transmission::Result<usize> {
    let have = current(client, &Ids::All).await?;
    apply(client, plan(have, |l| renamed(l, from, to))).await
}

pub async fn delete(client: &Client, label: &str) -> transmission::Result<usize> {
    let have = current(client, &Ids::All).await?;
    let remove = [label.to_owned()];
    apply(client, plan(have, |l| edited(l, &[], &remove))).await
}

/// How many torrents a label change touched.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct LabelsChanged {
    pub torrents: usize,
}

fn client(poller: &Poller) -> Result<std::sync::Arc<Client>, Problem> {
    poller.client().ok_or_else(|| Problem::from("Not connected to Transmission.".to_owned()))
}

/// Add and take away labels on torrents, keeping each torrent's other labels.
#[tauri::command]
pub async fn edit_labels(
    hashes: Vec<String>,
    add: Vec<String>,
    remove: Vec<String>,
    poller: State<'_, Poller>,
) -> Result<LabelsChanged, Problem> {
    let (add, remove) = (normalized(&add)?, normalized(&remove)?);
    if hashes.is_empty() || (add.is_empty() && remove.is_empty()) {
        return Ok(LabelsChanged { torrents: 0 });
    }
    let client = client(&poller)?;
    let result = edit(&client, hashes, &add, &remove).await;
    poller.poll_now();
    Ok(LabelsChanged { torrents: result.map_err(|e| Problem::from(&e))? })
}

/// Rename a label on every torrent that has it (merging into `to` where it's there already).
#[tauri::command]
pub async fn rename_label(from: String, to: String, poller: State<'_, Poller>) -> Result<LabelsChanged, Problem> {
    let to = normalized(&[to])?.pop().unwrap_or_default();
    let client = client(&poller)?;
    let result = rename(&client, &from, &to).await;
    poller.poll_now();
    Ok(LabelsChanged { torrents: result.map_err(|e| Problem::from(&e))? })
}

/// Take a label off every torrent that has it.
#[tauri::command]
pub async fn delete_label(label: String, poller: State<'_, Poller>) -> Result<LabelsChanged, Problem> {
    let client = client(&poller)?;
    let result = delete(&client, &label).await;
    poller.poll_now();
    Ok(LabelsChanged { torrents: result.map_err(|e| Problem::from(&e))? })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(labels: &[&str]) -> Vec<String> {
        labels.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn editing_keeps_the_rest_and_the_order() {
        assert_eq!(edited(&v(&["video", "4k"]), &v(&["linux"]), &[]), v(&["video", "4k", "linux"]));
        assert_eq!(edited(&v(&["video", "4k"]), &v(&["video"]), &[]), v(&["video", "4k"]), "already there");
        assert_eq!(edited(&v(&["video", "4k"]), &[], &v(&["video"])), v(&["4k"]));
        assert_eq!(edited(&v(&["a"]), &v(&["b"]), &v(&["b"])), v(&["a"]), "taking away wins");
    }

    #[test]
    fn renaming_merges() {
        assert_eq!(renamed(&v(&["tv", "4k"]), "tv", "shows"), v(&["shows", "4k"]));
        assert_eq!(renamed(&v(&["tv", "shows"]), "tv", "shows"), v(&["shows"]));
        assert_eq!(renamed(&v(&["4k"]), "tv", "shows"), v(&["4k"]));
    }

    #[test]
    fn planning_groups_alike_results_and_skips_the_unchanged() {
        let have = vec![
            ("a".to_owned(), v(&["video"])),
            ("b".to_owned(), v(&[])),
            ("c".to_owned(), v(&["linux"])),
            ("d".to_owned(), v(&["video", "linux"])),
        ];
        let groups = plan(have, |l| edited(l, &v(&["linux"]), &[]));
        assert_eq!(groups.len(), 2, "{groups:?}");
        assert_eq!(groups[&v(&["video", "linux"])], v(&["a"]));
        assert_eq!(groups[&v(&["linux"])], v(&["b"]));
    }

    #[test]
    fn labels_are_tidied_and_checked() {
        assert_eq!(normalized(&v(&[" linux ", "linux", "iso"])).unwrap(), v(&["linux", "iso"]));
        assert!(normalized(&v(&["a,b"])).is_err());
        assert!(normalized(&v(&["  "])).is_err());
    }

    /// Against a real Transmission (scripts/test-container.sh): torrents with
    /// different labels get one added, one renamed and one deleted, each keeping the rest.
    #[tokio::test]
    #[ignore = "needs a Transmission container: scripts/test-container.sh"]
    async fn container_labels_across_torrents() {
        use transmission::{AddTorrent, Config, TorrentSource};

        let port = std::env::var("XCVR_TEST_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(19091);
        let client = Client::new(Config::new("127.0.0.1", port).with_credentials("xcvr", "xcvr")).unwrap();
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let mut hashes = Vec::new();
        for (i, labels) in [v(&["video"]), v(&[]), v(&["video", "4k"])].into_iter().enumerate() {
            let hash = format!("{:032x}{i:08x}", nanos);
            let mut add = AddTorrent::new(TorrentSource::Link(format!("magnet:?xt=urn:btih:{hash}")));
            add.paused = true;
            add.labels = labels;
            client.torrent_add(&add).await.unwrap();
            hashes.push(hash);
        }
        let labels_of = |all: Vec<(String, Vec<String>)>| -> Vec<Vec<String>> {
            hashes.iter().map(|h| all.iter().find(|(x, _)| x == h).unwrap().1.clone()).collect()
        };

        // Adding "test" changes all three, keeping what each had.
        assert_eq!(edit(&client, hashes.clone(), &v(&["test"]), &[]).await.unwrap(), 3);
        let now = labels_of(current(&client, &Ids::hashes(hashes.clone())).await.unwrap());
        assert_eq!(now, [v(&["video", "test"]), v(&["test"]), v(&["video", "4k", "test"])]);

        // Renaming "video" everywhere touches only the two that have it.
        let tag = format!("vid-{nanos:x}");
        assert_eq!(rename(&client, "video", &tag).await.unwrap(), 2);
        let now = labels_of(current(&client, &Ids::hashes(hashes.clone())).await.unwrap());
        assert_eq!(now, [v(&[&tag, "test"]), v(&["test"]), v(&[&tag, "4k", "test"])]);

        // Deleting "test" leaves the rest.
        assert_eq!(delete(&client, "test").await.unwrap(), 3);
        let now = labels_of(current(&client, &Ids::hashes(hashes.clone())).await.unwrap());
        assert_eq!(now, [v(&[&tag]), v(&[]), v(&[&tag, "4k"])]);

        client.torrent_remove(&Ids::hashes(hashes), false).await.unwrap();
    }
}
