//! Path mappings: where the host's folders appear on this computer (an SMB
//! mount), so Open and Reveal can find a torrent's files. Per computer, since
//! the Mac and the Linux machine mount the share in different places; kept in
//! `paths.json` in the app's config folder.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One folder on the host, and where it is here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PathMapping {
    /// On the host, e.g. `/Users/jamie/Downloads`.
    pub host: String,
    /// Here, e.g. `/Volumes/Downloads` or `/mnt/mini/Downloads`.
    pub local: String,
}

pub struct Mappings {
    path: PathBuf,
    list: Mutex<Vec<PathMapping>>,
}

/// Trailing slashes off (but `/` stays `/`).
fn trimmed(p: &str) -> &str {
    let t = p.trim().trim_end_matches('/');
    if t.is_empty() && p.trim().starts_with('/') { "/" } else { t }
}

impl Mappings {
    pub fn load(path: PathBuf) -> Self {
        let list = fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Self { path, list: Mutex::new(list) }
    }

    pub fn list(&self) -> Vec<PathMapping> {
        self.list.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Replace them all (blank rows dropped) and save.
    pub fn set(&self, list: Vec<PathMapping>) -> Result<Vec<PathMapping>, String> {
        let list: Vec<PathMapping> = list
            .into_iter()
            .map(|m| PathMapping { host: trimmed(&m.host).to_owned(), local: trimmed(&m.local).to_owned() })
            .filter(|m| !m.host.is_empty() && !m.local.is_empty())
            .collect();
        let json = serde_json::to_vec_pretty(&list).map_err(|e| e.to_string())?;
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        fs::write(&self.path, json).map_err(|e| format!("Couldn't save the path mappings: {e}"))?;
        *self.list.lock().unwrap_or_else(|e| e.into_inner()) = list.clone();
        Ok(list)
    }

    /// Where a path on the host is on this computer, by the mapping with the longest
    /// matching host folder; None when no mapping covers it.
    pub fn to_local(&self, host_path: &str) -> Option<PathBuf> {
        map(&self.list(), host_path)
    }
}

fn map(list: &[PathMapping], host_path: &str) -> Option<PathBuf> {
    let path = trimmed(host_path);
    list.iter()
        .filter_map(|m| {
            let host = trimmed(&m.host);
            // Whole folder names only: /Downloads covers /Downloads/x, not /Downloads2.
            let rest = if host == "/" {
                Some(path.trim_start_matches('/'))
            } else if path == host {
                Some("")
            } else {
                path.strip_prefix(host).and_then(|r| r.strip_prefix('/'))
            }?;
            Some((host.len(), m, rest))
        })
        .max_by_key(|(len, _, _)| *len)
        .map(|(_, m, rest)| {
            let mut local = PathBuf::from(trimmed(&m.local));
            for part in rest.split('/').filter(|p| !p.is_empty()) {
                local.push(part);
            }
            local
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(host: &str, local: &str) -> PathMapping {
        PathMapping { host: host.into(), local: local.into() }
    }

    #[test]
    fn maps_by_whole_folders() {
        let list = [m("/Users/jamie/Downloads", "/Volumes/Downloads")];
        assert_eq!(
            map(&list, "/Users/jamie/Downloads/debian/disk.iso"),
            Some(PathBuf::from("/Volumes/Downloads/debian/disk.iso"))
        );
        assert_eq!(map(&list, "/Users/jamie/Downloads"), Some(PathBuf::from("/Volumes/Downloads")));
        assert_eq!(map(&list, "/Users/jamie/Downloads/"), Some(PathBuf::from("/Volumes/Downloads")));
        assert_eq!(map(&list, "/Users/jamie/Downloads2/x"), None, "not a folder of it");
        assert_eq!(map(&list, "/Users/jamie/Movies/x"), None);
    }

    #[test]
    fn the_longest_match_wins() {
        let list = [m("/data", "/mnt/data"), m("/data/tv", "/mnt/tv")];
        assert_eq!(map(&list, "/data/tv/show/e1.mkv"), Some(PathBuf::from("/mnt/tv/show/e1.mkv")));
        assert_eq!(map(&list, "/data/films/f.mkv"), Some(PathBuf::from("/mnt/data/films/f.mkv")));
    }

    #[test]
    fn the_root_maps_everything() {
        let list = [m("/", "/mnt/mini/")];
        assert_eq!(map(&list, "/srv/x"), Some(PathBuf::from("/mnt/mini/srv/x")));
    }

    #[test]
    fn saving_tidies_and_reloads() {
        let dir = std::env::temp_dir().join(format!("xcvr-paths-{}", std::process::id()));
        let file = dir.join("paths.json");
        let mappings = Mappings::load(file.clone());
        let saved = mappings.set(vec![m(" /Users/jamie/Downloads/ ", "/Volumes/Downloads/"), m("", "/x")]).unwrap();
        assert_eq!(saved, [m("/Users/jamie/Downloads", "/Volumes/Downloads")]);
        assert_eq!(Mappings::load(file).list(), saved);
        fs::remove_dir_all(dir).unwrap();
    }
}
