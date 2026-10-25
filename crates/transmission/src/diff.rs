//! What changed between two polls of the torrent list: the poller keeps a
//! [`Snapshot`] and sends the UI only the [`Diff`].

use std::collections::HashMap;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::torrent::{Status, Torrent};

/// One poll's changes, keyed by info hash (ids change when Transmission restarts).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Diff {
    /// The whole list, not a change to it: the first poll after connecting.
    /// `added` then holds every torrent, and the UI replaces what it had.
    pub reset: bool,
    pub added: Vec<Torrent>,
    /// Only the fields that changed, and `hash_string`.
    pub changed: Vec<Torrent>,
    /// Hashes of torrents that are gone.
    pub removed: Vec<String>,
    /// Hashes of torrents that finished downloading since the last poll.
    pub completed: Vec<String>,
}

impl Diff {
    pub fn is_empty(&self) -> bool {
        !self.reset
            && self.added.is_empty()
            && self.changed.is_empty()
            && self.removed.is_empty()
            && self.completed.is_empty()
    }
}

/// The torrents as of the last poll.
#[derive(Debug, Default)]
pub struct Snapshot {
    torrents: HashMap<String, Torrent>,
    /// Hashes in Transmission's order, for [`Snapshot::torrents`].
    order: Vec<String>,
    /// Whether a poll has been applied since it was made or cleared.
    primed: bool,
}

impl Snapshot {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forget everything: the next poll is a reset (after reconnecting, or waking).
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Every torrent, in the order Transmission listed them.
    pub fn torrents(&self) -> Vec<Torrent> {
        self.order.iter().filter_map(|h| self.torrents.get(h).cloned()).collect()
    }

    /// Take a new poll (every torrent, the same fields each time) and say what changed.
    /// Torrents without a `hash_string` are ignored.
    pub fn apply(&mut self, torrents: Vec<Torrent>) -> Diff {
        let reset = !self.primed;
        self.primed = true;
        let mut diff = Diff { reset, ..Diff::default() };

        let mut next = HashMap::with_capacity(torrents.len());
        let mut order = Vec::with_capacity(torrents.len());
        for torrent in torrents {
            let Some(hash) = torrent.hash_string.clone() else { continue };
            match self.torrents.get(&hash) {
                None => diff.added.push(torrent.clone()),
                Some(_) if reset => unreachable!("a cleared snapshot is empty"),
                Some(prev) => {
                    if let Some(changed) = changed_fields(prev, &torrent) {
                        diff.changed.push(changed);
                    }
                    if finished(prev, &torrent) {
                        diff.completed.push(hash.clone());
                    }
                }
            }
            order.push(hash.clone());
            next.insert(hash, torrent);
        }
        diff.removed = self.order.iter().filter(|h| !next.contains_key(*h)).cloned().collect();

        self.torrents = next;
        self.order = order;
        diff
    }
}

/// `next`'s fields that differ from `prev`'s, with its hash; None when nothing changed.
/// Compared as JSON values, so it works for every field without listing them.
fn changed_fields(prev: &Torrent, next: &Torrent) -> Option<Torrent> {
    let (Ok(Value::Object(before)), Ok(Value::Object(after))) =
        (serde_json::to_value(prev), serde_json::to_value(next))
    else {
        return None;
    };
    let mut changed: Map<String, Value> =
        after.into_iter().filter(|(key, value)| before.get(key) != Some(value)).collect();
    if changed.is_empty() {
        return None;
    }
    changed.insert("hash_string".into(), Value::String(next.hash_string.clone().unwrap_or_default()));
    serde_json::from_value(Value::Object(changed)).ok()
}

