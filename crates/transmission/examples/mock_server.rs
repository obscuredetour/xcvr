//! A stand-in Transmission for screenshots and UI work: it speaks the same
//! JSON-RPC (the 409 handshake included), holds a library of real, freely
//! shared torrents (Linux images, Blender's open movies, public-domain films,
//! open data, music released to share), and keeps them moving: downloads
//! advance and finish, speeds drift, the turtle and speed limits apply. Adding,
//! removing, pausing, labels, renames and moves all work, held in memory until
//! it's stopped.
//!
//!     cargo run -p transmission --example mock_server -- [--port 9092] [--bind 127.0.0.1] [--copies <dir>]
//!
//! Then connect the app to localhost, port 9092, with no username. `--bind
//! 0.0.0.0` lets another computer connect too.
//!
//! Original names: a few torrents are renamed, and the mock writes its copies
//! of their .torrent files (as Transmission keeps them) to `--copies` (a temp
//! folder by default). Map the folder it prints in Settings › Paths to show
//! them.

// json! for the session's sixty-odd settings nests deeper than the default allows.
#![recursion_limit = "256"]

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde_json::{Map, Value, json};
use sha1::{Digest, Sha1};
use transmission::{
    File, FileStat, LimitMode, Peer, PeersFrom, Priority, Status, Torrent, TorrentError, TorrentSet, Tracker,
    TrackerStat, read_metainfo,
};

const SESSION_ID: &str = "xcvr-mock-session";
const RPC_VERSION: &str = "6.0.1";
const HOME: &str = "/Users/alice";
/// Where the mock says the host keeps its .torrent copies (Transmission.app's folder).
const TORRENTS_DIR: &str = "/Users/alice/Library/Application Support/Transmission/Torrents";
const GB: f64 = 1e9;
const MB: f64 = 1e6;

fn main() {
    let mut port = 9092u16;
    let mut bind = "127.0.0.1".to_owned();
    let mut copies = std::env::temp_dir().join("xcvr-mock").join("Torrents");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| usage());
        match arg.as_str() {
            "--port" => port = value().parse().unwrap_or_else(|_| usage()),
            "--bind" => bind = value(),
            "--copies" => copies = PathBuf::from(value()),
            _ => usage(),
        }
    }

    let mock = Mock::new(&copies);
    let listener = TcpListener::bind((bind.as_str(), port)).unwrap_or_else(|e| {
        eprintln!("can't listen on {bind}:{port}: {e}");
        std::process::exit(1);
    });
    println!("Mock Transmission 4.1.3 on http://{bind}:{port}/transmission/rpc ({} torrents)", mock.torrents.len());
    println!("Connect to localhost (or this computer's address), port {port}, with no username.");
    println!("For original names, map in Settings › Paths:\n  {TORRENTS_DIR}\n  → {}", copies.display());

    let mock = Arc::new(Mutex::new(mock));
    for stream in listener.incoming().flatten() {
        let mock = Arc::clone(&mock);
        std::thread::spawn(move || serve(stream, &mock));
    }
}

fn usage() -> ! {
    eprintln!("usage: mock_server [--port 9092] [--bind 127.0.0.1] [--copies <dir>]");
    std::process::exit(2);
}

// ---------- HTTP ----------

/// One connection, kept alive for as many requests as the client sends.
fn serve(stream: TcpStream, mock: &Mutex<Mock>) {
    let Ok(read_half) = stream.try_clone() else { return };
    let mut reader = BufReader::new(read_half);
    let mut out = stream;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let mut headers = HashMap::new();
        loop {
            let mut h = String::new();
            if reader.read_line(&mut h).unwrap_or(0) == 0 {
                return;
            }
            let h = h.trim_end();
            if h.is_empty() {
                break;
            }
            if let Some((k, v)) = h.split_once(':') {
                headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_owned());
            }
        }
        let len = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
        let mut body = vec![0; len];
        if reader.read_exact(&mut body).is_err() {
            return;
        }

        let response = if headers.get("x-transmission-session-id").map(String::as_str) != Some(SESSION_ID) {
            let page = "<h1>409: Conflict</h1><p>Your request had an invalid session-id header.</p>";
            format!(
                "HTTP/1.1 409 Conflict\r\nX-Transmission-Session-Id: {SESSION_ID}\r\nX-Transmission-Rpc-Version: \
                 {RPC_VERSION}\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{page}",
                page.len()
            )
        } else {
            let reply = answer(&body, mock).to_string();
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{reply}",
                reply.len()
            )
        };
        if out.write_all(response.as_bytes()).is_err() {
            return;
        }
    }
}

/// A JSON-RPC request's answer.
fn answer(body: &[u8], mock: &Mutex<Mock>) -> Value {
    let Ok(request) = serde_json::from_slice::<Value>(body) else {
        return json!({ "jsonrpc": "2.0", "error": { "code": -32700, "message": "Parse error" }, "id": null });
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or_default();
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
    let mut mock = mock.lock().unwrap_or_else(|e| e.into_inner());
    mock.tick();
    match mock.call(method, &params) {
        Ok(result) => json!({ "jsonrpc": "2.0", "result": result, "id": id }),
        Err((code, message)) => json!({
            "jsonrpc": "2.0",
            "error": { "code": code, "message": message, "data": { "error_string": message } },
            "id": id,
        }),
    }
}

// ---------- The library ----------

#[derive(Clone, Copy, PartialEq)]
enum Start {
    Downloading,
    Seeding,
    Paused,
    Queued,
    Verifying,
    /// A magnet still fetching its metadata.
    Fetching,
}

struct Seed {
    name: &'static str,
    /// The name in its .torrent, when it's been renamed since.
    original: Option<&'static str>,
    /// Under ~/Downloads ("" for right in it), or a full path.
    folder: &'static str,
    labels: &'static [&'static str],
    /// Paths inside the torrent and their sizes; empty for a single file named `name`.
    files: &'static [(&'static str, f64)],
    /// Bytes, for a single-file torrent.
    size: f64,
    done: f64,
    start: Start,
    /// Bytes per second while moving.
    down: f64,
    up: f64,
    added_days: f64,
    tracker: &'static str,
    warning: Option<&'static str>,
    comment: &'static str,
}

const UBUNTU: &str = "https://torrent.ubuntu.com/announce";
const DEBIAN: &str = "http://bttracker.debian.org:6969/announce";
const FEDORA: &str = "http://torrent.fedoraproject.org:6969/announce";
const ARCHIVE: &str = "http://bt1.archive.org:6969/announce";
const OPEN: &str = "udp://tracker.opentrackr.org:1337/announce";

