//! The client against a real Transmission, in a throwaway container.
//! Run with `scripts/test-container.sh`, which starts the container, runs these
//! (they're ignored otherwise), and removes it. They add and remove their own
//! torrents, so never point them at a Transmission that matters.

use std::time::{SystemTime, UNIX_EPOCH};

use sha1::{Digest, Sha1};
use transmission::{
    AddOutcome, AddTorrent, Client, Config, Error, Ids, LimitMode, Priority, QueueMove, RpcCode, SessionField,
    SessionSet, Status, TorrentField, TorrentSet, TorrentSource,
};

fn config() -> Config {
    let host = std::env::var("XCVR_TEST_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("XCVR_TEST_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(19091);
    Config::new(host, port).with_credentials("xcvr", "xcvr")
}

fn client() -> Client {
    Client::new(config()).unwrap()
}

/// A name no other run has used.
fn unique(prefix: &str) -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    format!("{prefix}-{nanos:x}")
}

/// A one-file .torrent for `name` (bencoded by hand; keys in sorted order, as bencode requires).
fn tiny_torrent(name: &str) -> Vec<u8> {
    let content: Vec<u8> = name.bytes().cycle().take(1000).collect();
    let piece = Sha1::digest(&content);
    let mut info = Vec::new();
    info.extend_from_slice(
        format!("d6:lengthi{}e4:name{}:{name}12:piece lengthi16384e6:pieces20:", content.len(), name.len()).as_bytes(),
    );
    info.extend_from_slice(&piece);
    info.extend_from_slice(b"e");
    let mut torrent = b"d4:info".to_vec();
    torrent.extend_from_slice(&info);
    torrent.extend_from_slice(b"e");
    torrent
}

#[tokio::test]
#[ignore = "needs a Transmission container: scripts/test-container.sh"]
async fn session_speaks_4_1() {
    let s = client().session_get(&[]).await.unwrap();
    let version = s.version.unwrap();
    assert!(version.starts_with("4.1"), "{version}");
    assert!(s.rpc_version_semver.unwrap().starts_with("6."));
    assert!(s.download_dir.is_some_and(|d| d.starts_with('/')));
    assert!(s.units.is_some_and(|u| u.speed_bytes > 0));

    // Only what's asked for.
    let some = client().session_get(&[SessionField::Version]).await.unwrap();
    assert!(some.version.is_some());
    assert!(some.download_dir.is_none());
}

