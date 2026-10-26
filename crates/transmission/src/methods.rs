//! One method per RPC call the app uses.

use base64::Engine;
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};

use crate::client::{Client, NoParams};
use crate::error::Result;
use crate::ids::Ids;
use crate::session::{FreeSpace, PortTest, Session, SessionField, SessionSet, SessionStats};
use crate::torrent::{LimitMode, Priority, TorrentField, TorrentGet};

#[derive(Serialize)]
struct IdsParams<'a> {
    #[serde(skip_serializing_if = "Ids::is_all")]
    ids: &'a Ids,
}

#[derive(Serialize)]
struct TorrentGetParams<'a> {
    #[serde(skip_serializing_if = "Ids::is_all")]
    ids: &'a Ids,
    fields: &'a [TorrentField],
}

#[derive(Serialize)]
struct SessionGetParams<'a> {
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    fields: &'a [SessionField],
}

#[derive(Serialize)]
struct RemoveParams<'a> {
    #[serde(skip_serializing_if = "Ids::is_all")]
    ids: &'a Ids,
    delete_local_data: bool,
}

#[derive(Serialize)]
struct SetLocationParams<'a> {
    ids: &'a Ids,
    location: &'a str,
    #[serde(rename = "move")]
    move_data: bool,
}

#[derive(Serialize)]
struct RenameParams<'a> {
    ids: &'a Ids,
    path: &'a str,
    name: &'a str,
}

#[derive(Serialize)]
struct FreeSpaceParams<'a> {
    path: &'a str,
}

#[derive(Serialize)]
struct TorrentSetParams<'a> {
    #[serde(skip_serializing_if = "Ids::is_all")]
    ids: &'a Ids,
    #[serde(flatten)]
    set: &'a TorrentSet,
}

/// Changes to torrents (`torrent_set`): only the fields that are Some are sent.
/// File lists are indices into the torrent's `files`; an empty list means every file
/// (see [`TorrentSet::without_empty_file_lists`]).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
#[serde(default)]
pub struct TorrentSet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bandwidth_priority: Option<Priority>,
    /// kB/s.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_limited: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_wanted: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_unwanted: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub honors_session_limits: Option<bool>,
    /// Replaces the torrent's labels. Check them with [`crate::label::normalize`] first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_high: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_low: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_normal: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_position: Option<i64>,
    /// Minutes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_idle_limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_idle_mode: Option<LimitMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_ratio_limit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_ratio_mode: Option<LimitMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequential_download: Option<bool>,
    /// Announce URLs one per line, a blank line between tiers. Replaces the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracker_list: Option<String>,
    /// kB/s.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload_limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload_limited: Option<bool>,
}

impl TorrentSet {
    /// The same changes, minus any empty file list: Transmission reads an empty
    /// list as every file, which is never what a change from the UI means.
    pub fn without_empty_file_lists(mut self) -> Self {
        for list in [
            &mut self.files_wanted,
            &mut self.files_unwanted,
            &mut self.priority_high,
            &mut self.priority_low,
            &mut self.priority_normal,
        ] {
            if list.as_ref().is_some_and(Vec::is_empty) {
                *list = None;
            }
        }
        self
    }
}

/// Where a new torrent comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TorrentSource {
    /// A magnet link, or a URL the host can fetch a .torrent from.
    Link(String),
    /// The bytes of a .torrent file read on this machine.
    Metainfo(Vec<u8>),
}

/// `torrent_add`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddTorrent {
    pub source: TorrentSource,
    /// A folder on the host; the session's default when None.
    pub download_dir: Option<String>,
    pub paused: bool,
    pub labels: Vec<String>,
    pub bandwidth_priority: Option<Priority>,
    pub files_unwanted: Vec<i64>,
}

impl AddTorrent {
    pub fn new(source: TorrentSource) -> Self {
        Self {
            source,
            download_dir: None,
            paused: false,
            labels: Vec::new(),
            bandwidth_priority: None,
            files_unwanted: Vec::new(),
        }
    }
}

#[derive(Serialize)]
struct AddParams<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    filename: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metainfo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    download_dir: Option<&'a str>,
    paused: bool,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    labels: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    bandwidth_priority: Option<Priority>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    files_unwanted: &'a [i64],
}