const SEEDS: &[Seed] = &[
    Seed {
        name: "Fedora-Workstation-Live-43-1.6.x86_64.iso",
        original: None,
        folder: "Linux",
        labels: &["linux", "iso"],
        files: &[],
        size: 2.31 * GB,
        done: 0.63,
        start: Start::Downloading,
        down: 4.2 * MB,
        up: 180e3,
        added_days: 0.02,
        tracker: FEDORA,
        warning: None,
        comment: "Fedora Workstation 43 Live",
    },
    Seed {
        name: "Tears of Steel",
        original: Some("tears_of_steel_1080p"),
        folder: "Films",
        labels: &["film", "open-movie"],
        files: &[
            ("Tears of Steel/tears_of_steel_1080p.mov", 1.12 * GB),
            ("Tears of Steel/subtitles/tears_of_steel_en.srt", 0.004 * MB),
            ("Tears of Steel/subtitles/tears_of_steel_de.srt", 0.004 * MB),
            ("Tears of Steel/poster.jpg", 2.1 * MB),
        ],
        size: 0.0,
        done: 0.87,
        start: Start::Downloading,
        down: 3.4 * MB,
        up: 95e3,
        added_days: 0.05,
        tracker: OPEN,
        warning: None,
        comment: "Blender Foundation | mango.blender.org",
    },
    Seed {
        name: "archlinux-2026.10.01-x86_64.iso",
        original: None,
        folder: "Linux",
        labels: &["linux", "iso"],
        files: &[],
        size: 1.42 * GB,
        done: 0.18,
        start: Start::Downloading,
        down: 2.1 * MB,
        up: 60e3,
        added_days: 0.01,
        tracker: OPEN,
        warning: None,
        comment: "Arch Linux 2026.10.01 (www.archlinux.org)",
    },
    Seed {
        name: "enwiki-20251001-pages-articles-multistream.xml.bz2",
        original: None,
        folder: "Data",
        labels: &["data"],
        files: &[],
        size: 25.7 * GB,
        done: 0.34,
        start: Start::Downloading,
        down: 0.9 * MB,
        up: 40e3,
        added_days: 2.0,
        tracker: OPEN,
        warning: None,
        comment: "Wikimedia dump",
    },
    Seed {
        name: "Night of the Living Dead (1968)",
        original: Some("night_of_the_living_dead"),
        folder: "Films",
        labels: &["film", "public-domain"],
        files: &[
            ("Night of the Living Dead (1968)/night_of_the_living_dead.mp4", 945.0 * MB),
            ("Night of the Living Dead (1968)/night_of_the_living_dead.gif", 0.4 * MB),
            ("Night of the Living Dead (1968)/night_of_the_living_dead_meta.xml", 0.01 * MB),
        ],
        size: 0.0,
        done: 0.12,
        start: Start::Downloading,
        down: 0.6 * MB,
        up: 12e3,
        added_days: 0.3,
        tracker: ARCHIVE,
        warning: Some("Tracker gave a warning: Could not connect to tracker"),
        comment: "This content hosted at the Internet Archive",
    },
    Seed {
        name: "Blender 4.5 LTS (macOS, Apple silicon)",
        original: None,
        folder: "Software",
        labels: &["software"],
        files: &[],
        size: 0.0,
        done: 0.0,
        start: Start::Fetching,
        down: 0.0,
        up: 0.0,
        added_days: 0.004,
        tracker: OPEN,
        warning: None,
        comment: "",
    },
    Seed {
        name: "kali-linux-2025.3-installer-amd64.iso",
        original: None,
        folder: "Linux",
        labels: &["linux", "iso"],
        files: &[],
        size: 4.41 * GB,
        done: 0.0,
        start: Start::Queued,
        down: 3.0 * MB,
        up: 0.0,
        added_days: 0.01,
        tracker: OPEN,
        warning: None,
        comment: "",
    },
    Seed {
        name: "linuxmint-22.2-cinnamon-64bit.iso",
        original: None,
        folder: "Linux",
        labels: &["linux", "iso"],
        files: &[],
        size: 2.94 * GB,
        done: 0.41,
        start: Start::Paused,
        down: 2.5 * MB,
        up: 0.0,
        added_days: 1.2,
        tracker: OPEN,
        warning: None,
        comment: "",
    },
    Seed {
        name: "planet-251006.osm.pbf",
        original: None,
        folder: "Data",
        labels: &["data", "maps"],
        files: &[],
        size: 84.6 * GB,
        done: 0.46,
        start: Start::Verifying,
        down: 0.0,
        up: 0.0,
        added_days: 6.0,
        tracker: OPEN,
        warning: None,
        comment: "OpenStreetMap data © OpenStreetMap contributors, ODbL",
    },
    Seed {
        name: "ubuntu-24.04.3-desktop-amd64.iso",
        original: None,
        folder: "Linux",
        labels: &["linux", "iso"],
        files: &[],
        size: 6.23 * GB,
        done: 1.0,
        start: Start::Seeding,
        down: 0.0,
        up: 310e3,
        added_days: 12.0,
        tracker: UBUNTU,
        warning: None,
        comment: "Ubuntu CD releases.ubuntu.com",
    },
    Seed {
        name: "debian-13.1.0-amd64-netinst.iso",
        original: None,
        folder: "Linux",
        labels: &["linux", "iso"],
        files: &[],
        size: 754.0 * MB,
        done: 1.0,
        start: Start::Seeding,
        down: 0.0,
        up: 120e3,
        added_days: 21.0,
        tracker: DEBIAN,
        warning: None,
        comment: "\"Debian CD from cdimage.debian.org\"",
    },
    Seed {
        name: "2025-10-01-raspios-trixie-arm64.img.xz",
        original: None,
        folder: "Linux",
        labels: &["linux"],
        files: &[],
        size: 1.21 * GB,
        done: 1.0,
        start: Start::Seeding,
        down: 0.0,
        up: 45e3,
        added_days: 18.0,
        tracker: OPEN,
        warning: None,
        comment: "",
    },
    Seed {
        name: "Big Buck Bunny",
        original: Some("bbb_sunflower_1080p_60fps_normal"),
        folder: "Films",
        labels: &["film", "open-movie"],
        files: &[],
        size: 355.9 * MB,
        done: 1.0,
        start: Start::Seeding,
        down: 0.0,
        up: 85e3,
        added_days: 40.0,
        tracker: OPEN,
        warning: None,
        comment: "Big Buck Bunny, Sunflower version. Blender Foundation | peach.blender.org",
    },
    Seed {
        name: "Sintel",
        original: Some("Sintel.2010.1080p"),
        folder: "Films",
        labels: &["film", "open-movie"],
        files: &[
            ("Sintel/Sintel.2010.1080p.mkv", 1.08 * GB),
            ("Sintel/Subs/sintel_en.srt", 0.002 * MB),
            ("Sintel/Subs/sintel_fr.srt", 0.002 * MB),
            ("Sintel/Subs/sintel_es.srt", 0.002 * MB),
            ("Sintel/poster.jpg", 1.4 * MB),
        ],
        size: 0.0,
        done: 1.0,
        start: Start::Seeding,
        down: 0.0,
        up: 140e3,
        added_days: 33.0,
        tracker: OPEN,
        warning: None,
        comment: "Durian Open Movie | durian.blender.org",
    },
    Seed {
        name: "Spring (2019)",
        original: None,
        folder: "/Volumes/Media/Films",
        labels: &["film", "open-movie"],
        files: &[("Spring (2019)/spring_1080p.mp4", 1.02 * GB), ("Spring (2019)/spring_poster.png", 3.2 * MB)],
        size: 0.0,
        done: 1.0,
        start: Start::Seeding,
        down: 0.0,
        up: 0.0,
        added_days: 9.0,
        tracker: OPEN,
        warning: None,
        comment: "Blender Studio",
    },
    Seed {
        name: "Nosferatu (1922)",
        original: Some("Nosferatu_1922"),
        folder: "Films",
        labels: &["film", "public-domain"],
        files: &[
            ("Nosferatu (1922)/Nosferatu_1922.mp4", 1.48 * GB),
            ("Nosferatu (1922)/Nosferatu_1922.ogv", 410.0 * MB),
        ],
        size: 0.0,
        done: 1.0,
        start: Start::Seeding,
        down: 0.0,
        up: 22e3,
        added_days: 55.0,
        tracker: ARCHIVE,
        warning: None,
        comment: "This content hosted at the Internet Archive",
    },
    Seed {
        name: "Nine Inch Nails - Ghosts I-IV",
        original: None,
        folder: "Music",
        labels: &["music"],
        files: &[
            ("Nine Inch Nails - Ghosts I-IV/01 1 Ghosts I.flac", 41.0 * MB),
            ("Nine Inch Nails - Ghosts I-IV/02 2 Ghosts I.flac", 28.0 * MB),
            ("Nine Inch Nails - Ghosts I-IV/03 3 Ghosts I.flac", 35.0 * MB),
            ("Nine Inch Nails - Ghosts I-IV/04 4 Ghosts I.flac", 43.0 * MB),
            ("Nine Inch Nails - Ghosts I-IV/05 5 Ghosts I.flac", 26.0 * MB),
            ("Nine Inch Nails - Ghosts I-IV/06 6 Ghosts I.flac", 39.0 * MB),
            ("Nine Inch Nails - Ghosts I-IV/cover.jpg", 2.4 * MB),
        ],
        size: 0.0,
        done: 1.0,
        start: Start::Seeding,
        down: 0.0,
        up: 64e3,
        added_days: 70.0,
        tracker: OPEN,
        warning: None,
        comment: "Released under a Creative Commons Attribution Non-Commercial Share Alike license",
    },
    Seed {
        name: "LibreOffice_25.8.2_MacOS_aarch64.dmg",
        original: None,
        folder: "",
        labels: &["software"],
        files: &[],
        size: 312.0 * MB,
        done: 1.0,
        start: Start::Paused,
        down: 0.0,
        up: 0.0,
        added_days: 4.0,
        tracker: OPEN,
        warning: None,
        comment: "",
    },
];

