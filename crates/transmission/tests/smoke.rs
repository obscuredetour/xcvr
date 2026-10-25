//! A read-only check against a real Transmission (the Mac mini): it only
//! reads, so it's safe on a library that matters. Decodes every field of every
//! torrent, which catches real-world values the container tests can't.
//!
//!   XCVR_HOST=mac-mini.local XCVR_USER=… XCVR_PASS=… \
//!     cargo test -p transmission --test smoke -- --ignored --nocapture
//!
//! XCVR_PORT defaults to 9091.

use transmission::{Client, Config, Ids, SessionField, TorrentField};

fn client() -> Client {
    let host = std::env::var("XCVR_HOST").expect("set XCVR_HOST (and XCVR_USER, XCVR_PASS if the host needs them)");
    let port = std::env::var("XCVR_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(transmission::DEFAULT_PORT);
    let mut config = Config::new(host, port);
    if let Ok(user) = std::env::var("XCVR_USER") {
        config = config.with_credentials(user, std::env::var("XCVR_PASS").unwrap_or_default());
    }
    Client::new(config).unwrap()
}

/// Every torrent_get field, for the "decode everything" pass.
const EVERY_FIELD: &[TorrentField] = &[
    TorrentField::ActivityDate,
    TorrentField::AddedDate,
    TorrentField::BandwidthPriority,
    TorrentField::Comment,
    TorrentField::CorruptEver,
    TorrentField::Creator,
    TorrentField::DateCreated,
    TorrentField::DesiredAvailable,
    TorrentField::DoneDate,
    TorrentField::DownloadDir,
    TorrentField::DownloadedEver,
    TorrentField::DownloadLimit,
    TorrentField::DownloadLimited,
    TorrentField::EditDate,
    TorrentField::Error,
    TorrentField::ErrorString,
    TorrentField::Eta,
    TorrentField::EtaIdle,
    TorrentField::FileCount,
    TorrentField::Files,
    TorrentField::FileStats,
    TorrentField::Group,
    TorrentField::HashString,
    TorrentField::HaveUnchecked,
    TorrentField::HaveValid,
    TorrentField::HonorsSessionLimits,
    TorrentField::Id,
    TorrentField::IsFinished,
    TorrentField::IsPrivate,
    TorrentField::IsStalled,
    TorrentField::Labels,
    TorrentField::LeftUntilDone,
    TorrentField::MagnetLink,
    TorrentField::MaxConnectedPeers,
    TorrentField::MetadataPercentComplete,
    TorrentField::Name,
    TorrentField::PeerLimit,
    TorrentField::Peers,
    TorrentField::PeersConnected,
    TorrentField::PeersFrom,
    TorrentField::PeersGettingFromUs,
    TorrentField::PeersSendingToUs,
    TorrentField::PercentComplete,
    TorrentField::PercentDone,
    TorrentField::PieceCount,
    TorrentField::PieceSize,
    TorrentField::Priorities,
    TorrentField::PrimaryMimeType,
    TorrentField::QueuePosition,
    TorrentField::RateDownload,
    TorrentField::RateUpload,
    TorrentField::RecheckProgress,
    TorrentField::SecondsDownloading,
    TorrentField::SecondsSeeding,
    TorrentField::SeedIdleLimit,
    TorrentField::SeedIdleMode,
    TorrentField::SeedRatioLimit,
    TorrentField::SeedRatioMode,
    TorrentField::SequentialDownload,
    TorrentField::SequentialDownloadFromPiece,
    TorrentField::SizeWhenDone,
    TorrentField::StartDate,
    TorrentField::Status,
    TorrentField::TorrentFile,
    TorrentField::TotalSize,
    TorrentField::Trackers,
    TorrentField::TrackerList,
    TorrentField::TrackerStats,
    TorrentField::UploadedEver,
    TorrentField::UploadLimit,
    TorrentField::UploadLimited,
    TorrentField::UploadRatio,
    TorrentField::Wanted,
    TorrentField::Webseeds,
    TorrentField::WebseedsSendingToUs,
];

#[tokio::test]
#[ignore = "reads a real Transmission: set XCVR_HOST and run with --ignored"]
async fn read_only_smoke() {
    let c = client();

    let session = c.session_get(&[]).await.unwrap();
    println!(
        "Transmission {} (RPC {}), download dir {}",
        session.version.as_deref().unwrap_or("?"),
        session.rpc_version_semver.as_deref().unwrap_or("?"),
        session.download_dir.as_deref().unwrap_or("?"),
    );
    let some = c.session_get(&[SessionField::AltSpeedEnabled, SessionField::SpeedLimitDown]).await.unwrap();
    println!("Alt speed on: {:?}, download limit: {:?} kB/s", some.alt_speed_enabled, some.speed_limit_down);

    let stats = c.session_stats().await.unwrap();
    println!(
        "{} torrents ({} active), ↓ {} B/s ↑ {} B/s",
        stats.torrent_count, stats.active_torrent_count, stats.download_speed, stats.upload_speed
    );

    let list = c.torrent_get(&Ids::All, TorrentField::LIST).await.unwrap();
    assert_eq!(list.torrents.len() as i64, stats.torrent_count);
    for t in &list.torrents {
        assert!(t.hash_string.is_some() && t.status.is_some());
        println!(
            "  {:?} {:>5.1}% {} [{}]",
            t.status.unwrap(),
            t.percent_done.unwrap_or(0.0) * 100.0,
            t.name.as_deref().unwrap_or("?"),
            t.labels.as_deref().unwrap_or_default().join(", "),
        );
    }

    let everything = c.torrent_get(&Ids::All, EVERY_FIELD).await.unwrap();
    assert_eq!(everything.torrents.len(), list.torrents.len());
    let files: usize = everything.torrents.iter().map(|t| t.files.as_ref().map_or(0, Vec::len)).sum();
    println!("Every field decoded for every torrent ({files} files in all).");

    if let Some(dir) = session.download_dir {
        match c.free_space(&dir).await {
            Ok(free) => println!("Free in {dir}: {} of {} bytes", free.size_bytes, free.total_size),
            Err(e) => println!("free_space({dir}): {e}"),
        }
    }
}