/// The torrent `torrent_add` made, or the one already there.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AddedTorrent {
    pub id: i64,
    pub name: String,
    pub hash_string: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddOutcome {
    Added(AddedTorrent),
    /// Transmission already had it; nothing changed.
    Duplicate(AddedTorrent),
}

impl AddOutcome {
    pub fn torrent(&self) -> &AddedTorrent {
        match self {
            Self::Added(t) | Self::Duplicate(t) => t,
        }
    }
}

#[derive(Deserialize)]
struct AddResult {
    torrent_added: Option<AddedTorrent>,
    torrent_duplicate: Option<AddedTorrent>,
}

/// `torrent_rename_path`'s answer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct Renamed {
    pub id: i64,
    pub path: String,
    pub name: String,
}

/// Where `queue_move_*` moves torrents in the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueMove {
    Top,
    Up,
    Down,
    Bottom,
}

impl Client {
    /// Session settings: the given fields, or all of them when `fields` is empty.
    pub async fn session_get(&self, fields: &[SessionField]) -> Result<Session> {
        self.call("session_get", &SessionGetParams { fields }).await
    }

    pub async fn session_set(&self, set: &SessionSet) -> Result<()> {
        self.call::<_, IgnoredAny>("session_set", set).await.map(drop)
    }

    pub async fn session_stats(&self) -> Result<SessionStats> {
        self.call("session_stats", &NoParams {}).await
    }

    pub async fn free_space(&self, path: &str) -> Result<FreeSpace> {
        self.call("free_space", &FreeSpaceParams { path }).await
    }

    /// Ask Transmission's port-check service whether the peer port is reachable
    /// (over IPv4 when `ipv4` is set; whichever it connects with otherwise).
    pub async fn port_test(&self, ipv4: bool) -> Result<PortTest> {
        #[derive(Serialize)]
        struct Params {
            #[serde(skip_serializing_if = "Option::is_none")]
            ip_protocol: Option<&'static str>,
        }
        self.call("port_test", &Params { ip_protocol: ipv4.then_some("ipv4") }).await
    }

    /// Fetch the blocklist from its URL now. Returns how many rules it has.
    pub async fn blocklist_update(&self) -> Result<i64> {
        #[derive(Deserialize)]
        struct Answer {
            blocklist_size: i64,
        }
        let a: Answer = self.call("blocklist_update", &NoParams {}).await?;
        Ok(a.blocklist_size)
    }

    pub async fn torrent_get(&self, ids: &Ids, fields: &[TorrentField]) -> Result<TorrentGet> {
        self.call("torrent_get", &TorrentGetParams { ids, fields }).await
    }

    pub async fn torrent_add(&self, add: &AddTorrent) -> Result<AddOutcome> {
        let (filename, metainfo) = match &add.source {
            TorrentSource::Link(link) => (Some(link.as_str()), None),
            TorrentSource::Metainfo(bytes) => (None, Some(base64::engine::general_purpose::STANDARD.encode(bytes))),
        };
        let params = AddParams {
            filename,
            metainfo,
            download_dir: add.download_dir.as_deref(),
            paused: add.paused,
            labels: &add.labels,
            bandwidth_priority: add.bandwidth_priority,
            files_unwanted: &add.files_unwanted,
        };
        let result: AddResult = self.call("torrent_add", &params).await?;
        match (result.torrent_added, result.torrent_duplicate) {
            (Some(t), _) => Ok(AddOutcome::Added(t)),
            (None, Some(t)) => Ok(AddOutcome::Duplicate(t)),
            (None, None) => Err(crate::Error::Protocol("torrent_add returned no torrent".into())),
        }
    }

    pub async fn torrent_set(&self, ids: &Ids, set: &TorrentSet) -> Result<()> {
        self.call::<_, IgnoredAny>("torrent_set", &TorrentSetParams { ids, set }).await.map(drop)
    }

    pub async fn torrent_start(&self, ids: &Ids) -> Result<()> {
        self.ids_call("torrent_start", ids).await
    }

    /// Start, skipping the queue.
    pub async fn torrent_start_now(&self, ids: &Ids) -> Result<()> {
        self.ids_call("torrent_start_now", ids).await
    }

    pub async fn torrent_stop(&self, ids: &Ids) -> Result<()> {
        self.ids_call("torrent_stop", ids).await
    }