/// One torrent and what it's doing.
struct Sim {
    id: i64,
    hash: String,
    name: String,
    download_dir: String,
    labels: Vec<String>,
    /// Paths (starting with the torrent's name for multi-file ones) and sizes.
    files: Vec<(String, i64)>,
    wanted: Vec<bool>,
    priorities: Vec<Priority>,
    /// Bytes of the wanted files done.
    have: f64,
    status: Status,
    /// While verifying: progress, and what to go back to.
    verify: f64,
    after_verify: Status,
    metadata: f64,
    base_down: f64,
    base_up: f64,
    rate_down: f64,
    rate_up: f64,
    uploaded: f64,
    downloaded: f64,
    added: i64,
    done_date: i64,
    activity: i64,
    secs_down: f64,
    secs_seed: f64,
    warning: Option<String>,
    comment: String,
    created: i64,
    tracker: String,
    queue: i64,
    priority: Priority,
    download_limit: i64,
    download_limited: bool,
    upload_limit: i64,
    upload_limited: bool,
    honors_session_limits: bool,
    seed_ratio_limit: f64,
    seed_ratio_mode: LimitMode,
    seed_idle_limit: i64,
    seed_idle_mode: LimitMode,
    peer_limit: i64,
    sequential: bool,
    piece_size: i64,
}

impl Sim {
    fn total(&self) -> i64 {
        self.files.iter().map(|(_, n)| n).sum()
    }

    fn size_when_done(&self) -> i64 {
        self.files.iter().zip(&self.wanted).filter(|(_, w)| **w).map(|((_, n), _)| n).sum()
    }

    fn left(&self) -> i64 {
        (self.size_when_done() as f64 - self.have).max(0.0) as i64
    }

    fn fetching(&self) -> bool {
        self.metadata < 1.0
    }

    /// Bytes done per file, filling wanted files in order.
    fn per_file(&self) -> Vec<i64> {
        let mut left = self.have;
        self.files
            .iter()
            .zip(&self.wanted)
            .map(|((_, n), wanted)| {
                if !wanted {
                    return 0;
                }
                let got = left.min(*n as f64);
                left -= got;
                got as i64
            })
            .collect()
    }

    /// A stable pseudo-random number from this torrent's hash, for counts and addresses.
    fn byte(&self, i: usize) -> u8 {
        u8::from_str_radix(&self.hash[(i % 20) * 2..(i % 20) * 2 + 2], 16).unwrap_or(0)
    }

    fn resume(&mut self, now: i64) {
        if matches!(self.status, Status::Stopped | Status::QueuedToDownload | Status::QueuedToSeed) {
            self.status = if self.left() == 0 && !self.fetching() { Status::Seeding } else { Status::Downloading };
            self.activity = now;
        }
    }