#[tokio::test]
#[ignore = "needs a Transmission container: scripts/test-container.sh"]
async fn session_set_round_trips() {
    let c = client();
    let before = c.session_get(&[SessionField::AltSpeedDown]).await.unwrap().alt_speed_down.unwrap();
    let wanted = before + 7;
    c.session_set(&SessionSet { alt_speed_down: Some(wanted), ..Default::default() }).await.unwrap();
    assert_eq!(c.session_get(&[SessionField::AltSpeedDown]).await.unwrap().alt_speed_down, Some(wanted));
    c.session_set(&SessionSet { alt_speed_down: Some(before), ..Default::default() }).await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Transmission container: scripts/test-container.sh"]
async fn stats_and_free_space() {
    let c = client();
    let stats = c.session_stats().await.unwrap();
    assert!(stats.torrent_count >= 0);
    assert!(stats.cumulative_stats.session_count >= 1);

    // The config folder always exists (the download folder may not until something finishes).
    let dir = c.session_get(&[SessionField::ConfigDir]).await.unwrap().config_dir.unwrap();
    let free = c.free_space(&dir).await.unwrap();
    assert_eq!(free.path, dir);
    assert!(free.size_bytes > 0 && free.total_size >= free.size_bytes);

    // A folder that isn't there is a system error with the OS's reason.
    match c.free_space("/no/such/folder").await.unwrap_err() {
        Error::Rpc { code: RpcCode::SystemError, detail, .. } => assert!(detail.is_some()),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
#[ignore = "needs a Transmission container: scripts/test-container.sh"]
async fn a_torrents_whole_life() {
    let c = client();
    let name = unique("xcvr-test");

    // Add, paused, with labels.
    let mut add = AddTorrent::new(TorrentSource::Metainfo(tiny_torrent(&name)));
    add.paused = true;
    add.labels = vec!["xcvr".into(), "test".into()];
    let added = match c.torrent_add(&add).await.unwrap() {
        AddOutcome::Added(t) => t,
        other => panic!("{other:?}"),
    };
    assert_eq!(added.name, name);
    let hash = added.hash_string.clone();
    let ids = Ids::hashes([hash.as_str()]);

    // Adding it again is a duplicate, not an error.
    assert!(matches!(c.torrent_add(&add).await.unwrap(), AddOutcome::Duplicate(t) if t.hash_string == hash));

    // Everything the table polls for decodes.
    let got = c.torrent_get(&ids, TorrentField::LIST).await.unwrap();
    let t = &got.torrents[0];
    assert_eq!(t.hash_string.as_deref(), Some(hash.as_str()));
    assert_eq!(t.name.as_deref(), Some(name.as_str()));
    assert_eq!(t.status, Some(Status::Stopped));
    assert_eq!(t.labels.as_deref(), Some(&["xcvr".to_owned(), "test".to_owned()][..]));
    assert_eq!(t.total_size, Some(1000));

    // So do the inspector's fields.
    let fields = [
        TorrentField::Files,
        TorrentField::FileStats,
        TorrentField::Wanted,
        TorrentField::Priorities,
        TorrentField::Peers,
        TorrentField::PeersFrom,
        TorrentField::Trackers,
        TorrentField::TrackerStats,
        TorrentField::MagnetLink,
        TorrentField::PieceCount,
        TorrentField::SeedRatioMode,
        TorrentField::BandwidthPriority,
    ];
    let detail = &c.torrent_get(&ids, &fields).await.unwrap().torrents[0];
    assert_eq!(detail.files.as_ref().unwrap()[0].name, name);
    assert_eq!(detail.wanted.as_deref(), Some(&[true][..]));
    assert_eq!(detail.piece_count, Some(1));
    assert!(detail.magnet_link.as_ref().unwrap().contains(&hash));

    // Labels: replaced as a whole; commas refused by Transmission too.
    c.torrent_set(&ids, &TorrentSet { labels: Some(vec!["only".into()]), ..Default::default() }).await.unwrap();
    let t = &c.torrent_get(&ids, &[TorrentField::Labels]).await.unwrap().torrents[0];
    assert_eq!(t.labels.as_deref(), Some(&["only".to_owned()][..]));
    let err =
        c.torrent_set(&ids, &TorrentSet { labels: Some(vec!["a,b".into()]), ..Default::default() }).await.unwrap_err();
    assert!(matches!(err, Error::Rpc { code: RpcCode::InvalidParams, .. }), "{err:?}");

    // Per-torrent options.
    let options = TorrentSet {
        seed_ratio_mode: Some(LimitMode::Single),
        seed_ratio_limit: Some(2.5),
        bandwidth_priority: Some(Priority::High),
        upload_limited: Some(true),
        upload_limit: Some(123),
        ..Default::default()
    };
    c.torrent_set(&ids, &options).await.unwrap();
    let fields = [
        TorrentField::SeedRatioMode,
        TorrentField::SeedRatioLimit,
        TorrentField::BandwidthPriority,
        TorrentField::UploadLimit,
    ];
    let t = &c.torrent_get(&ids, &fields).await.unwrap().torrents[0];
    assert_eq!(t.seed_ratio_mode, Some(LimitMode::Single));
    assert_eq!(t.seed_ratio_limit, Some(2.5));
    assert_eq!(t.bandwidth_priority, Some(Priority::High));
    assert_eq!(t.upload_limit, Some(123));

    // Rename (its single file is its top level).
    let renamed_to = format!("{name}-renamed");
    let renamed = c.torrent_rename_path(&ids, &name, &renamed_to).await.unwrap();
    assert_eq!(renamed.name, renamed_to);
    let t = &c.torrent_get(&ids, &[TorrentField::Name]).await.unwrap().torrents[0];
    assert_eq!(t.name.as_deref(), Some(renamed_to.as_str()));

    // Set location without moving anything.
    c.torrent_set_location(&ids, "/tmp", false).await.unwrap();
    let t = &c.torrent_get(&ids, &[TorrentField::DownloadDir]).await.unwrap().torrents[0];
    assert_eq!(t.download_dir.as_deref(), Some("/tmp"));

    // The actions are accepted.
    c.queue_move(&ids, QueueMove::Top).await.unwrap();
    c.queue_move(&ids, QueueMove::Bottom).await.unwrap();
    c.torrent_reannounce(&ids).await.unwrap();
    c.torrent_verify(&ids).await.unwrap();
    c.torrent_start_now(&ids).await.unwrap();
    c.torrent_stop(&ids).await.unwrap();

    // Remove, data too; then it's gone.
    c.torrent_remove(&ids, true).await.unwrap();
    assert!(c.torrent_get(&ids, &[TorrentField::Id]).await.unwrap().torrents.is_empty());
}

#[tokio::test]
#[ignore = "needs a Transmission container: scripts/test-container.sh"]
async fn magnet_without_metadata() {
    let c = client();
    // A random info hash: no peers will ever have it, so it stays at "fetching metadata".
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let hash: String = Sha1::digest(nanos.to_le_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    let mut add = AddTorrent::new(TorrentSource::Link(format!("magnet:?xt=urn:btih:{hash}&dn=xcvr-magnet-test")));
    add.paused = true;
    let added = c.torrent_add(&add).await.unwrap();
    assert_eq!(added.torrent().hash_string, hash);

    let ids = Ids::hashes([hash.as_str()]);
    let t = &c.torrent_get(&ids, TorrentField::LIST).await.unwrap().torrents[0];
    assert!(t.metadata_percent_complete.unwrap() < 1.0);
    c.torrent_remove(&ids, false).await.unwrap();
}

#[tokio::test]
#[ignore = "needs a Transmission container: scripts/test-container.sh"]
async fn wrong_password_is_unauthorized() {
    let mut cfg = config();
    cfg.password = Some("not-it".into());
    let err = Client::new(cfg).unwrap().session_stats().await.unwrap_err();
    assert!(matches!(err, Error::Unauthorized), "{err:?}");
}
