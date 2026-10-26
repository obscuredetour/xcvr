//! The poller: one background task that owns the connection to Transmission,
//! fetches the torrent list and session stats on an interval, and tells the UI
//! what changed. The UI never polls; it listens for:
//!
//! - `connection` ([`ConnectionState`]) when the state changes,
//! - `torrents` ([`transmission::Diff`]) when the list changes,
//! - `stats` ([`transmission::SessionStats`]) when the speeds or counts change,
//! - `details` ([`transmission::Torrent`], every DETAIL field) when the torrent
//!   the inspector follows changes (see [`Poller::follow`]),
//! - `session` ([`transmission::Session`], the turtle and speed limits) when they change.
//!
//! Every poll fetches every torrent's list fields (tens of torrents makes that
//! cheap), so there's no `recently_active` bookkeeping to go stale.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::sync::mpsc;
use transmission::{
    Client, Config, Diff, Error, Ids, Session, SessionField, SessionStats, Snapshot, Torrent, TorrentField,
};
use ts_rs::TS;

pub const CONNECTION_EVENT: &str = "connection";
pub const TORRENTS_EVENT: &str = "torrents";
pub const STATS_EVENT: &str = "stats";
pub const DETAILS_EVENT: &str = "details";
pub const SESSION_EVENT: &str = "session";

/// The session settings the window shows all the time (the turtle and the speed
/// limits), polled with the list since the schedule or another client can change them.
const SESSION_FIELDS: &[SessionField] = &[
    SessionField::AltSpeedEnabled,
    SessionField::AltSpeedDown,
    SessionField::AltSpeedUp,
    SessionField::AltSpeedTimeEnabled,
    SessionField::SpeedLimitDown,
    SessionField::SpeedLimitDownEnabled,
    SessionField::SpeedLimitUp,
    SessionField::SpeedLimitUpEnabled,
    // The default download folder: the sidebar names folders relative to it.
    SessionField::DownloadDir,
];

/// How often to poll while the window is showing, and while it's hidden or minimized.
const VISIBLE_INTERVAL: Duration = Duration::from_millis(1500);
const HIDDEN_INTERVAL: Duration = Duration::from_secs(5);
/// The longest wait between attempts while the host can't be reached.
const MAX_BACKOFF_SECS: u64 = 30;
/// A wall-clock gap this much longer than the wait means the computer slept.
const SLEPT_AFTER: Duration = Duration::from_secs(30);

/// Where the connection stands, as the status bar and Settings show it.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ConnectionState {
    /// No host set up yet.
    NotConfigured,
    Connecting {
        host: String,
    },
    Connected {
        host: String,
        version: String,
    },
    /// The username or password was rejected. Not retried (Transmission can lock out
    /// repeated failures) until the settings change or Retry is pressed.
    AuthFailed {
        host: String,
        message: String,
    },
    /// Something that retrying won't fix: a Transmission that's too old.
    Failed {
        host: String,
        message: String,
    },
    /// Couldn't reach it, or it refused; retried with a growing wait (`retry`, in seconds).
    Unreachable {
        host: String,
        message: String,
        hint: Option<String>,
        retry: u64,
    },
}

/// A host to connect to: its config, and how to name it.
pub struct Target {
    pub label: String,
    pub config: Config,
}

enum Msg {
    /// Connect somewhere else, or (None) disconnect.
    Connect(Option<Target>),
    /// Poll now: after a command, or Retry.
    PollNow,
}

#[derive(Default)]
struct Shared {
    state: Mutex<Option<ConnectionState>>,
    snapshot: Mutex<Snapshot>,
    stats: Mutex<Option<SessionStats>>,
    client: Mutex<Option<Arc<Client>>>,
    /// The torrent the inspector shows (by hash), and its details as last sent.
    followed: Mutex<Option<String>>,
    details: Mutex<Option<Torrent>>,
    /// SESSION_FIELDS as last sent.
    session: Mutex<Option<Session>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The handle commands use; the task itself runs until the app quits.
pub struct Poller {
    tx: mpsc::UnboundedSender<Msg>,
    shared: Arc<Shared>,
}

impl Poller {
    pub fn start<R: Runtime>(app: AppHandle<R>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let shared = Arc::new(Shared::default());
        tauri::async_runtime::spawn(run(app, rx, shared.clone()));
        Self { tx, shared }
    }