    fn to_torrent(&self, now: i64) -> Torrent {
        let total = self.total();
        let size_when_done = self.size_when_done();
        let left = self.left();
        let per_file = self.per_file();
        let done = if size_when_done > 0 { self.have / size_when_done as f64 } else { 0.0 };
        let downloading = self.status == Status::Downloading && !self.fetching();
        let eta = if downloading && self.rate_down > 0.0 { (left as f64 / self.rate_down) as i64 } else { -1 };
        let pieces = ((total + self.piece_size - 1) / self.piece_size).max(1);
        let peers = self.peers();
        let connected = match self.status {
            Status::Downloading => 18 + i64::from(self.byte(3) % 40),
            Status::Seeding => 2 + i64::from(self.byte(4) % 12),
            _ => 0,
        };
        let sending = peers.iter().filter(|p| p.is_downloading_from).count() as i64;
        let getting = peers.iter().filter(|p| p.is_uploading_to).count() as i64;
        Torrent {
            activity_date: Some(self.activity),
            added_date: Some(self.added),
            bandwidth_priority: Some(self.priority),
            comment: Some(self.comment.clone()),
            corrupt_ever: Some(0),
            creator: Some("mktorrent 1.1".into()),
            date_created: Some(self.created),
            desired_available: Some(left),
            done_date: Some(self.done_date),
            download_dir: Some(self.download_dir.clone()),
            downloaded_ever: Some(self.downloaded as i64),
            download_limit: Some(self.download_limit),
            download_limited: Some(self.download_limited),
            edit_date: Some(0),
            error: Some(if self.warning.is_some() { TorrentError::TrackerWarning } else { TorrentError::None }),
            error_string: Some(self.warning.clone().unwrap_or_default()),
            eta: Some(eta),
            eta_idle: Some(-1),
            file_count: Some(self.files.len() as i64),
            files: Some(
                self.files
                    .iter()
                    .zip(&per_file)
                    .scan(0i64, |offset, ((name, length), got)| {
                        let begin = *offset / self.piece_size;
                        *offset += length;
                        Some(File {
                            name: name.clone(),
                            length: *length,
                            bytes_completed: *got,
                            begin_piece: begin,
                            end_piece: (*offset - 1).max(0) / self.piece_size + 1,
                        })
                    })
                    .collect(),
            ),
            file_stats: Some(
                per_file
                    .iter()
                    .zip(&self.wanted)
                    .zip(&self.priorities)
                    .map(|((got, wanted), priority)| FileStat {
                        bytes_completed: *got,
                        wanted: *wanted,
                        priority: *priority,
                    })
                    .collect(),
            ),
            bytes_completed: Some(per_file.clone()),
            group: Some(String::new()),
            hash_string: Some(self.hash.clone()),
            have_unchecked: Some(0),
            have_valid: Some(self.have as i64),
            honors_session_limits: Some(self.honors_session_limits),
            id: Some(self.id),
            // Done and stopped at its seed ratio: what Transmission calls finished.
            is_finished: Some(
                self.status == Status::Stopped
                    && left == 0
                    && self.downloaded > 0.0
                    && self.uploaded / self.downloaded >= self.seed_ratio_limit,
            ),
            is_private: Some(false),
            is_stalled: Some(downloading && self.rate_down < 1.0),
            labels: Some(self.labels.clone()),
            left_until_done: Some(left),
            magnet_link: Some(format!(
                "magnet:?xt=urn:btih:{}&dn={}&tr={}",
                self.hash,
                self.name.replace(' ', "+"),
                self.tracker
            )),
            max_connected_peers: Some(self.peer_limit),
            metadata_percent_complete: Some(self.metadata),
            name: Some(self.name.clone()),
            peer_limit: Some(self.peer_limit),
            peers: Some(peers),
            peers_connected: Some(connected),
            peers_from: Some(PeersFrom {
                from_cache: 0,
                from_dht: connected / 2,
                from_incoming: connected / 6,
                from_lpd: 0,
                from_ltep: 0,
                from_pex: connected / 5,
                from_tracker: connected - connected / 2 - connected / 6 - connected / 5,
            }),
            peers_getting_from_us: Some(getting),
            peers_sending_to_us: Some(sending),
            percent_complete: Some(if total > 0 { self.have / total as f64 } else { 0.0 }),
            percent_done: Some(done),
            pieces: Some(self.bitfield(pieces, if total > 0 { self.have / total as f64 } else { 0.0 })),
            piece_count: Some(pieces),
            piece_size: Some(self.piece_size),
            priorities: Some(self.priorities.clone()),
            primary_mime_type: Some("application/octet-stream".into()),
            queue_position: Some(self.queue),
            rate_download: Some(self.rate_down as i64),
            rate_upload: Some(self.rate_up as i64),
            recheck_progress: Some(self.verify),
            seconds_downloading: Some(self.secs_down as i64),
            seconds_seeding: Some(self.secs_seed as i64),
            seed_idle_limit: Some(self.seed_idle_limit),
            seed_idle_mode: Some(self.seed_idle_mode),
            seed_ratio_limit: Some(self.seed_ratio_limit),
            seed_ratio_mode: Some(self.seed_ratio_mode),
            sequential_download: Some(self.sequential),
            size_when_done: Some(size_when_done),
            start_date: Some(self.activity),
            status: Some(self.status),
            torrent_file: Some(format!("{TORRENTS_DIR}/{}.torrent", self.hash)),
            total_size: Some(total),
            trackers: Some(vec![Tracker {
                announce: self.tracker.clone(),
                id: 0,
                scrape: self.tracker.replace("announce", "scrape"),
                sitename: sitename(&self.tracker),
                tier: 0,
            }]),
            tracker_list: Some(self.tracker.clone()),
            tracker_stats: Some(vec![self.tracker_stat(now)]),
            uploaded_ever: Some(self.uploaded as i64),
            upload_limit: Some(self.upload_limit),
            upload_limited: Some(self.upload_limited),
            upload_ratio: Some(if self.downloaded > 0.0 { self.uploaded / self.downloaded } else { -1.0 }),
            wanted: Some(self.wanted.clone()),
            webseeds: Some(Vec::new()),
            webseeds_sending_to_us: Some(0),
            availability: None,
            sequential_download_from_piece: Some(0),
        }
    }

    /// The pieces it has, scattered the way rarest-first leaves them.
    fn bitfield(&self, pieces: i64, fraction: f64) -> String {
        let mut bits = vec![0u8; (pieces as usize).div_ceil(8)];
        let mut seed = u64::from(self.byte(0)) << 8 | u64::from(self.byte(1)) | 1;
        for i in 0..pieces as usize {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            if fraction >= 1.0 || (seed % 10_000) as f64 / 10_000.0 < fraction {
                bits[i / 8] |= 0x80 >> (i % 8);
            }
        }
        base64::engine::general_purpose::STANDARD.encode(bits)
    }

    fn peers(&self) -> Vec<Peer> {
        const CLIENTS: &[&str] = &[
            "qBittorrent 5.1.2",
            "Transmission 4.1.3",
            "Deluge 2.2.0",
            "libtorrent (Rasterbar) 2.0.11",
            "BiglyBT 3.8.0.2",
            "Transmission 4.0.6",
            "rTorrent 0.15.6",
            "qBittorrent 4.6.7",
        ];
        const NETS: &[&str] = &["192.0.2", "198.51.100", "203.0.113"];
        let moving = matches!(self.status, Status::Downloading | Status::Seeding) && !self.fetching();
        if !moving {
            return Vec::new();
        }
        let count = 4 + usize::from(self.byte(5) % 6);
        let senders = if self.status == Status::Downloading { count.div_ceil(2) } else { 0 };
        (0..count)
            .map(|i| {
                let b = self.byte(i * 3 + 7);
                let sending = i < senders;
                let getting = self.rate_up > 0.0 && (i % 3 == 1 || !sending && i % 2 == 0);
                let share = |rate: f64, n: usize| {
                    if n == 0 { 0 } else { (rate / n as f64 * (0.6 + f64::from(b % 80) / 100.0)) as i64 }
                };
                Peer {
                    address: format!("{}.{}", NETS[i % NETS.len()], 10 + u32::from(b) % 240),
                    client_name: CLIENTS[(usize::from(b) + i) % CLIENTS.len()].into(),
                    flag_str: match (sending, getting, i % 2 == 0) {
                        (true, true, _) => "DUEX",
                        (true, false, true) => "DEX",
                        (true, false, false) => "DIX",
                        (false, true, _) => "UEI",
                        _ => "?E",
                    }
                    .into(),
                    is_downloading_from: sending,
                    is_uploading_to: getting,
                    is_encrypted: true,
                    is_incoming: i % 2 == 1,
                    is_utp: i % 3 != 0,
                    port: 51413 + i64::from(b),
                    progress: if self.status == Status::Seeding || sending { 1.0 } else { f64::from(b % 90) / 100.0 },
                    rate_to_client: if sending { share(self.rate_down, senders) } else { 0 },
                    rate_to_peer: if getting { share(self.rate_up, count - senders) } else { 0 },
                    client_is_interested: sending,
                    peer_is_interested: getting,
                    ..Default::default()
                }
            })
            .collect()
    }

