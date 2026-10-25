//! Torrents' names from before they were renamed in Transceiver, by hash, in
//! `originals.json` in the app's config folder. Transmission forgets them: after
//! a rename its name, files and magnet link all carry the new name, and only its
//! own copy of the .torrent (on the host) keeps the old one. That copy is read
//! when a path mapping reaches it (inspect.rs, `original_name`); this is for when
//! one doesn't. Per computer, so renames made elsewhere aren't here.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Originals {
    path: PathBuf,
    names: Mutex<BTreeMap<String, String>>,
}

impl Originals {
    pub fn load(path: PathBuf) -> Self {
        let names = fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Self { path, names: Mutex::new(names) }
    }

    pub fn get(&self, hash: &str) -> Option<String> {
        self.names.lock().unwrap_or_else(|e| e.into_inner()).get(hash).cloned()
    }

    /// Note a torrent's name before a rename. The first one is kept: renaming twice
    /// still remembers the original, not the name in between.
    pub fn remember(&self, hash: &str, name: &str) {
        let mut names = self.names.lock().unwrap_or_else(|e| e.into_inner());
        if names.contains_key(hash) {
            return;
        }
        names.insert(hash.to_owned(), name.to_owned());
        // Losing this file costs only the "originally" line, so failures aren't surfaced.
        if let Ok(json) = serde_json::to_vec_pretty(&*names) {
            if let Some(dir) = self.path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = fs::write(&self.path, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_first_name() {
        let dir = std::env::temp_dir().join(format!("xcvr-originals-{}", std::process::id()));
        let file = dir.join("originals.json");
        let o = Originals::load(file.clone());
        assert_eq!(o.get("abc"), None);
        o.remember("abc", "Original.iso");
        o.remember("abc", "In.Between.iso");
        assert_eq!(Originals::load(file).get("abc").as_deref(), Some("Original.iso"));
        fs::remove_dir_all(dir).unwrap();
    }
}
