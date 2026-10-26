//! The session: settings (`session_get`, `session_set`), statistics and free space (spec 4).

use serde::{Deserialize, Serialize};

/// Session settings, with only the fields that were asked for. Speeds are kB/s,
/// times of day are minutes after midnight.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
#[serde(default)]
pub struct Session {
    pub alt_speed_down: Option<i64>,
    pub alt_speed_enabled: Option<bool>,
    pub alt_speed_time_begin: Option<i64>,
    /// Days as a bitmask (`tr_sched_day`): Sunday 1, Monday 2 … Saturday 64.
    pub alt_speed_time_day: Option<i64>,
    pub alt_speed_time_enabled: Option<bool>,
    pub alt_speed_time_end: Option<i64>,
    pub alt_speed_up: Option<i64>,
    pub anti_brute_force_enabled: Option<bool>,
    pub blocklist_enabled: Option<bool>,
    pub blocklist_size: Option<i64>,
    pub blocklist_url: Option<String>,
    pub cache_size_mib: Option<i64>,
    pub config_dir: Option<String>,
    pub default_trackers: Option<String>,
    pub dht_enabled: Option<bool>,
    pub download_dir: Option<String>,
    pub download_queue_enabled: Option<bool>,
    pub download_queue_size: Option<i64>,
    pub encryption: Option<Encryption>,
    /// Minutes.
    pub idle_seeding_limit: Option<i64>,
    pub idle_seeding_limit_enabled: Option<bool>,
    pub incomplete_dir: Option<String>,
    pub incomplete_dir_enabled: Option<bool>,
    pub lpd_enabled: Option<bool>,
    pub peer_limit_global: Option<i64>,
    pub peer_limit_per_torrent: Option<i64>,
    pub peer_port_random_on_start: Option<bool>,
    pub peer_port: Option<i64>,
    pub pex_enabled: Option<bool>,
    pub port_forwarding_enabled: Option<bool>,
    pub preferred_transports: Option<Vec<String>>,
    pub queue_stalled_enabled: Option<bool>,
    pub queue_stalled_minutes: Option<i64>,
    pub rename_partial_files: Option<bool>,
    pub reqq: Option<i64>,
    /// `6.0.1` on Transmission 4.1.1.
    pub rpc_version_semver: Option<String>,
    pub script_torrent_added_enabled: Option<bool>,
    pub script_torrent_added_filename: Option<String>,
    pub script_torrent_done_enabled: Option<bool>,
    pub script_torrent_done_filename: Option<String>,
    pub script_torrent_done_seeding_enabled: Option<bool>,
    pub script_torrent_done_seeding_filename: Option<String>,
    pub seed_queue_enabled: Option<bool>,
    pub seed_queue_size: Option<i64>,
    pub seed_ratio_limit: Option<f64>,
    pub seed_ratio_limited: Option<bool>,
    pub sequential_download: Option<bool>,
    pub session_id: Option<String>,
    pub speed_limit_down: Option<i64>,
    pub speed_limit_down_enabled: Option<bool>,
    pub speed_limit_up: Option<i64>,
    pub speed_limit_up_enabled: Option<bool>,
    pub start_added_torrents: Option<bool>,
    pub trash_original_torrent_files: Option<bool>,
    pub units: Option<Units>,
    /// `4.1.1 (abc1234)`.
    pub version: Option<String>,
}