    fn tracker_stat(&self, now: i64) -> TrackerStat {
        let seeders = 40 + i64::from(self.byte(8)) * 7;
        let leechers = 3 + i64::from(self.byte(9) % 60);
        let failed = self.warning.is_some();
        TrackerStat {
            announce: self.tracker.clone(),
            announce_state: 1,
            download_count: seeders * 9,
            has_announced: true,
            has_scraped: !failed,
            host: host(&self.tracker),
            id: 0,
            last_announce_peer_count: if failed { 0 } else { 30.min(seeders + leechers) },
            last_announce_result: if failed { "Could not connect to tracker".into() } else { "Success".into() },
            last_announce_start_time: now - 300,
            last_announce_succeeded: !failed,
            last_announce_time: now - 300,
            last_announce_timed_out: failed,
            last_scrape_succeeded: !failed,
            last_scrape_time: now - 300,
            leecher_count: if failed { -1 } else { leechers },
            next_announce_time: now + 1500,
            next_scrape_time: now + 1500,
            scrape: self.tracker.replace("announce", "scrape"),
            seeder_count: if failed { -1 } else { seeders },
            sitename: sitename(&self.tracker),
            ..Default::default()
        }
    }
}

fn host(url: &str) -> String {
    url.split("://").nth(1).unwrap_or(url).split('/').next().unwrap_or_default().to_owned()
}

fn sitename(url: &str) -> String {
    let host = host(url);
    let name = host.split(':').next().unwrap_or_default();
    let parts: Vec<&str> = name.split('.').collect();
    parts.get(parts.len().saturating_sub(2)).copied().unwrap_or(name).to_owned()
}

fn hash_of(text: &str) -> String {
    Sha1::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or_default()
}

/// A minimal .torrent: enough for its name, size and file count to read back.
fn bencode_copy(name: &str, files: &[(String, i64)], piece_size: i64) -> Vec<u8> {
    fn s(out: &mut Vec<u8>, text: &str) {
        out.extend_from_slice(format!("{}:", text.len()).as_bytes());
        out.extend_from_slice(text.as_bytes());
    }
    let mut out = b"d8:announce".to_vec();
    s(&mut out, OPEN);
    out.extend_from_slice(b"4:infod");
    if files.len() == 1 && !files[0].0.contains('/') {
        out.extend_from_slice(format!("6:lengthi{}e", files[0].1).as_bytes());
    } else {
        out.extend_from_slice(b"5:filesl");
        for (path, length) in files {
            out.extend_from_slice(format!("d6:lengthi{length}e4:pathl").as_bytes());
            for part in path.split('/').skip(1) {
                s(&mut out, part);
            }
            out.extend_from_slice(b"ee");
        }
        out.push(b'e');
    }
    s(&mut out, "name");
    s(&mut out, name);
    out.extend_from_slice(format!("12:piece lengthi{piece_size}e6:pieces20:").as_bytes());
    out.extend_from_slice(&[0u8; 20]);
    out.extend_from_slice(b"ee");
    out
}

// ---------- The server's state ----------

struct Mock {
    torrents: Vec<Sim>,
    session: Map<String, Value>,
    next_id: i64,
    last_tick: Instant,
    started: Instant,
    copies: PathBuf,
    noise: u64,
    downloaded_total: f64,
    uploaded_total: f64,
}

impl Mock {
    fn new(copies: &Path) -> Self {
        let now = now();
        let _ = std::fs::create_dir_all(copies);
        let mut mock = Self {
            torrents: Vec::new(),
            session: initial_session(),
            next_id: 1,
            last_tick: Instant::now(),
            started: Instant::now(),
            copies: copies.to_owned(),
            noise: 0x2545_f491_4f6c_dd1d,
            downloaded_total: 1.84e12,
            uploaded_total: 3.07e12,
        };
        for seed in SEEDS {
            let files: Vec<(String, i64)> = if seed.files.is_empty() {
                vec![(seed.name.to_owned(), seed.size as i64)]
            } else {
                seed.files.iter().map(|(p, n)| ((*p).to_owned(), *n as i64)).collect()
            };
            let (status, metadata) = match seed.start {
                Start::Downloading => (Status::Downloading, 1.0),
                Start::Seeding => (Status::Seeding, 1.0),
                Start::Paused => (Status::Stopped, 1.0),
                Start::Queued => (Status::QueuedToDownload, 1.0),
                Start::Verifying => (Status::Verifying, 1.0),
                Start::Fetching => (Status::Downloading, 0.37),
            };
            let added = now - (seed.added_days * 86_400.0) as i64;
            let id = mock.add_sim(
                seed.name,
                seed.original.unwrap_or(seed.name),
                files,
                &match seed.folder {
                    "" => format!("{HOME}/Downloads"),
                    f if f.starts_with('/') => f.to_owned(),
                    f => format!("{HOME}/Downloads/{f}"),
                },
            );
            let sim = mock.torrents.last_mut().expect("just added");
            debug_assert_eq!(sim.id, id);
            sim.labels = seed.labels.iter().map(|l| (*l).to_owned()).collect();
            sim.have = sim.size_when_done() as f64 * seed.done;
            sim.status = status;
            sim.metadata = metadata;
            if status == Status::Verifying {
                sim.verify = 0.46;
                sim.after_verify = Status::Downloading;
            }
            sim.base_down = seed.down;
            sim.base_up = seed.up;
            sim.added = added;
            sim.activity = if matches!(status, Status::Stopped) { added + 3600 } else { now - 2 };
            sim.downloaded = sim.have;
            let age = seed.added_days * 86_400.0;
            if seed.done >= 1.0 {
                sim.done_date = added + 1800 + i64::from(sim.byte(2)) * 20;
                sim.secs_down = 1800.0;
                sim.secs_seed = age - 1800.0;
                sim.uploaded = sim.have * (0.4 + f64::from(sim.byte(6)) / 40.0);
            } else {
                sim.secs_down = age.min(86_400.0);
                sim.uploaded = sim.have * 0.15;
            }
            sim.warning = seed.warning.map(str::to_owned);
            sim.comment = seed.comment.to_owned();
            sim.created = added - 86_400 * (3 + i64::from(sim.byte(10) % 60));
            sim.tracker = seed.tracker.to_owned();
        }
        mock.requeue();
        mock
    }

