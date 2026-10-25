//! Torrents as `torrent_get` returns them (spec 3.3).

use serde::{Deserialize, Serialize};

/// What `torrent_get` returns.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TorrentGet {
    pub torrents: Vec<Torrent>,
    /// Ids removed recently; only when asking for `Ids::RecentlyActive`.
    #[serde(default)]
    pub removed: Vec<i64>,
}

/// A torrent, with only the fields that were asked for: everything else is None.
/// Sizes are in bytes, rates in bytes per second, dates in Unix seconds (0 for never).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Torrent {
    pub activity_date: Option<i64>,
    pub added_date: Option<i64>,
    /// Per piece: how many connected peers have it, or -1 when we already do.
    pub availability: Option<Vec<i64>>,
    pub bandwidth_priority: Option<Priority>,
    /// Per file: bytes done.
    pub bytes_completed: Option<Vec<i64>>,
    pub comment: Option<String>,
    pub corrupt_ever: Option<i64>,
    pub creator: Option<String>,
    pub date_created: Option<i64>,
    pub desired_available: Option<i64>,
    pub done_date: Option<i64>,
    pub download_dir: Option<String>,
    pub downloaded_ever: Option<i64>,
    /// kB/s.
    pub download_limit: Option<i64>,
    pub download_limited: Option<bool>,
    pub edit_date: Option<i64>,
    pub error: Option<TorrentError>,
    pub error_string: Option<String>,
    /// Seconds; -1 when it can't be estimated, -2 when it never will be.
    pub eta: Option<i64>,
    pub eta_idle: Option<i64>,
    pub file_count: Option<i64>,
    pub files: Option<Vec<File>>,
    pub file_stats: Option<Vec<FileStat>>,
    /// Its bandwidth group.
    pub group: Option<String>,
    pub hash_string: Option<String>,
    pub have_unchecked: Option<i64>,
    pub have_valid: Option<i64>,
    pub honors_session_limits: Option<bool>,
    pub id: Option<i64>,
    pub is_finished: Option<bool>,
    pub is_private: Option<bool>,
    pub is_stalled: Option<bool>,
    pub labels: Option<Vec<String>>,
    pub left_until_done: Option<i64>,
    pub magnet_link: Option<String>,
    pub max_connected_peers: Option<i64>,
    /// 0–1; below 1 while a magnet is still fetching its metadata.
    pub metadata_percent_complete: Option<f64>,
    pub name: Option<String>,
    pub peer_limit: Option<i64>,
    pub peers: Option<Vec<Peer>>,
    pub peers_connected: Option<i64>,
    pub peers_from: Option<PeersFrom>,
    pub peers_getting_from_us: Option<i64>,
    pub peers_sending_to_us: Option<i64>,
    /// 0–1, of the whole torrent.
    pub percent_complete: Option<f64>,
    /// 0–1, of the wanted files.
    pub percent_done: Option<f64>,
    /// Base64 bitfield of the pieces we have.
    pub pieces: Option<String>,
    pub piece_count: Option<i64>,
    pub piece_size: Option<i64>,
    /// Per file.
    pub priorities: Option<Vec<Priority>>,
    pub primary_mime_type: Option<String>,
    pub queue_position: Option<i64>,
    pub rate_download: Option<i64>,
    pub rate_upload: Option<i64>,
    pub recheck_progress: Option<f64>,
    pub seconds_downloading: Option<i64>,
    pub seconds_seeding: Option<i64>,
    /// Minutes.
    pub seed_idle_limit: Option<i64>,
    pub seed_idle_mode: Option<LimitMode>,
    pub seed_ratio_limit: Option<f64>,
    pub seed_ratio_mode: Option<LimitMode>,
    pub sequential_download: Option<bool>,
    pub sequential_download_from_piece: Option<i64>,
    pub size_when_done: Option<i64>,
    pub start_date: Option<i64>,
    pub status: Option<Status>,
    /// Where the host keeps its copy of the .torrent file.
    pub torrent_file: Option<String>,
    pub total_size: Option<i64>,
    pub trackers: Option<Vec<Tracker>>,
    /// Announce URLs one per line, a blank line between tiers.
    pub tracker_list: Option<String>,
    pub tracker_stats: Option<Vec<TrackerStat>>,
    pub uploaded_ever: Option<i64>,
    /// kB/s.
    pub upload_limit: Option<i64>,
    pub upload_limited: Option<bool>,
    /// -1 when there's nothing to compare against, -2 for infinite.
    pub upload_ratio: Option<f64>,
    /// Per file: whether it's downloaded.
    pub wanted: Option<Vec<bool>>,
    pub webseeds: Option<Vec<String>>,
    pub webseeds_sending_to_us: Option<i64>,
}