/// The keys `session_get` can be asked for (leaving out the deprecated ones).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionField {
    AltSpeedDown,
    AltSpeedEnabled,
    AltSpeedTimeBegin,
    AltSpeedTimeDay,
    AltSpeedTimeEnabled,
    AltSpeedTimeEnd,
    AltSpeedUp,
    AntiBruteForceEnabled,
    BlocklistEnabled,
    BlocklistSize,
    BlocklistUrl,
    CacheSizeMib,
    ConfigDir,
    DefaultTrackers,
    DhtEnabled,
    DownloadDir,
    DownloadQueueEnabled,
    DownloadQueueSize,
    Encryption,
    IdleSeedingLimit,
    IdleSeedingLimitEnabled,
    IncompleteDir,
    IncompleteDirEnabled,
    LpdEnabled,
    PeerLimitGlobal,
    PeerLimitPerTorrent,
    PeerPortRandomOnStart,
    PeerPort,
    PexEnabled,
    PortForwardingEnabled,
    PreferredTransports,
    QueueStalledEnabled,
    QueueStalledMinutes,
    RenamePartialFiles,
    Reqq,
    RpcVersionSemver,
    ScriptTorrentAddedEnabled,
    ScriptTorrentAddedFilename,
    ScriptTorrentDoneEnabled,
    ScriptTorrentDoneFilename,
    ScriptTorrentDoneSeedingEnabled,
    ScriptTorrentDoneSeedingFilename,
    SeedQueueEnabled,
    SeedQueueSize,
    SeedRatioLimit,
    SeedRatioLimited,
    SequentialDownload,
    SessionId,
    SpeedLimitDown,
    SpeedLimitDownEnabled,
    SpeedLimitUp,
    SpeedLimitUpEnabled,
    StartAddedTorrents,
    TrashOriginalTorrentFiles,
    Units,
    Version,
}

/// Changes to the session's settings (`session_set`): only the fields that are Some are sent.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
#[serde(default)]
pub struct SessionSet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_speed_down: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_speed_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_speed_time_begin: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_speed_time_day: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_speed_time_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_speed_time_end: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_speed_up: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anti_brute_force_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocklist_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocklist_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_size_mib: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_trackers: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dht_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_queue_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_queue_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption: Option<Encryption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_seeding_limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_seeding_limit_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incomplete_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incomplete_dir_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lpd_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_limit_global: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_limit_per_torrent: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_port_random_on_start: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pex_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_forwarding_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_transports: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_stalled_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_stalled_minutes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rename_partial_files: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reqq: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_torrent_added_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_torrent_added_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_torrent_done_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_torrent_done_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_torrent_done_seeding_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_torrent_done_seeding_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_queue_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_queue_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_ratio_limit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_ratio_limited: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequential_download: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_limit_down: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_limit_down_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_limit_up: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_limit_up_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_added_torrents: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trash_original_torrent_files: Option<bool>,
}

/// Peer connection encryption. 4.1 renamed `tolerated` to `allowed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum Encryption {
    Required,
    Preferred,
    Allowed,
    /// Anything else a newer Transmission sends. Never sent back.
    #[serde(other)]
    Unknown,
}

/// How the host formats sizes and speeds (what a kB is).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct Units {
    pub speed_units: Vec<String>,
    pub speed_bytes: i64,
    pub size_units: Vec<String>,
    pub size_bytes: i64,
    pub memory_units: Vec<String>,
    pub memory_bytes: i64,
}

/// `session_stats`: what's happening now, and totals.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct SessionStats {
    pub active_torrent_count: i64,
    /// Bytes per second.
    pub download_speed: i64,
    pub paused_torrent_count: i64,
    pub torrent_count: i64,
    /// Bytes per second.
    pub upload_speed: i64,
    pub cumulative_stats: StatsTotals,
    pub current_stats: StatsTotals,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct StatsTotals {
    pub uploaded_bytes: i64,
    pub downloaded_bytes: i64,
    pub files_added: i64,
    pub seconds_active: i64,
    pub session_count: i64,
}

/// `free_space` for a folder on the host.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct FreeSpace {
    pub path: String,
    /// Free bytes.
    pub size_bytes: i64,
    /// The volume's capacity in bytes.
    pub total_size: i64,
}

/// `port_test`: whether the peer port can be reached from the internet (as
/// Transmission's port-check service sees it).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct PortTest {
    pub port_is_open: bool,
    /// `ipv4` or `ipv6`, when Transmission could tell.
    pub ip_protocol: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_sends_only_what_changed() {
        let set =
            SessionSet { alt_speed_enabled: Some(true), encryption: Some(Encryption::Preferred), ..Default::default() };
        assert_eq!(serde_json::to_string(&set).unwrap(), r#"{"alt_speed_enabled":true,"encryption":"preferred"}"#);
    }

    #[test]
    fn unknown_encryption_decodes() {
        let s: Session = serde_json::from_str(r#"{"encryption":"something_new","version":"4.1.1 (x)"}"#).unwrap();
        assert_eq!(s.encryption, Some(Encryption::Unknown));
        assert_eq!(s.version.as_deref(), Some("4.1.1 (x)"));
    }
}