    /// A new torrent, its .torrent copy written. Returns its id.
    fn add_sim(&mut self, name: &str, original: &str, files: Vec<(String, i64)>, dir: &str) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        let hash = hash_of(original);
        let total: i64 = files.iter().map(|(_, n)| n).sum();
        let piece_size = match total {
            t if t > 8_000_000_000 => 16 << 20,
            t if t > 1_000_000_000 => 4 << 20,
            t if t > 100_000_000 => 1 << 20,
            _ => 256 << 10,
        };
        // The copy holds the original name, with the original folder at the start of each path.
        let original_files: Vec<(String, i64)> = files
            .iter()
            .map(|(p, n)| {
                (p.split_once('/').map_or_else(|| original.to_owned(), |(_, rest)| format!("{original}/{rest}")), *n)
            })
            .collect();
        let _ = std::fs::write(
            self.copies.join(format!("{hash}.torrent")),
            bencode_copy(original, &original_files, piece_size),
        );
        let count = files.len();
        self.torrents.push(Sim {
            id,
            hash,
            name: name.to_owned(),
            download_dir: dir.to_owned(),
            labels: Vec::new(),
            files,
            wanted: vec![true; count],
            priorities: vec![Priority::Normal; count],
            have: 0.0,
            status: Status::Downloading,
            verify: 0.0,
            after_verify: Status::Stopped,
            metadata: 1.0,
            base_down: 2.0 * MB,
            base_up: 30e3,
            rate_down: 0.0,
            rate_up: 0.0,
            uploaded: 0.0,
            downloaded: 0.0,
            added: now(),
            done_date: 0,
            activity: now(),
            secs_down: 0.0,
            secs_seed: 0.0,
            warning: None,
            comment: String::new(),
            created: now() - 86_400,
            tracker: OPEN.to_owned(),
            queue: 0,
            priority: Priority::Normal,
            download_limit: 100,
            download_limited: false,
            upload_limit: 100,
            upload_limited: false,
            honors_session_limits: true,
            seed_ratio_limit: 2.0,
            seed_ratio_mode: LimitMode::Global,
            seed_idle_limit: 30,
            seed_idle_mode: LimitMode::Global,
            peer_limit: 50,
            sequential: false,
            piece_size,
        });
        id
    }

    /// Queue positions in order of adding, as Transmission numbers them.
    fn requeue(&mut self) {
        let mut order: Vec<usize> = (0..self.torrents.len()).collect();
        order.sort_by_key(|&i| (self.torrents[i].queue, self.torrents[i].added));
        for (pos, i) in order.into_iter().enumerate() {
            self.torrents[i].queue = pos as i64;
        }
    }

    fn rand(&mut self) -> f64 {
        self.noise ^= self.noise << 13;
        self.noise ^= self.noise >> 7;
        self.noise ^= self.noise << 17;
        (self.noise % 10_000) as f64 / 10_000.0
    }

    fn flag(&self, key: &str) -> bool {
        self.session.get(key).and_then(Value::as_bool).unwrap_or(false)
    }

    fn number(&self, key: &str) -> f64 {
        self.session.get(key).and_then(Value::as_f64).unwrap_or(0.0)
    }

    /// Move time on: speeds drift, downloads advance and finish, checks progress.
    fn tick(&mut self) {
        let dt = self.last_tick.elapsed().as_secs_f64().min(5.0);
        self.last_tick = Instant::now();
        let t = self.started.elapsed().as_secs_f64();
        let now = now();

        // Each torrent's own speed first...
        for i in 0..self.torrents.len() {
            let r1 = self.rand();
            let r2 = self.rand();
            let sim = &mut self.torrents[i];
            let wave = |phase: f64| 0.82 + 0.18 * (t * 0.31 + phase).sin();
            let (mut down, mut up) = match sim.status {
                Status::Downloading if !sim.fetching() => (
                    sim.base_down * wave(sim.id as f64) * (0.9 + 0.2 * r1),
                    sim.base_up * wave(sim.id as f64 * 2.1) * (0.8 + 0.4 * r2),
                ),
                Status::Seeding => (0.0, sim.base_up * wave(sim.id as f64 * 1.7) * (0.7 + 0.6 * r2)),
                _ => (0.0, 0.0),
            };
            if sim.download_limited {
                down = down.min(sim.download_limit as f64 * 1000.0);
            }
            if sim.upload_limited {
                up = up.min(sim.upload_limit as f64 * 1000.0);
            }
            sim.rate_down = down;
            sim.rate_up = up;
        }
        // ...then the session's limits (the turtle's when it's on) over all of them.
        let turtle = self.flag("alt_speed_enabled");
        let (down_cap, up_cap) = if turtle {
            (Some(self.number("alt_speed_down") * 1000.0), Some(self.number("alt_speed_up") * 1000.0))
        } else {
            (
                self.flag("speed_limit_down_enabled").then(|| self.number("speed_limit_down") * 1000.0),
                self.flag("speed_limit_up_enabled").then(|| self.number("speed_limit_up") * 1000.0),
            )
        };
        let total_down: f64 = self.torrents.iter().map(|s| s.rate_down).sum();
        let total_up: f64 = self.torrents.iter().map(|s| s.rate_up).sum();
        let down_scale = down_cap.map_or(1.0, |c| if total_down > c { c / total_down } else { 1.0 });
        let up_scale = up_cap.map_or(1.0, |c| if total_up > c { c / total_up } else { 1.0 });

        for sim in &mut self.torrents {
            if sim.honors_session_limits {
                sim.rate_down *= down_scale;
                sim.rate_up *= up_scale;
            }
            self.downloaded_total += sim.rate_down * dt;
            self.uploaded_total += sim.rate_up * dt;
            sim.uploaded += sim.rate_up * dt;
            match sim.status {
                Status::Downloading if sim.fetching() => {
                    sim.metadata = (sim.metadata + dt * 0.004).min(0.94);
                }
                Status::Downloading => {
                    let size = sim.size_when_done() as f64;
                    sim.have = (sim.have + sim.rate_down * dt).min(size);
                    sim.downloaded += sim.rate_down * dt;
                    sim.secs_down += dt;
                    sim.activity = now;
                    if sim.have >= size {
                        sim.status = Status::Seeding;
                        sim.done_date = now;
                        sim.rate_down = 0.0;
                    }
                }
                Status::Seeding => {
                    sim.secs_seed += dt;
                    if sim.rate_up > 0.0 {
                        sim.activity = now;
                    }
                }
                Status::Verifying => {
                    sim.verify += dt * 0.012;
                    if sim.verify >= 1.0 {
                        sim.verify = 0.0;
                        sim.status = sim.after_verify;
                    }
                }
                _ => {}
            }
        }
        // A finished download lets the next queued one start.
        let active = self.torrents.iter().filter(|s| s.status == Status::Downloading).count();
        let queue_size = self.number("download_queue_size") as usize;
        if self.flag("download_queue_enabled") && active < queue_size {
            let mut queued: Vec<&mut Sim> =
                self.torrents.iter_mut().filter(|s| s.status == Status::QueuedToDownload).collect();
            queued.sort_by_key(|s| s.queue);
            for sim in queued.into_iter().take(queue_size - active) {
                sim.status = Status::Downloading;
            }
        }
    }

    /// Indices of the torrents `ids` names: all when it's absent or "recently_active".
    fn select(&self, params: &Value) -> Vec<usize> {
        let matches = |sim: &Sim, id: &Value| match id {
            Value::Number(n) => n.as_i64() == Some(sim.id),
            Value::String(h) => h.eq_ignore_ascii_case(&sim.hash),
            _ => false,
        };
        let all = 0..self.torrents.len();
        match params.get("ids") {
            None => all.collect(),
            Some(Value::String(s)) if s == "recently_active" => all.collect(),
            Some(Value::Array(ids)) => all.filter(|&i| ids.iter().any(|id| matches(&self.torrents[i], id))).collect(),
            Some(one) => all.filter(|&i| matches(&self.torrents[i], one)).collect(),
        }
    }

    fn call(&mut self, method: &str, params: &Value) -> Result<Value, (i64, String)> {
        let now = now();
        match method {
            "session_get" => Ok(Value::Object(only(&self.session, params.get("fields")))),
            "session_set" => {
                if let Value::Object(set) = params {
                    for (k, v) in set {
                        self.session.insert(k.clone(), v.clone());
                    }
                }
                Ok(json!({}))
            }
            "session_stats" => {
                let active = self.torrents.iter().filter(|s| s.rate_down > 0.0 || s.rate_up > 0.0).count();
                let paused = self.torrents.iter().filter(|s| s.status == Status::Stopped).count();
                let session_secs = self.started.elapsed().as_secs() as i64;
                Ok(json!({
                    "active_torrent_count": active,
                    "paused_torrent_count": paused,
                    "torrent_count": self.torrents.len(),
                    "download_speed": self.torrents.iter().map(|s| s.rate_down).sum::<f64>() as i64,
                    "upload_speed": self.torrents.iter().map(|s| s.rate_up).sum::<f64>() as i64,
                    "cumulative_stats": {
                        "uploaded_bytes": self.uploaded_total as i64,
                        "downloaded_bytes": self.downloaded_total as i64,
                        "files_added": 412,
                        "seconds_active": 38_200_000 + session_secs,
                        "session_count": 57,
                    },
                    "current_stats": {
                        "uploaded_bytes": 2_100_000_000i64,
                        "downloaded_bytes": 6_400_000_000i64,
                        "files_added": self.torrents.len(),
                        "seconds_active": 86_400 * 3 + session_secs,
                        "session_count": 1,
                    },
                }))
            }
            "free_space" => {
                let path = params.get("path").and_then(Value::as_str).unwrap_or_default();
                // Only the host's home folder "exists", so a typo shows the app's "not on the host yet".
                if !(path == HOME || path.starts_with(&format!("{HOME}/"))) {
                    return Err((5, format!("{path}: No such file or directory")));
                }
                Ok(json!({ "path": path, "size_bytes": 1_342_000_000_000i64, "total_size": 4_000_787_030_016i64 }))
            }
            "port_test" => Ok(json!({ "port_is_open": true, "ip_protocol": "ipv4" })),
            "blocklist_update" => {
                self.session.insert("blocklist_size".into(), json!(412_871));
                Ok(json!({ "blocklist_size": 412_871 }))
            }
            "torrent_get" => {
                let fields = params.get("fields");
                let torrents: Vec<Value> = self
                    .select(params)
                    .into_iter()
                    .map(|i| {
                        let t = serde_json::to_value(self.torrents[i].to_torrent(now)).unwrap_or_default();
                        Value::Object(only(t.as_object().unwrap_or(&Map::new()), fields))
                    })
                    .collect();
                let mut result = json!({ "torrents": torrents });
                if params.get("ids").and_then(Value::as_str) == Some("recently_active") {
                    result["removed"] = json!([]);
                }
                Ok(result)
            }
            "torrent_add" => self.add(params),
            "torrent_set" => {
                let set: TorrentSet = serde_json::from_value(params.clone()).map_err(|e| (-32602, e.to_string()))?;
                if set.labels.iter().flatten().any(|l| l.contains(',') || l.trim().is_empty()) {
                    return Err((-32602, "Invalid argument: labels can't contain commas or be empty".into()));
                }
                for i in self.select(params) {
                    apply(&mut self.torrents[i], &set);
                }
                self.requeue();
                Ok(json!({}))
            }
            "torrent_start" | "torrent_start_now" => {
                for i in self.select(params) {
                    self.torrents[i].resume(now);
                }
                Ok(json!({}))
            }
            "torrent_stop" => {
                for i in self.select(params) {
                    let sim = &mut self.torrents[i];
                    sim.status = Status::Stopped;
                    sim.rate_down = 0.0;
                    sim.rate_up = 0.0;
                }
                Ok(json!({}))
            }
            "torrent_verify" => {
                for i in self.select(params) {
                    let sim = &mut self.torrents[i];
                    if sim.status != Status::Verifying {
                        sim.after_verify = sim.status;
                        sim.status = Status::Verifying;
                        sim.verify = 0.0;
                    }
                }
                Ok(json!({}))
            }
            "torrent_reannounce" => Ok(json!({})),
            "torrent_remove" => {
                let gone = self.select(params);
                let mut i = 0;
                self.torrents.retain(|_| {
                    i += 1;
                    !gone.contains(&(i - 1))
                });
                self.requeue();
                Ok(json!({}))
            }
            "torrent_set_location" => {
                let location = params.get("location").and_then(Value::as_str).unwrap_or_default().to_owned();
                for i in self.select(params) {
                    self.torrents[i].download_dir = location.clone();
                }
                Ok(json!({}))
            }
            "torrent_rename_path" => {
                let path = params.get("path").and_then(Value::as_str).unwrap_or_default().to_owned();
                let name = params.get("name").and_then(Value::as_str).unwrap_or_default().to_owned();
                if name.is_empty() || name.contains('/') {
                    return Err((-32602, "Invalid argument".into()));
                }
                let Some(&i) = self.select(params).first() else { return Err((-32602, "No such torrent".into())) };
                let sim = &mut self.torrents[i];
                let renamed = match path.rsplit_once('/') {
                    Some((parent, _)) => format!("{parent}/{name}"),
                    None => name.clone(),
                };
                for (file, _) in &mut sim.files {
                    if *file == path {
                        file.clone_from(&renamed);
                    } else if let Some(rest) = file.strip_prefix(&format!("{path}/")) {
                        *file = format!("{renamed}/{rest}");
                    }
                }
                if path == sim.name {
                    sim.name = name.clone();
                }
                Ok(json!({ "id": sim.id, "path": path, "name": name }))
            }
            "queue_move_top" | "queue_move_up" | "queue_move_down" | "queue_move_bottom" => {
                let delta = match method {
                    "queue_move_top" => -1_000_000.5,
                    "queue_move_up" => -1.5,
                    "queue_move_down" => 1.5,
                    _ => 1_000_000.5,
                };
                let chosen = self.select(params);
                let mut order: Vec<(f64, usize)> = (0..self.torrents.len())
                    .map(|i| (self.torrents[i].queue as f64 + if chosen.contains(&i) { delta } else { 0.0 }, i))
                    .collect();
                order.sort_by(|a, b| a.0.total_cmp(&b.0));
                for (pos, (_, i)) in order.into_iter().enumerate() {
                    self.torrents[i].queue = pos as i64;
                }
                Ok(json!({}))
            }
            _ => Err((-32601, format!("Method not found: {method}"))),
        }
    }

    fn add(&mut self, params: &Value) -> Result<Value, (i64, String)> {
        let dir = params.get("download_dir").and_then(Value::as_str).map_or_else(
            || self.session.get("download_dir").and_then(Value::as_str).unwrap_or(HOME).to_owned(),
            str::to_owned,
        );
        let paused = params.get("paused").and_then(Value::as_bool).unwrap_or(false);
        let labels: Vec<String> = params
            .get("labels")
            .and_then(Value::as_array)
            .map(|l| l.iter().filter_map(Value::as_str).map(str::to_owned).collect())
            .unwrap_or_default();

        let (name, hash_source, files, fetching) = if let Some(meta) = params.get("metainfo").and_then(Value::as_str) {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(meta)
                .map_err(|_| (-32602, "Invalid metainfo".to_owned()))?;
            let info = read_metainfo(&bytes).map_err(|e| (-32602, format!("Unrecognized info: {e}")))?;
            let count = info.file_count.max(1);
            let files = if count == 1 {
                vec![(info.name.clone(), info.total_size)]
            } else {
                (1..=count).map(|i| (format!("{}/part {i:02}", info.name), info.total_size / count)).collect()
            };
            (info.name, hash_of(&String::from_utf8_lossy(&bytes)), files, false)
        } else if let Some(link) = params.get("filename").and_then(Value::as_str) {
            let field = |key: &str| {
                link.split(['?', '&'])
                    .find_map(|p| p.strip_prefix(key))
                    .map(|v| v.replace('+', " ").replace("%20", " "))
            };
            let hash = field("xt=urn:btih:").unwrap_or_else(|| hash_of(link)).to_ascii_lowercase();
            let name = field("dn=").unwrap_or_else(|| hash.clone());
            (name, hash, vec![(String::new(), 0)], true)
        } else {
            return Err((-32602, "No filename or metainfo".into()));
        };

        if let Some(t) = self.torrents.iter().find(|s| s.hash == hash_source || s.name == name) {
            return Ok(json!({ "torrent_duplicate": { "id": t.id, "name": t.name, "hash_string": t.hash } }));
        }
        let files = if fetching { vec![(name.clone(), 0)] } else { files };
        let id = self.add_sim(&name, &name, files, &dir);
        let sim = self.torrents.last_mut().expect("just added");
        if fetching {
            sim.hash = hash_source;
            sim.metadata = 0.0;
        }
        sim.labels = labels;
        sim.status = if paused { Status::Stopped } else { Status::Downloading };
        sim.queue = i64::MAX;
        let (id, name, hash) = (id, sim.name.clone(), sim.hash.clone());
        self.requeue();
        Ok(json!({ "torrent_added": { "id": id, "name": name, "hash_string": hash } }))
    }
}

