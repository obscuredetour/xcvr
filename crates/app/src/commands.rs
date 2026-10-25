//! Commands the UI calls (through ui/src/api.ts). Thin: they parse input, call
//! the poller, profiles or keychain, and shape the answer.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use transmission::{Client, SessionField, SessionStats, Torrent};
use ts_rs::TS;

use crate::poller::{self, ConnectionState, Poller, Target};
use crate::profiles::{Profile, Profiles};
use crate::secrets::Secrets;

/// The connection as Settings shows it.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionSettings {
    /// The active profile; None until one is saved.
    pub profile: Option<Profile>,
    /// Whether a password is saved for it.
    pub has_password: bool,
    /// Where passwords are kept, for a sentence ("the macOS Keychain").
    pub key_storage: String,
}

/// What Test Connection found.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    /// `4.1.3 (838877323f)`.
    pub version: String,
    pub torrent_count: i64,
}

/// A failure the UI shows as it is, with a hint at the fix when there is one.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Problem {
    pub message: String,
    pub hint: Option<String>,
}

impl From<String> for Problem {
    fn from(message: String) -> Self {
        Self { message, hint: None }
    }
}

impl From<&transmission::Error> for Problem {
    fn from(e: &transmission::Error) -> Self {
        let (message, hint) = poller::describe(e);
        Self { message, hint }
    }
}

/// Run keychain work off the async threads (a locked keyring can block, and may ask to unlock).
async fn with_secrets<T: Send + 'static>(
    secrets: &Arc<Secrets>,
    f: impl FnOnce(&Secrets) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let secrets = secrets.clone();
    tauri::async_runtime::spawn_blocking(move || f(&secrets)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn connection_settings(
    profiles: State<'_, Profiles>,
    secrets: State<'_, Arc<Secrets>>,
) -> Result<ConnectionSettings, Problem> {
    let profile = profiles.active();
    let id = profile.as_ref().map(|p| p.id.clone());
    let (has_password, key_storage) = with_secrets(&secrets, move |s| {
        let has = match id {
            Some(id) => s.get(&id)?.is_some(),
            None => false,
        };
        Ok((has, s.place()))
    })
    .await?;
    Ok(ConnectionSettings { profile, has_password, key_storage })
}

/// Save the connection and connect to it. `password`: None keeps the saved one,
/// empty removes it, anything else replaces it.
#[tauri::command]
pub async fn save_connection(
    profile: Profile,
    password: Option<String>,
    profiles: State<'_, Profiles>,
    secrets: State<'_, Arc<Secrets>>,
    poller: State<'_, Poller>,
) -> Result<ConnectionSettings, Problem> {
    let profile = check(profile)?;
    let saved = profiles.save_active(profile)?;
    let id = saved.id.clone();
    let password = with_secrets(&secrets, move |s| {
        match password.as_deref() {
            Some("") => s.delete(&id)?,
            Some(pw) => s.set(&id, pw)?,
            None => {}
        }
        s.get(&id)
    })
    .await?;

    let has_password = password.is_some();
    poller.connect(Some(Target { label: saved.label(), config: saved.config(password) }));
    let key_storage = with_secrets(&secrets, |s| Ok(s.place())).await?;
    Ok(ConnectionSettings { profile: Some(saved), has_password, key_storage })
}

/// Forget the connection: disconnect, remove it, and delete its password from the keychain.
#[tauri::command]
pub async fn remove_connection(
    profiles: State<'_, Profiles>,
    secrets: State<'_, Arc<Secrets>>,
    poller: State<'_, Poller>,
) -> Result<ConnectionSettings, Problem> {
    poller.connect(None);
    let removed = profiles.remove_active()?;
    let key_storage = with_secrets(&secrets, move |s| {
        if let Some(profile) = removed {
            s.delete(&profile.id)?;
        }
        Ok(s.place())
    })
    .await?;
    Ok(ConnectionSettings { profile: None, has_password: false, key_storage })
}

/// Try a connection without saving it. `password` None uses the one saved for `profile.id`.
#[tauri::command]
pub async fn test_connection(
    profile: Profile,
    password: Option<String>,
    secrets: State<'_, Arc<Secrets>>,
) -> Result<TestResult, Problem> {
    let profile = check(profile)?;
    let password = match password {
        Some(pw) => Some(pw),
        None if !profile.id.is_empty() => {
            let id = profile.id.clone();
            with_secrets(&secrets, move |s| s.get(&id)).await?
        }
        None => None,
    };
    let client = Client::new(profile.config(password)).map_err(|e| Problem::from(&e))?;
    let session = client.session_get(&[SessionField::Version]).await.map_err(|e| Problem::from(&e))?;
    let stats = client.session_stats().await.map_err(|e| Problem::from(&e))?;
    Ok(TestResult { version: session.version.unwrap_or_default(), torrent_count: stats.torrent_count })
}

#[tauri::command]
pub fn connection_state(poller: State<'_, Poller>) -> ConnectionState {
    poller.state()
}

/// Every torrent as of the last poll; after this, `torrents` events keep it current.
#[tauri::command]
pub fn torrents(poller: State<'_, Poller>) -> Vec<Torrent> {
    poller.torrents()
}

#[tauri::command]
pub fn session_stats(poller: State<'_, Poller>) -> Option<SessionStats> {
    poller.stats()
}

/// Poll now; also retries a failed connection.
#[tauri::command]
pub fn poll_now(poller: State<'_, Poller>) {
    poller.poll_now();
}

/// A profile with its fields tidied, or what's missing.
fn check(mut profile: Profile) -> Result<Profile, Problem> {
    profile.host = profile.host.trim().to_owned();
    profile.username = profile.username.trim().to_owned();
    profile.path = profile.path.trim().to_owned();
    if profile.host.is_empty() {
        return Err(Problem::from("Enter the host's name or IP address.".to_owned()));
    }
    if profile.host.contains("://") || profile.host.contains('/') {
        return Err(Problem::from("Enter just the host name, without http:// or a path.".to_owned()));
    }
    if profile.port == 0 {
        return Err(Problem::from("Enter the RPC port (9091 unless it was changed).".to_owned()));
    }
    if profile.path.is_empty() {
        profile.path = transmission::DEFAULT_PATH.to_owned();
    }
    Ok(profile)
}
