//! Updates (Settings › About): Transceiver looks for a newer version in the
//! latest GitHub release's `latest.json` soon after launch and once a day,
//! unless that's turned off, and installs it when the user says so, then
//! restarts. Installers are signed (the release workflow's updater key;
//! tauri.conf.json has its public half), and an update that doesn't match is
//! refused.
//!
//! Only the copies Transceiver's own installers put there update themselves:
//! the AppImage, the macOS app, and the .deb and .rpm while dpkg or rpm owns the
//! program. A copy a package manager installed some other way (the AUR's
//! transceiver-bin, which repackages the .deb) is that package manager's to
//! update, and so is one built from source.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::utils::config::BundleType;
use tauri::utils::platform::{bundle_type, current_exe};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};
use ts_rs::TS;

use crate::commands::Problem;

pub const UPDATE_PROGRESS_EVENT: &str = "update-progress";

/// Where release notes are, by tag.
const RELEASES: &str = "https://github.com/obscuredetour/xcvr/releases/tag/v";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Settings {
    auto_check: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { auto_check: true }
    }
}

/// What Settings shows.
#[derive(Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    /// Whether this copy can update itself.
    supported: bool,
    /// Why not, when it can't ("Your package manager keeps Transceiver up to date.").
    unsupported: Option<String>,
    /// Look for updates soon after launch and once a day.
    auto_check: bool,
}

/// A newer version, found by a check.
#[derive(Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    version: String,
    /// When it was released (RFC 3339), when the release says.
    date: Option<String>,
}

/// Where an install is up to.
#[derive(Clone, Serialize, TS)]
#[ts(export)]
#[serde(tag = "phase", rename_all = "lowercase")]
pub enum UpdateProgress {
    Downloading {
        done: usize,
        total: Option<usize>,
    },
    /// Running the installer; on Linux, pkexec may be asking for a password first.
    Installing,
}

/// The update the last check found, for install_update.
#[derive(Default)]
pub struct Pending(Mutex<Option<Update>>);

fn settings_path(app: &AppHandle) -> Result<PathBuf, Problem> {
    Ok(app.path().app_config_dir().map_err(|e| Problem::from(e.to_string()))?.join("updates.json"))
}

fn load_settings(app: &AppHandle) -> Settings {
    settings_path(app)
        .ok()
        .and_then(|p| fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_settings(app: &AppHandle, s: &Settings) -> Result<(), Problem> {
    let path = settings_path(app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| Problem::from(e.to_string()))?;
    }
    let json = serde_json::to_vec_pretty(s).map_err(|e| Problem::from(e.to_string()))?;
    fs::write(&path, json).map_err(|e| Problem::from(format!("Couldn’t save the update settings: {e}")))
}

/// Why this copy can't update itself, if it can't. Asked once: it runs dpkg or rpm.
fn unsupported() -> Option<&'static str> {
    static REASON: OnceLock<Option<&'static str>> = OnceLock::new();
    *REASON.get_or_init(|| {
        const PACKAGED: &str = "Transceiver was installed by your system’s package manager, which keeps it up to date.";
        if cfg!(debug_assertions) {
            return Some("This is a development build.");
        }
        let owned_by = |tool: &str, query: &str| current_exe().is_ok_and(|exe| package_owns(tool, query, &exe));
        match bundle_type() {
            None => Some("This copy was built from source: update it the way you built it."),
            // Tauri marks the binary with the bundle it shipped in, so a .deb repackaged for another
            // distribution (the AUR's transceiver-bin) still says deb: dpkg has to own it too.
            Some(BundleType::Deb) if !owned_by("dpkg", "-S") => Some(PACKAGED),
            Some(BundleType::Rpm) if !owned_by("rpm", "-qf") => Some(PACKAGED),
            _ => None,
        }
    })
}

/// Whether `tool` (dpkg, rpm) has a package installed that owns `exe`. False when there's no such tool.
fn package_owns(tool: &str, query: &str, exe: &Path) -> bool {
    Command::new(tool)
        .arg(query)
        .arg(exe)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn status(app: &AppHandle) -> UpdateStatus {
    let reason = unsupported();
    UpdateStatus {
        supported: reason.is_none(),
        unsupported: reason.map(str::to_owned),
        auto_check: load_settings(app).auto_check,
    }
}

#[tauri::command]
pub async fn update_status(app: AppHandle) -> Result<UpdateStatus, Problem> {
    Ok(status(&app))
}

/// Turn the automatic checks on or off.
#[tauri::command]
pub async fn set_auto_update(app: AppHandle, on: bool) -> Result<UpdateStatus, Problem> {
    save_settings(&app, &Settings { auto_check: on })?;
    Ok(status(&app))
}

/// Look for a newer version. None when this one is the latest.
#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<Option<UpdateInfo>, Problem> {
    if let Some(reason) = unsupported() {
        return Err(Problem::from(reason.to_owned()));
    }
    let updater =
        app.updater_builder().timeout(Duration::from_secs(30)).build().map_err(|e| Problem::from(e.to_string()))?;
    let found = updater.check().await.map_err(|e| Problem::from(format!("Couldn’t check for updates: {e}")))?;
    let info = found.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        date: u.raw_json.get("pub_date").and_then(|d| d.as_str()).map(str::to_owned),
    });
    *app.state::<Pending>().0.lock().unwrap_or_else(|e| e.into_inner()) = found;
    Ok(info)
}

/// Download and install the update the last check found, then restart into it.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), Problem> {
    let update = app.state::<Pending>().0.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let update =
        update.ok_or_else(|| Problem::from("There’s no update to install. Check for updates first.".to_owned()))?;
    let mut done = 0;
    let mut last = Instant::now();
    let bytes = update
        .download(
            |chunk, total| {
                done += chunk;
                // Chunks are small: a few updates a second are plenty.
                if last.elapsed() >= Duration::from_millis(200) {
                    last = Instant::now();
                    let total = total.map(|t| t as usize);
                    let _ = app.emit(UPDATE_PROGRESS_EVENT, UpdateProgress::Downloading { done, total });
                }
            },
            || {},
        )
        .await
        .map_err(|e| Problem::from(format!("Couldn’t download Transceiver {}: {e}", update.version)))?;
    let _ = app.emit(UPDATE_PROGRESS_EVENT, UpdateProgress::Installing);
    // Blocks while pkexec asks for a password (deb, rpm).
    let version = update.version.clone();
    tauri::async_runtime::spawn_blocking(move || update.install(bytes))
        .await
        .map_err(|e| Problem::from(e.to_string()))?
        .map_err(|e| Problem::from(format!("Couldn’t install Transceiver {version}: {e}")))?;
    app.request_restart();
    Ok(())
}

/// Open a version's release notes in the browser.
#[tauri::command]
pub async fn open_release_notes(app: AppHandle, version: String) -> Result<(), Problem> {
    if !version.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+')) {
        return Err(Problem::from("That isn’t a version.".to_owned()));
    }
    app.opener()
        .open_url(format!("{RELEASES}{version}"), None::<&str>)
        .map_err(|e| Problem::from(format!("Couldn’t open the release notes: {e}")))
}