/// An object with only `fields` (all of it when there's no list).
fn only(all: &Map<String, Value>, fields: Option<&Value>) -> Map<String, Value> {
    match fields.and_then(Value::as_array) {
        Some(list) if !list.is_empty() => {
            list.iter().filter_map(Value::as_str).filter_map(|k| Some((k.to_owned(), all.get(k)?.clone()))).collect()
        }
        _ => all.clone(),
    }
}

fn apply(sim: &mut Sim, set: &TorrentSet) {
    if let Some(v) = &set.labels {
        sim.labels.clone_from(v);
    }
    if let Some(v) = set.bandwidth_priority {
        sim.priority = v;
    }
    if let Some(v) = set.download_limit {
        sim.download_limit = v;
    }
    if let Some(v) = set.download_limited {
        sim.download_limited = v;
    }
    if let Some(v) = set.upload_limit {
        sim.upload_limit = v;
    }
    if let Some(v) = set.upload_limited {
        sim.upload_limited = v;
    }
    if let Some(v) = set.honors_session_limits {
        sim.honors_session_limits = v;
    }
    if let Some(v) = set.peer_limit {
        sim.peer_limit = v;
    }
    if let Some(v) = set.seed_ratio_limit {
        sim.seed_ratio_limit = v;
    }
    if let Some(v) = set.seed_ratio_mode {
        sim.seed_ratio_mode = v;
    }
    if let Some(v) = set.seed_idle_limit {
        sim.seed_idle_limit = v;
    }
    if let Some(v) = set.seed_idle_mode {
        sim.seed_idle_mode = v;
    }
    if let Some(v) = set.sequential_download {
        sim.sequential = v;
    }
    if let Some(v) = set.queue_position {
        sim.queue = v;
    }
    if let Some(v) = &set.tracker_list {
        sim.tracker = v.lines().find(|l| !l.trim().is_empty()).unwrap_or(OPEN).trim().to_owned();
    }
    let count = sim.files.len();
    let each = |list: &Option<Vec<i64>>, mut f: Box<dyn FnMut(usize) + '_>| {
        // An empty list means every file, as in Transmission.
        match list {
            Some(l) if l.is_empty() => (0..count).for_each(&mut f),
            Some(l) => l.iter().filter_map(|&i| usize::try_from(i).ok()).filter(|&i| i < count).for_each(&mut f),
            None => {}
        }
    };
    let (wanted, priorities) = (&mut sim.wanted, &mut sim.priorities);
    each(&set.files_wanted, Box::new(|i| wanted[i] = true));
    each(&set.files_unwanted, Box::new(|i| wanted[i] = false));
    each(&set.priority_high, Box::new(|i| priorities[i] = Priority::High));
    each(&set.priority_normal, Box::new(|i| priorities[i] = Priority::Normal));
    each(&set.priority_low, Box::new(|i| priorities[i] = Priority::Low));
    // Less wanted can mean it's already done.
    sim.have = sim.have.min(sim.size_when_done() as f64);
}