    /// Connect to `target`, dropping the current connection; None disconnects.
    pub fn connect(&self, target: Option<Target>) {
        let _ = self.tx.send(Msg::Connect(target));
    }

    /// Poll now rather than at the next tick. Also retries after a failure.
    pub fn poll_now(&self) {
        let _ = self.tx.send(Msg::PollNow);
    }

    pub fn state(&self) -> ConnectionState {
        lock(&self.shared.state).clone().unwrap_or(ConnectionState::NotConfigured)
    }

    /// Every torrent as of the last poll (for a UI that's just loaded).
    pub fn torrents(&self) -> Vec<Torrent> {
        lock(&self.shared.snapshot).torrents()
    }

    pub fn stats(&self) -> Option<SessionStats> {
        lock(&self.shared.stats).clone()
    }

    /// The turtle and speed limits as of the last poll.
    pub fn session(&self) -> Option<Session> {
        lock(&self.shared.session).clone()
    }

    /// The connected client, for commands (start, stop, add…).
    pub fn client(&self) -> Option<Arc<Client>> {
        lock(&self.shared.client).clone()
    }

    /// Follow one torrent's details (files, peers, trackers, options) on every poll,
    /// as `details` events; None stops. `current` is what the caller just fetched, so
    /// the next event is only a change from it.
    pub fn follow(&self, hash: Option<String>, current: Option<Torrent>) {
        *lock(&self.shared.followed) = hash;
        *lock(&self.shared.details) = current;
    }
}

/// The task: wait for a tick or a message, poll, report.
async fn run<R: Runtime>(app: AppHandle<R>, mut rx: mpsc::UnboundedReceiver<Msg>, shared: Arc<Shared>) {
    let mut task = Task { app, shared, label: String::new(), client: None, connected: false, failures: 0 };
    task.set_state(ConnectionState::NotConfigured);
    // Nothing to do until there's a client, or after a failure retrying won't fix.
    let mut idle = true;
    let mut wait = Duration::ZERO;
    let mut last = SystemTime::now();

    loop {
        let msg = if idle {
            match rx.recv().await {
                Some(msg) => Some(msg),
                None => return,
            }
        } else {
            tokio::select! {
                msg = rx.recv() => match msg {
                    Some(msg) => Some(msg),
                    None => return,
                },
                () = tokio::time::sleep(wait) => None,
            }
        };
        match msg {
            Some(Msg::Connect(target)) => task.connect(target),
            Some(Msg::PollNow) => task.failures = 0,
            None => {}
        }
        if task.client.is_none() {
            idle = true;
            continue;
        }

        // Woken from sleep: the list may be far behind, and completions while asleep
        // aren't news any more. Start over with a full list.
        let now = SystemTime::now();
        if now.duration_since(last).unwrap_or_default() > wait + SLEPT_AFTER {
            lock(&task.shared.snapshot).clear();
        }
        last = now;

        match task.poll().await {
            Ok(()) => {
                idle = false;
                wait = task.interval();
            }
            Err(retry) => match retry {
                Some(after) => {
                    idle = false;
                    wait = after;
                }
                None => idle = true,
            },
        }
    }
}

struct Task<R: Runtime> {
    app: AppHandle<R>,
    shared: Arc<Shared>,
    label: String,
    client: Option<Arc<Client>>,
    /// Whether the version check has passed since connecting.
    connected: bool,
    /// Failures in a row, for the backoff.
    failures: u32,
}

impl<R: Runtime> Task<R> {
    fn connect(&mut self, target: Option<Target>) {
        self.connected = false;
        self.failures = 0;
        lock(&self.shared.snapshot).clear();
        *lock(&self.shared.stats) = None;
        *lock(&self.shared.followed) = None;
        *lock(&self.shared.details) = None;
        *lock(&self.shared.session) = None;
        // The old host's torrents go from the UI straight away.
        let _ = self.app.emit(TORRENTS_EVENT, Diff { reset: true, ..Diff::default() });

        let (label, client) = match target {
            Some(t) => match Client::new(t.config) {
                Ok(c) => (t.label, Some(Arc::new(c))),
                Err(e) => {
                    self.set_state(ConnectionState::Failed { host: t.label.clone(), message: e.to_string() });
                    (t.label, None)
                }
            },
            None => (String::new(), None),
        };
        self.label = label;
        self.client = client.clone();
        *lock(&self.shared.client) = client;
        if self.client.is_some() {
            self.set_state(ConnectionState::Connecting { host: self.label.clone() });
        } else if self.label.is_empty() {
            self.set_state(ConnectionState::NotConfigured);
        }
    }