    pub async fn torrent_verify(&self, ids: &Ids) -> Result<()> {
        self.ids_call("torrent_verify", ids).await
    }

    /// Ask the trackers for more peers now.
    pub async fn torrent_reannounce(&self, ids: &Ids) -> Result<()> {
        self.ids_call("torrent_reannounce", ids).await
    }

    /// Remove torrents, and with `delete_local_data` their downloaded files on the host.
    pub async fn torrent_remove(&self, ids: &Ids, delete_local_data: bool) -> Result<()> {
        self.call::<_, IgnoredAny>("torrent_remove", &RemoveParams { ids, delete_local_data }).await.map(drop)
    }

    /// Point torrents at `location` on the host: moving their data there (`move_data`), or
    /// expecting it to be there already.
    pub async fn torrent_set_location(&self, ids: &Ids, location: &str, move_data: bool) -> Result<()> {
        let params = SetLocationParams { ids, location, move_data };
        self.call::<_, IgnoredAny>("torrent_set_location", &params).await.map(drop)
    }

    /// Rename a file or folder inside one torrent: `path` as in its `files` (the torrent's own
    /// name for its top folder or single file), `name` the new last part.
    pub async fn torrent_rename_path(&self, ids: &Ids, path: &str, name: &str) -> Result<Renamed> {
        self.call("torrent_rename_path", &RenameParams { ids, path, name }).await
    }

    pub async fn queue_move(&self, ids: &Ids, to: QueueMove) -> Result<()> {
        let method = match to {
            QueueMove::Top => "queue_move_top",
            QueueMove::Up => "queue_move_up",
            QueueMove::Down => "queue_move_down",
            QueueMove::Bottom => "queue_move_bottom",
        };
        self.ids_call(method, ids).await
    }

    async fn ids_call(&self, method: &str, ids: &Ids) -> Result<()> {
        self.call::<_, IgnoredAny>(method, &IdsParams { ids }).await.map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TorrentId;

    #[test]
    fn torrent_set_flattens_beside_ids() {
        let ids = Ids::hashes(["aa", "bb"]);
        let set = TorrentSet {
            labels: Some(vec!["linux".into()]),
            seed_ratio_mode: Some(LimitMode::Single),
            ..Default::default()
        };
        let json = serde_json::to_string(&TorrentSetParams { ids: &ids, set: &set }).unwrap();
        assert_eq!(json, r#"{"ids":["aa","bb"],"labels":["linux"],"seed_ratio_mode":1}"#);
    }

    #[test]
    fn all_ids_are_left_out() {
        let json = serde_json::to_string(&IdsParams { ids: &Ids::All }).unwrap();
        assert_eq!(json, "{}");
        let mixed = Ids::List(vec![TorrentId::Id(4), TorrentId::from("cafe")]);
        assert_eq!(serde_json::to_string(&IdsParams { ids: &mixed }).unwrap(), r#"{"ids":[4,"cafe"]}"#);
        assert_eq!(
            serde_json::to_string(&IdsParams { ids: &Ids::RecentlyActive }).unwrap(),
            r#"{"ids":"recently_active"}"#
        );
    }

    #[test]
    fn empty_file_lists_are_dropped() {
        let set = TorrentSet {
            files_wanted: Some(vec![]),
            files_unwanted: Some(vec![2]),
            priority_high: Some(vec![]),
            ..Default::default()
        }
        .without_empty_file_lists();
        assert_eq!(set.files_wanted, None, "empty would mean every file");
        assert_eq!(set.files_unwanted, Some(vec![2]));
        assert_eq!(set.priority_high, None);
    }

    #[test]
    fn torrent_set_reads_what_the_ui_sends() {
        let set: TorrentSet =
            serde_json::from_str(r#"{"seed_ratio_mode":1,"seed_ratio_limit":2.5,"bandwidth_priority":-1}"#).unwrap();
        assert_eq!(set.seed_ratio_mode, Some(LimitMode::Single));
        assert_eq!(set.bandwidth_priority, Some(Priority::Low));
        assert_eq!(set.labels, None);
    }

    #[test]
    fn set_location_says_move() {
        let ids = Ids::one(1);
        let json = serde_json::to_string(&SetLocationParams { ids: &ids, location: "/data", move_data: true }).unwrap();
        assert_eq!(json, r#"{"ids":[1],"location":"/data","move":true}"#);
    }
}