fn initial_session() -> Map<String, Value> {
    let session = json!({
        "alt_speed_down": 1000,
        "alt_speed_enabled": false,
        "alt_speed_time_begin": 540,
        "alt_speed_time_day": 62,
        "alt_speed_time_enabled": false,
        "alt_speed_time_end": 1020,
        "alt_speed_up": 250,
        "anti_brute_force_enabled": true,
        "blocklist_enabled": true,
        "blocklist_size": 398_412,
        "blocklist_url": "https://github.com/Naunter/BT_BlockLists/raw/master/bt_blocklists.gz",
        "cache_size_mib": 16,
        "config_dir": "/Users/alice/Library/Application Support/Transmission",
        "default_trackers": "",
        "dht_enabled": true,
        "download_dir": "/Users/alice/Downloads",
        "download_queue_enabled": true,
        "download_queue_size": 5,
        "encryption": "preferred",
        "idle_seeding_limit": 30,
        "idle_seeding_limit_enabled": false,
        "incomplete_dir": "/Users/alice/Downloads/Incomplete",
        "incomplete_dir_enabled": false,
        "lpd_enabled": false,
        "peer_limit_global": 200,
        "peer_limit_per_torrent": 50,
        "peer_port_random_on_start": false,
        "peer_port": 51413,
        "pex_enabled": true,
        "port_forwarding_enabled": true,
        "preferred_transports": ["utp", "tcp"],
        "queue_stalled_enabled": true,
        "queue_stalled_minutes": 30,
        "rename_partial_files": true,
        "reqq": 2000,
        "rpc_version_semver": RPC_VERSION,
        "script_torrent_added_enabled": false,
        "script_torrent_added_filename": "",
        "script_torrent_done_enabled": false,
        "script_torrent_done_filename": "",
        "script_torrent_done_seeding_enabled": false,
        "script_torrent_done_seeding_filename": "",
        "seed_queue_enabled": false,
        "seed_queue_size": 10,
        "seed_ratio_limit": 2.0,
        "seed_ratio_limited": true,
        "sequential_download": false,
        "session_id": SESSION_ID,
        "speed_limit_down": 20000,
        "speed_limit_down_enabled": false,
        "speed_limit_up": 2000,
        "speed_limit_up_enabled": true,
        "start_added_torrents": true,
        "trash_original_torrent_files": false,
        "units": {
            "speed_units": ["kB/s", "MB/s", "GB/s", "TB/s"],
            "speed_bytes": 1000,
            "size_units": ["kB", "MB", "GB", "TB"],
            "size_bytes": 1000,
            "memory_units": ["KiB", "MiB", "GiB", "TiB"],
            "memory_bytes": 1024,
        },
        "version": "4.1.3 (mock)",
    });
    session.as_object().cloned().unwrap_or_default()
}