    /// One poll. On failure, when to try again (None: not until told to).
    async fn poll(&mut self) -> Result<(), Option<Duration>> {
        let Some(client) = self.client.clone() else { return Err(None) };

        if !self.connected {
            let session = client.session_get(&[SessionField::Version]).await.map_err(|e| self.failed(&e))?;
            self.connected = true;
            self.failures = 0;
            let version = session.version.unwrap_or_default();
            self.set_state(ConnectionState::Connected { host: self.label.clone(), version });
        }

        let (list, stats, session) = tokio::try_join!(
            client.torrent_get(&Ids::All, TorrentField::LIST),
            client.session_stats(),
            client.session_get(SESSION_FIELDS),
        )
        .map_err(|e| self.failed(&e))?;
        self.failures = 0;

        let diff = lock(&self.shared.snapshot).apply(list.torrents);
        if !diff.is_empty() {
            let _ = self.app.emit(TORRENTS_EVENT, &diff);
        }
        if !diff.completed.is_empty() {
            let names: Vec<String> = {
                let snapshot = lock(&self.shared.snapshot);
                let all = snapshot.torrents();
                diff.completed
                    .iter()
                    .filter_map(|h| all.iter().find(|t| t.hash_string.as_ref() == Some(h))?.name.clone())
                    .collect()
            };
            notify_finished(&self.app, &names);
        }
        let changed = lock(&self.shared.stats).as_ref() != Some(&stats);
        if changed {
            let _ = self.app.emit(STATS_EVENT, &stats);
            *lock(&self.shared.stats) = Some(stats);
        }
        let changed = lock(&self.shared.session).as_ref() != Some(&session);
        if changed {
            let _ = self.app.emit(SESSION_EVENT, &session);
            *lock(&self.shared.session) = Some(session);
        }
        crate::tray::update(&self.app, lock(&self.shared.stats).as_ref(), lock(&self.shared.session).as_ref());

        // The inspector's torrent, in full. A failure here isn't a lost connection
        // (the list just came back), so it's skipped until the next poll.
        let followed = lock(&self.shared.followed).clone();
        if let Some(hash) = followed
            && let Ok(got) = client.torrent_get(&Ids::hashes([hash.as_str()]), TorrentField::DETAIL).await
            && let Some(details) = got.torrents.into_iter().next()
            // Still the one followed (the inspector may have moved on while this was asked).
            && lock(&self.shared.followed).as_deref() == Some(hash.as_str())
        {
            let mut last = lock(&self.shared.details);
            if last.as_ref() != Some(&details) {
                let _ = self.app.emit(DETAILS_EVENT, &details);
                *last = Some(details);
            }
        }
        Ok(())
    }