/// The keys `torrent_get` can be asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TorrentField {
    ActivityDate,
    AddedDate,
    Availability,
    BandwidthPriority,
    BytesCompleted,
    Comment,
    CorruptEver,
    Creator,
    DateCreated,
    DesiredAvailable,
    DoneDate,
    DownloadDir,
    DownloadedEver,
    DownloadLimit,
    DownloadLimited,
    EditDate,
    Error,
    ErrorString,
    Eta,
    EtaIdle,
    FileCount,
    Files,
    FileStats,
    Group,
    HashString,
    HaveUnchecked,
    HaveValid,
    HonorsSessionLimits,
    Id,
    IsFinished,
    IsPrivate,
    IsStalled,
    Labels,
    LeftUntilDone,
    MagnetLink,
    MaxConnectedPeers,
    MetadataPercentComplete,
    Name,
    PeerLimit,
    Peers,
    PeersConnected,
    PeersFrom,
    PeersGettingFromUs,
    PeersSendingToUs,
    PercentComplete,
    PercentDone,
    Pieces,
    PieceCount,
    PieceSize,
    Priorities,
    PrimaryMimeType,
    QueuePosition,
    RateDownload,
    RateUpload,
    RecheckProgress,
    SecondsDownloading,
    SecondsSeeding,
    SeedIdleLimit,
    SeedIdleMode,
    SeedRatioLimit,
    SeedRatioMode,
    SequentialDownload,
    SequentialDownloadFromPiece,
    SizeWhenDone,
    StartDate,
    Status,
    TorrentFile,
    TotalSize,
    Trackers,
    TrackerList,
    TrackerStats,
    UploadedEver,
    UploadLimit,
    UploadLimited,
    UploadRatio,
    Wanted,
    Webseeds,
    WebseedsSendingToUs,
}

impl TorrentField {
    /// What the torrent table needs on every poll (PLAN.md, "RPC notes").
    pub const LIST: &[TorrentField] = &[
        Self::Id,
        Self::HashString,
        Self::Name,
        Self::Status,
        Self::PercentDone,
        Self::MetadataPercentComplete,
        Self::RecheckProgress,
        Self::TotalSize,
        Self::SizeWhenDone,
        Self::LeftUntilDone,
        Self::RateDownload,
        Self::RateUpload,
        Self::Eta,
        Self::UploadRatio,
        Self::UploadedEver,
        Self::AddedDate,
        Self::DoneDate,
        Self::ActivityDate,
        Self::SecondsSeeding,
        Self::IsFinished,
        Self::IsStalled,
        Self::Error,
        Self::ErrorString,
        Self::QueuePosition,
        Self::Labels,
        Self::DownloadDir,
        Self::PeersConnected,
    ];
}

