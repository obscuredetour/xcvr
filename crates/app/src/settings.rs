//! The app's own settings (Settings › General), in `app.json` in its config
//! folder: read in Rust, since closing the window and finishing torrents are
//! handled there. Per computer.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    /// Closing the window hides it (the app stays in the menu bar / tray); off, it quits.
    pub keep_running: bool,
    /// A notification when a torrent finishes downloading.
    pub notify_finished: bool,
    /// The global shortcut that shows the window.
    pub shortcut_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self { keep_running: true, notify_finished: true, shortcut_enabled: true }
    }
}

pub struct Settings {
    path: PathBuf,
    current: Mutex<AppSettings>,
}

impl Settings {
    pub fn load(path: PathBuf) -> Self {
        let current = fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Self { path, current: Mutex::new(current) }
    }

    pub fn get(&self) -> AppSettings {
        self.current.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn set(&self, next: AppSettings) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(&next).map_err(|e| e.to_string())?;
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        fs::write(&self.path, json).map_err(|e| format!("Couldn't save the settings: {e}"))?;
        *self.current.lock().unwrap_or_else(|e| e.into_inner()) = next;
        Ok(())
    }
}