    /// Record a failed poll: the new state, and when to try again.
    fn failed(&mut self, e: &Error) -> Option<Duration> {
        self.connected = false;
        self.failures += 1;
        let host = self.label.clone();
        match e {
            Error::Unauthorized => {
                self.set_state(ConnectionState::AuthFailed { host, message: e.to_string() });
                None
            }
            Error::UnsupportedVersion { .. } => {
                self.set_state(ConnectionState::Failed { host, message: e.to_string() });
                None
            }
            _ => {
                let secs = (1u64 << (self.failures - 1).min(5)).min(MAX_BACKOFF_SECS);
                let (message, hint) = describe(e);
                self.set_state(ConnectionState::Unreachable { host, message, hint, retry: secs });
                Some(Duration::from_secs(secs))
            }
        }
    }

    /// Faster while someone can see the window.
    fn interval(&self) -> Duration {
        let visible = self
            .app
            .get_webview_window("main")
            .is_some_and(|w| w.is_visible().unwrap_or(true) && !w.is_minimized().unwrap_or(false));
        if visible { VISIBLE_INTERVAL } else { HIDDEN_INTERVAL }
    }

    fn set_state(&self, state: ConnectionState) {
        let mut current = lock(&self.shared.state);
        if current.as_ref() != Some(&state) {
            // Anything but connected: the tray says so.
            if !matches!(state, ConnectionState::Connected { .. }) {
                crate::tray::update(&self.app, None, None);
            }
            let _ = self.app.emit(CONNECTION_EVENT, &state);
            *current = Some(state);
        }
    }
}

/// A notification for torrents that finished downloading, unless they're turned
/// off (Settings › General) or the notification plugin isn't there (tests).
fn notify_finished<R: Runtime>(app: &AppHandle<R>, names: &[String]) {
    use tauri_plugin_notification::NotificationExt;
    let wanted = app.try_state::<crate::settings::Settings>().is_some_and(|s| s.get().notify_finished);
    if names.is_empty() || !wanted || app.try_state::<tauri_plugin_notification::Notification<R>>().is_none() {
        return;
    }
    let (title, body) = match names {
        [one] => ("Download finished".to_owned(), one.clone()),
        many => (format!("{} downloads finished", many.len()), many.join(", ")),
    };
    let _ = app.notification().builder().title(title).body(body).show();
}

/// An error as a message, and a hint at the likely fix when there is one.
pub fn describe(e: &Error) -> (String, Option<String>) {
    let hint = if e.is_host_unreachable() && cfg!(target_os = "macos") {
        Some(
            "macOS may be keeping Transceiver off the local network. Allow it in System Settings › Privacy & \
             Security › Local Network."
                .to_owned(),
        )
    } else if is_name_lookup(e) {
        Some(if cfg!(target_os = "linux") {
            "The host name didn't resolve. Names ending in .local need avahi and nss-mdns.".to_owned()
        } else {
            "The host name didn't resolve. Check it, or use the host's IP address.".to_owned()
        })
    } else if matches!(e, Error::Forbidden(_)) {
        Some("Transmission's allowed addresses (Settings › Remote) need to include this computer.".to_owned())
    } else {
        None
    };
    (e.to_string(), hint)
}

/// Whether the host name couldn't be looked up (hyper says "dns error").
fn is_name_lookup(e: &Error) -> bool {
    let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(e);
    while let Some(err) = cause {
        let text = err.to_string();
        if text.contains("dns error") || text.contains("failed to lookup address") {
            return true;
        }
        cause = err.source();
    }
    false
}

#[cfg(test)]
mod tests {
    //! The poller against a real Transmission in a container (scripts/test-container.sh),
    //! in Tauri's mock app, watching the events the UI would get.

    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use tauri::Listener;
    use transmission::{AddTorrent, Client, Config, Ids, TorrentSource};

    use super::*;

