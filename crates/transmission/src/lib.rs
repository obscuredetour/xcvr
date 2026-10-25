//! A typed client for Transmission's RPC, in its JSON-RPC 2.0 form with
//! snake_case keys (Transmission 4.1 and later, `rpc_version_semver` 6.x).
//! The older bespoke protocol isn't supported.
//!
//! No Tauri here: the app wraps this crate in commands and a poller, and the
//! tests run it against a mock server and a real Transmission in a container.
//!
//! Spec: <https://github.com/transmission/transmission/blob/4.1.x/docs/rpc-spec.md>

mod client;
mod diff;
mod error;
mod ids;
mod metainfo;
mod methods;
mod session;
mod torrent;

pub use client::{Client, Config, DEFAULT_PATH, DEFAULT_PORT};
pub use diff::{Diff, Snapshot, finished};
pub use error::{Error, Result, RpcCode};
pub use ids::{Ids, TorrentId};
pub use metainfo::{BadMetainfo, Metainfo, read_metainfo};
pub use methods::{AddOutcome, AddTorrent, AddedTorrent, QueueMove, Renamed, TorrentSet, TorrentSource};
pub use session::{Encryption, FreeSpace, Session, SessionField, SessionSet, SessionStats, StatsTotals, Units};
pub use torrent::{
    File, FileStat, LimitMode, Peer, PeersFrom, Priority, Status, Torrent, TorrentError, TorrentField, TorrentGet,
    Tracker, TrackerStat, label,
};

/// The RPC protocol major version this crate speaks (`rpc_version_semver`).
pub const RPC_MAJOR: u64 = 6;
