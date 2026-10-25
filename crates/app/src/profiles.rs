//! Connection profiles: the Transmission hosts this app knows, in
//! `connections.json` in the app's config folder. Kept as a list though there's
//! one host today (multiple profiles with a switcher come later). Passwords
//! aren't here; they're in the keychain under each profile's id (secrets.rs).

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A Transmission host and how to sign in to it (less the password).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    /// Stable, made when the profile is first saved; the keychain account for its password.
    #[serde(default)]
    pub id: String,
    pub host: String,
    pub port: u16,
    /// The RPC path, `/transmission/rpc` unless the host changed it.
    pub path: String,
    /// Empty when the host doesn't ask for one.
    pub username: String,
}

impl Profile {
    pub fn config(&self, password: Option<String>) -> transmission::Config {
        let mut config = transmission::Config::new(self.host.trim(), self.port);
        if !self.path.trim().is_empty() {
            config.path = self.path.trim().to_owned();
        }
        if !self.username.is_empty() {
            config = config.with_credentials(&self.username, password.unwrap_or_default());
        }
        config
    }

    /// How the UI names it: the host, and the port when it isn't the default.
    pub fn label(&self) -> String {
        if self.port == transmission::DEFAULT_PORT {
            self.host.trim().to_owned()
        } else {
            format!("{}:{}", self.host.trim(), self.port)
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ProfilesFile {
    active: Option<String>,
    profiles: Vec<Profile>,
}

pub struct Profiles {
    path: PathBuf,
    file: Mutex<ProfilesFile>,
}

impl Profiles {
    /// Read `connections.json` at `path`; a missing or unreadable file is no profiles.
    pub fn load(path: PathBuf) -> Self {
        let file = fs::read(&path).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
        Self { path, file: Mutex::new(file) }
    }

    pub fn active(&self) -> Option<Profile> {
        let file = self.file.lock().unwrap_or_else(|e| e.into_inner());
        let id = file.active.as_deref()?;
        file.profiles.iter().find(|p| p.id == id).cloned()
    }

    /// Save `profile` (a new one gets an id) and make it the active one. Returns it as saved.
    pub fn save_active(&self, mut profile: Profile) -> Result<Profile, String> {
        let mut file = self.file.lock().unwrap_or_else(|e| e.into_inner());
        if profile.id.is_empty() || !file.profiles.iter().any(|p| p.id == profile.id) {
            profile.id = new_id();
            file.profiles.push(profile.clone());
        } else if let Some(existing) = file.profiles.iter_mut().find(|p| p.id == profile.id) {
            *existing = profile.clone();
        }
        file.active = Some(profile.id.clone());
        self.write(&file)?;
        Ok(profile)
    }

    /// Forget the active profile. Returns it (for its keychain entry), or None when there wasn't one.
    pub fn remove_active(&self) -> Result<Option<Profile>, String> {
        let mut file = self.file.lock().unwrap_or_else(|e| e.into_inner());
        let Some(id) = file.active.take() else { return Ok(None) };
        let removed = file.profiles.iter().position(|p| p.id == id).map(|i| file.profiles.remove(i));
        self.write(&file)?;
        Ok(removed)
    }

    fn write(&self, file: &ProfilesFile) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(file).map_err(|e| e.to_string())?;
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        fs::write(&self.path, json).map_err(|e| format!("Couldn't save the connection: {e}"))
    }
}

/// A unique enough id: the time in nanoseconds, in hex.
fn new_id() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or_default();
    format!("{nanos:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(host: &str) -> Profile {
        Profile {
            id: String::new(),
            host: host.into(),
            port: 9091,
            path: "/transmission/rpc".into(),
            username: "me".into(),
        }
    }

    #[test]
    fn saves_and_reloads_the_active_profile() {
        let dir = std::env::temp_dir().join(format!("xcvr-profiles-{}", new_id()));
        let path = dir.join("connections.json");
        let profiles = Profiles::load(path.clone());
        assert_eq!(profiles.active(), None);

        let saved = profiles.save_active(profile("mini.local")).unwrap();
        assert!(!saved.id.is_empty());
        // Saving again with its id updates it in place.
        let mut edited = saved.clone();
        edited.port = 9092;
        profiles.save_active(edited.clone()).unwrap();

        let reloaded = Profiles::load(path);
        assert_eq!(reloaded.active(), Some(edited));
        assert_eq!(reloaded.file.lock().unwrap().profiles.len(), 1, "edited in place, not added");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn removing_forgets_the_active_profile() {
        let dir = std::env::temp_dir().join(format!("xcvr-profiles-{}", new_id()));
        let path = dir.join("connections.json");
        let profiles = Profiles::load(path.clone());
        assert_eq!(profiles.remove_active(), Ok(None), "nothing to remove");

        let saved = profiles.save_active(profile("mini.local")).unwrap();
        assert_eq!(profiles.remove_active(), Ok(Some(saved)));
        assert_eq!(profiles.active(), None);

        let reloaded = Profiles::load(path);
        assert_eq!(reloaded.active(), None);
        assert!(reloaded.file.lock().unwrap().profiles.is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn labels_and_configs() {
        let mut p = profile(" mini.local ");
        assert_eq!(p.label(), "mini.local");
        assert_eq!(p.config(Some("pw".into())).url(), "http://mini.local:9091/transmission/rpc");
        p.port = 8080;
        p.username.clear();
        assert_eq!(p.label(), "mini.local:8080");
        assert_eq!(p.config(None).username, None);
    }
}