    fn container(password: &str) -> Config {
        let port = std::env::var("XCVR_TEST_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(19091);
        Config::new("127.0.0.1", port).with_credentials("xcvr", password)
    }

    /// Wait up to 15 s for `check` to hold.
    async fn until(what: &str, check: impl Fn() -> bool) {
        let start = Instant::now();
        while !check() {
            assert!(start.elapsed() < Duration::from_secs(15), "timed out waiting for {what}");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    #[tokio::test]
    #[ignore = "needs a Transmission container: scripts/test-container.sh"]
    async fn container_poller_follows_the_host() {
        let app = tauri::test::mock_app();
        let diffs: Arc<Mutex<Vec<Diff>>> = Arc::default();
        let seen = diffs.clone();
        app.listen_any(TORRENTS_EVENT, move |e| {
            #[derive(serde::Deserialize)]
            struct Raw {
                reset: bool,
                added: Vec<Torrent>,
                removed: Vec<String>,
            }
            let raw: Raw = serde_json::from_str(e.payload()).unwrap();
            lock(&seen).push(Diff { reset: raw.reset, added: raw.added, removed: raw.removed, ..Diff::default() });
        });
        let details: Arc<Mutex<Vec<Torrent>>> = Arc::default();
        let seen_details = details.clone();
        app.listen_any(DETAILS_EVENT, move |e| lock(&seen_details).push(serde_json::from_str(e.payload()).unwrap()));

        let poller = Poller::start(app.handle().clone());
        assert_eq!(poller.state(), ConnectionState::NotConfigured);

        // Connects, and reports the version.
        poller.connect(Some(Target { label: "container".into(), config: container("xcvr") }));
        until("connected", || matches!(poller.state(), ConnectionState::Connected { .. })).await;
        until("stats", || poller.stats().is_some()).await;

        // A torrent added elsewhere shows up as added.
        let other = Client::new(container("xcvr")).unwrap();
        let hash = format!("{:040x}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos());
        let mut add = AddTorrent::new(TorrentSource::Link(format!("magnet:?xt=urn:btih:{hash}&dn=xcvr-poller-test")));
        add.paused = true;
        other.torrent_add(&add).await.unwrap();
        poller.poll_now();
        until("the torrent in the snapshot", || {
            poller.torrents().iter().any(|t| t.hash_string.as_deref() == Some(&hash))
        })
        .await;
        until("an added diff", || {
            lock(&diffs).iter().any(|d| d.added.iter().any(|t| t.hash_string.as_deref() == Some(&hash)))
        })
        .await;

        // Followed by the inspector: its details arrive, with the inspector's fields.
        poller.follow(Some(hash.clone()), None);
        poller.poll_now();
        until("details", || lock(&details).iter().any(|t| t.hash_string.as_deref() == Some(&hash))).await;
        let d = lock(&details).last().cloned().unwrap();
        assert!(d.tracker_stats.is_some() && d.peers.is_some() && d.files.is_some() && d.seed_ratio_mode.is_some());
        poller.follow(None, None);

        // Removed elsewhere: a removed diff.
        other.torrent_remove(&Ids::hashes([hash.as_str()]), false).await.unwrap();
        poller.poll_now();
        until("a removed diff", || lock(&diffs).iter().any(|d| d.removed.contains(&hash))).await;
        assert!(poller.torrents().iter().all(|t| t.hash_string.as_deref() != Some(&hash)));

        // A wrong password stops, rather than retrying into a lockout.
        poller.connect(Some(Target { label: "container".into(), config: container("wrong") }));
        until("auth failed", || matches!(poller.state(), ConnectionState::AuthFailed { .. })).await;
        assert!(poller.torrents().is_empty(), "the old host's torrents are dropped");

        // Nothing listening: unreachable, retrying after a second.
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        poller.connect(Some(Target { label: "nowhere".into(), config: Config::new("127.0.0.1", port) }));
        until("unreachable", || matches!(poller.state(), ConnectionState::Unreachable { retry: 1, .. })).await;

        poller.connect(None);
        until("not configured", || poller.state() == ConnectionState::NotConfigured).await;
    }
}