/// A file in a torrent; its index is its place in `files`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct File {
    /// Its path inside the torrent, `/`-separated, starting with the torrent's folder.
    pub name: String,
    pub length: i64,
    pub bytes_completed: i64,
    pub begin_piece: i64,
    pub end_piece: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FileStat {
    pub bytes_completed: i64,
    pub wanted: bool,
    pub priority: Priority,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Peer {
    pub address: String,
    pub bytes_to_client: i64,
    pub bytes_to_peer: i64,
    pub client_is_choked: bool,
    pub client_is_interested: bool,
    pub client_name: String,
    pub flag_str: String,
    pub is_downloading_from: bool,
    pub is_encrypted: bool,
    pub is_incoming: bool,
    pub is_uploading_to: bool,
    pub is_utp: bool,
    pub peer_id: String,
    pub peer_is_choked: bool,
    pub peer_is_interested: bool,
    pub port: i64,
    pub progress: f64,
    pub rate_to_client: i64,
    pub rate_to_peer: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PeersFrom {
    pub from_cache: i64,
    pub from_dht: i64,
    pub from_incoming: i64,
    pub from_lpd: i64,
    pub from_ltep: i64,
    pub from_pex: i64,
    pub from_tracker: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tracker {
    pub announce: String,
    pub id: i64,
    pub scrape: String,
    pub sitename: String,
    pub tier: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackerStat {
    pub announce: String,
    /// 0 inactive, 1 waiting, 2 queued, 3 active.
    pub announce_state: i64,
    pub download_count: i64,
    pub downloader_count: i64,
    pub has_announced: bool,
    pub has_scraped: bool,
    pub host: String,
    pub id: i64,
    pub is_backup: bool,
    pub last_announce_peer_count: i64,
    pub last_announce_result: String,
    pub last_announce_start_time: i64,
    pub last_announce_succeeded: bool,
    pub last_announce_time: i64,
    pub last_announce_timed_out: bool,
    pub last_scrape_result: String,
    pub last_scrape_start_time: i64,
    pub last_scrape_succeeded: bool,
    pub last_scrape_time: i64,
    pub last_scrape_timed_out: bool,
    pub leecher_count: i64,
    pub next_announce_time: i64,
    pub next_scrape_time: i64,
    pub scrape: String,
    pub scrape_state: i64,
    pub seeder_count: i64,
    pub sitename: String,
    pub tier: i64,
}

/// What a torrent is doing. Numbers outside 0–6 (a newer Transmission) come through as `Unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "i64", into = "i64")]
pub enum Status {
    Stopped,
    QueuedToVerify,
    Verifying,
    QueuedToDownload,
    Downloading,
    QueuedToSeed,
    Seeding,
    Unknown(i64),
}

impl From<i64> for Status {
    fn from(n: i64) -> Self {
        match n {
            0 => Self::Stopped,
            1 => Self::QueuedToVerify,
            2 => Self::Verifying,
            3 => Self::QueuedToDownload,
            4 => Self::Downloading,
            5 => Self::QueuedToSeed,
            6 => Self::Seeding,
            other => Self::Unknown(other),
        }
    }
}

impl From<Status> for i64 {
    fn from(s: Status) -> Self {
        match s {
            Status::Stopped => 0,
            Status::QueuedToVerify => 1,
            Status::Verifying => 2,
            Status::QueuedToDownload => 3,
            Status::Downloading => 4,
            Status::QueuedToSeed => 5,
            Status::Seeding => 6,
            Status::Unknown(n) => n,
        }
    }
}

/// `error` on a torrent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(from = "i64", into = "i64")]
pub enum TorrentError {
    #[default]
    None,
    TrackerWarning,
    TrackerError,
    /// A problem on the host: disk full, missing files, permissions.
    LocalError,
    Unknown(i64),
}

impl From<i64> for TorrentError {
    fn from(n: i64) -> Self {
        match n {
            0 => Self::None,
            1 => Self::TrackerWarning,
            2 => Self::TrackerError,
            3 => Self::LocalError,
            other => Self::Unknown(other),
        }
    }
}

impl From<TorrentError> for i64 {
    fn from(e: TorrentError) -> Self {
        match e {
            TorrentError::None => 0,
            TorrentError::TrackerWarning => 1,
            TorrentError::TrackerError => 2,
            TorrentError::LocalError => 3,
            TorrentError::Unknown(n) => n,
        }
    }
}

/// Bandwidth and file priority (`tr_priority_t`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(from = "i64", into = "i64")]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
}

impl From<i64> for Priority {
    fn from(n: i64) -> Self {
        match n {
            ..0 => Self::Low,
            0 => Self::Normal,
            _ => Self::High,
        }
    }
}

impl From<Priority> for i64 {
    fn from(p: Priority) -> Self {
        match p {
            Priority::Low => -1,
            Priority::Normal => 0,
            Priority::High => 1,
        }
    }
}

/// Which seeding limit a torrent follows (`tr_ratiolimit`, `tr_idlelimit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(from = "i64", into = "i64")]
pub enum LimitMode {
    /// The session's default.
    #[default]
    Global,
    /// This torrent's own limit.
    Single,
    Unlimited,
}

impl From<i64> for LimitMode {
    fn from(n: i64) -> Self {
        match n {
            1 => Self::Single,
            2 => Self::Unlimited,
            _ => Self::Global,
        }
    }
}

impl From<LimitMode> for i64 {
    fn from(m: LimitMode) -> Self {
        match m {
            LimitMode::Global => 0,
            LimitMode::Single => 1,
            LimitMode::Unlimited => 2,
        }
    }
}

/// Labels as Transmission accepts them.
pub mod label {
    /// Why a label was refused.
    #[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
    pub enum Invalid {
        #[error("A label can't be empty.")]
        Empty,
        #[error("A label can't contain a comma.")]
        Comma,
    }

    /// The label as Transmission will store it (trimmed), or why it would refuse it
    /// (libtransmission/rpcimpl.cc, `make_labels`).
    pub fn normalize(label: &str) -> Result<String, Invalid> {
        let label = label.trim();
        if label.is_empty() {
            Err(Invalid::Empty)
        } else if label.contains(',') {
            Err(Invalid::Comma)
        } else {
            Ok(label.to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_serialize_as_snake_case() {
        let json =
            serde_json::to_string(&[TorrentField::HashString, TorrentField::SequentialDownloadFromPiece]).unwrap();
        assert_eq!(json, r#"["hash_string","sequential_download_from_piece"]"#);
    }

    #[test]
    fn partial_torrent_decodes() {
        let t: Torrent = serde_json::from_str(
            r#"{"id":3,"hash_string":"abc","status":4,"error":0,"labels":["linux"],
                "eta":-1,"upload_ratio":-1,"wanted":[true,false],"file_stats":[{"bytes_completed":5,"wanted":true,"priority":1}]}"#,
        )
        .unwrap();
        assert_eq!(t.id, Some(3));
        assert_eq!(t.status, Some(Status::Downloading));
        assert_eq!(t.error, Some(TorrentError::None));
        assert_eq!(t.labels.as_deref(), Some(&["linux".to_owned()][..]));
        assert_eq!(t.eta, Some(-1));
        assert_eq!(t.file_stats.unwrap()[0].priority, Priority::High);
        assert_eq!(t.name, None);
    }

    #[test]
    fn unknown_status_survives() {
        let t: Torrent = serde_json::from_str(r#"{"status":9}"#).unwrap();
        assert_eq!(t.status, Some(Status::Unknown(9)));
        assert_eq!(serde_json::to_string(&Status::Unknown(9)).unwrap(), "9");
    }

    #[test]
    fn labels_follow_transmissions_rules() {
        assert_eq!(label::normalize("  linux isos "), Ok("linux isos".into()));
        assert_eq!(label::normalize("   "), Err(label::Invalid::Empty));
        assert_eq!(label::normalize("a,b"), Err(label::Invalid::Comma));
    }
}