/// Whether a torrent finished downloading between two polls: it was downloading with
/// something left, and now nothing's left. That rules out the first poll (no `prev`),
/// verifying a finished torrent (its progress drops and recovers, but it was never
/// downloading), and magnets (nothing is "left" until their metadata arrives).
pub fn finished(prev: &Torrent, next: &Torrent) -> bool {
    prev.status == Some(Status::Downloading)
        && prev.left_until_done.is_some_and(|left| left > 0)
        && next.left_until_done == Some(0)
        && next.metadata_percent_complete.is_none_or(|m| m >= 1.0)
        && !matches!(next.status, Some(Status::QueuedToVerify | Status::Verifying))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn torrent(hash: &str, status: Status, left: i64) -> Torrent {
        Torrent {
            hash_string: Some(hash.into()),
            name: Some(format!("name {hash}")),
            status: Some(status),
            left_until_done: Some(left),
            metadata_percent_complete: Some(1.0),
            rate_download: Some(0),
            ..Default::default()
        }
    }

    #[test]
    fn first_poll_is_a_reset_with_everything() {
        let mut s = Snapshot::new();
        let d = s.apply(vec![torrent("a", Status::Seeding, 0), torrent("b", Status::Downloading, 10)]);
        assert!(d.reset);
        assert_eq!(d.added.len(), 2);
        assert!(d.changed.is_empty() && d.removed.is_empty() && d.completed.is_empty());
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn nothing_changed_is_empty() {
        let mut s = Snapshot::new();
        s.apply(vec![torrent("a", Status::Seeding, 0)]);
        let d = s.apply(vec![torrent("a", Status::Seeding, 0)]);
        assert!(d.is_empty(), "{d:?}");
    }

    #[test]
    fn changes_carry_only_changed_fields() {
        let mut s = Snapshot::new();
        s.apply(vec![torrent("a", Status::Downloading, 100)]);
        let mut next = torrent("a", Status::Downloading, 60);
        next.rate_download = Some(2048);
        let d = s.apply(vec![next]);
        assert!(!d.reset);
        assert_eq!(d.changed.len(), 1);
        let c = &d.changed[0];
        assert_eq!(c.hash_string.as_deref(), Some("a"));
        assert_eq!(c.left_until_done, Some(60));
        assert_eq!(c.rate_download, Some(2048));
        assert_eq!(c.name, None, "unchanged fields stay out");
        assert_eq!(
            serde_json::to_string(c).unwrap(),
            r#"{"hash_string":"a","left_until_done":60,"rate_download":2048}"#
        );
    }

    #[test]
    fn additions_and_removals() {
        let mut s = Snapshot::new();
        s.apply(vec![torrent("a", Status::Seeding, 0), torrent("b", Status::Seeding, 0)]);
        let d = s.apply(vec![torrent("b", Status::Seeding, 0), torrent("c", Status::Stopped, 5)]);
        assert_eq!(d.added.iter().map(|t| t.hash_string.as_deref().unwrap()).collect::<Vec<_>>(), ["c"]);
        assert_eq!(d.removed, ["a"]);
        assert_eq!(s.torrents().iter().map(|t| t.hash_string.clone().unwrap()).collect::<Vec<_>>(), ["b", "c"]);
    }

    #[test]
    fn finishing_is_reported_once() {
        let mut s = Snapshot::new();
        s.apply(vec![torrent("a", Status::Downloading, 10)]);
        let d = s.apply(vec![torrent("a", Status::Seeding, 0)]);
        assert_eq!(d.completed, ["a"]);
        let d = s.apply(vec![torrent("a", Status::Seeding, 0)]);
        assert!(d.completed.is_empty());
    }

    #[test]
    fn finishing_into_stopped_counts() {
        // Reaching a seed limit of 0, or "stop when done".
        let mut s = Snapshot::new();
        s.apply(vec![torrent("a", Status::Downloading, 10)]);
        assert_eq!(s.apply(vec![torrent("a", Status::Stopped, 0)]).completed, ["a"]);
    }

    #[test]
    fn verifying_a_finished_torrent_is_not_finishing() {
        let mut s = Snapshot::new();
        s.apply(vec![torrent("a", Status::Seeding, 0)]);
        assert!(s.apply(vec![torrent("a", Status::Verifying, 500)]).completed.is_empty());
        assert!(s.apply(vec![torrent("a", Status::Verifying, 0)]).completed.is_empty());
        assert!(s.apply(vec![torrent("a", Status::Seeding, 0)]).completed.is_empty());
    }

    #[test]
    fn already_finished_on_first_poll_is_not_finishing() {
        let mut s = Snapshot::new();
        assert!(s.apply(vec![torrent("a", Status::Seeding, 0)]).completed.is_empty());
    }

    #[test]
    fn magnet_getting_metadata_is_not_finishing() {
        let mut s = Snapshot::new();
        let mut fetching = torrent("m", Status::Downloading, 0);
        fetching.metadata_percent_complete = Some(0.3);
        s.apply(vec![fetching]);
        // Metadata arrives: now there's something left, not nothing.
        assert!(s.apply(vec![torrent("m", Status::Downloading, 4096)]).completed.is_empty());
    }

    #[test]
    fn clearing_makes_the_next_poll_a_reset() {
        let mut s = Snapshot::new();
        s.apply(vec![torrent("a", Status::Downloading, 10)]);
        s.clear();
        let d = s.apply(vec![torrent("a", Status::Seeding, 0)]);
        assert!(d.reset);
        assert!(d.completed.is_empty(), "no completions across a reconnect");
    }

    #[test]
    fn torrents_without_a_hash_are_skipped() {
        let mut s = Snapshot::new();
        let d = s.apply(vec![Torrent { name: Some("?".into()), ..Default::default() }]);
        assert!(d.added.is_empty());
        assert!(s.is_empty());
    }
}
