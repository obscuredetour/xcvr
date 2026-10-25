use serde::{Serialize, Serializer};

/// One torrent: its id, or its info hash. Ids change when Transmission
/// restarts; hashes don't, so the app keys everything by hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(untagged)]
pub enum TorrentId {
    Id(i64),
    Hash(String),
}

impl From<i64> for TorrentId {
    fn from(id: i64) -> Self {
        Self::Id(id)
    }
}

impl From<&str> for TorrentId {
    fn from(hash: &str) -> Self {
        Self::Hash(hash.to_owned())
    }
}

impl From<String> for TorrentId {
    fn from(hash: String) -> Self {
        Self::Hash(hash)
    }
}

/// Which torrents a request is about (`ids`, spec 3.1).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Ids {
    /// Every torrent: `ids` is left out.
    #[default]
    All,
    List(Vec<TorrentId>),
    /// Torrents active in the last minute, and `removed` for those removed.
    RecentlyActive,
}

impl Ids {
    pub fn one(id: impl Into<TorrentId>) -> Self {
        Self::List(vec![id.into()])
    }

    /// Torrents by info hash.
    pub fn hashes<S: Into<String>>(hashes: impl IntoIterator<Item = S>) -> Self {
        Self::List(hashes.into_iter().map(|h| TorrentId::Hash(h.into())).collect())
    }

    pub(crate) fn is_all(&self) -> bool {
        matches!(self, Self::All)
    }
}

impl Serialize for Ids {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            // Skipped by every params struct (skip_serializing_if = "Ids::is_all").
            Self::All => s.serialize_none(),
            Self::List(list) => list.serialize(s),
            Self::RecentlyActive => s.serialize_str("recently_active"),
        }
    }
}
