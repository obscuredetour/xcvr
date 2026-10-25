//! Reading a .torrent file on this machine, enough for the add dialog to show
//! what it is (its name, size and files) before it's sent to Transmission.
//! A minimal bencode reader: no info hash, no validation beyond what's read.

use std::collections::BTreeMap;

use serde::Serialize;

/// What a .torrent file holds, as the add dialog shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct Metainfo {
    pub name: String,
    /// Bytes, every file together.
    pub total_size: i64,
    pub file_count: i64,
    pub private: bool,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("This isn't a .torrent file Transceiver can read: {0}.")]
pub struct BadMetainfo(String);

/// Read the name, size and file count from a .torrent file's bytes.
pub fn read_metainfo(bytes: &[u8]) -> Result<Metainfo, BadMetainfo> {
    let (value, _) = parse(bytes, 0, 0)?;
    let Value::Dict(root) = value else { return Err(BadMetainfo("not a dictionary".into())) };
    let Some(Value::Dict(info)) = root.get(b"info".as_slice()) else {
        return Err(BadMetainfo("no info".into()));
    };
    // Prefer the UTF-8 name some creators add beside the original.
    let name =
        [b"name.utf-8".as_slice(), b"name"].iter().find_map(|k| info.get(*k).and_then(Value::text)).unwrap_or_default();

    let (total_size, file_count) = match info.get(b"files".as_slice()) {
        Some(Value::List(files)) => {
            let total = files.iter().filter_map(|f| f.dict()?.get(b"length".as_slice())?.int()).sum();
            (total, files.len() as i64)
        }
        _ => (info.get(b"length".as_slice()).and_then(Value::int).unwrap_or(0), 1),
    };
    let private = info.get(b"private".as_slice()).and_then(Value::int) == Some(1);
    Ok(Metainfo { name, total_size, file_count, private })
}

#[derive(Debug)]
enum Value<'a> {
    Int(i64),
    Bytes(&'a [u8]),
    List(Vec<Value<'a>>),
    Dict(BTreeMap<&'a [u8], Value<'a>>),
}

impl<'a> Value<'a> {
    fn int(&self) -> Option<i64> {
        if let Value::Int(n) = self { Some(*n) } else { None }
    }

    fn text(&self) -> Option<String> {
        if let Value::Bytes(b) = self { Some(String::from_utf8_lossy(b).into_owned()) } else { None }
    }

    fn dict(&self) -> Option<&BTreeMap<&'a [u8], Value<'a>>> {
        if let Value::Dict(d) = self { Some(d) } else { None }
    }
}

/// Nesting deeper than any real .torrent, so a crafted file can't exhaust the stack.
const MAX_DEPTH: usize = 64;

fn parse(b: &[u8], at: usize, depth: usize) -> Result<(Value<'_>, usize), BadMetainfo> {
    let bad = |what: &str| BadMetainfo(format!("{what} at byte {at}"));
    if depth > MAX_DEPTH {
        return Err(bad("nested too deeply"));
    }
    match b.get(at) {
        Some(b'i') => {
            let end = find(b, at + 1, b'e').ok_or_else(|| bad("unterminated number"))?;
            let n = std::str::from_utf8(&b[at + 1..end])
                .ok()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| bad("bad number"))?;
            Ok((Value::Int(n), end + 1))
        }
        Some(b'l') => {
            let mut items = Vec::new();
            let mut pos = at + 1;
            while b.get(pos) != Some(&b'e') {
                let (item, next) = parse(b, pos, depth + 1)?;
                items.push(item);
                pos = next;
            }
            Ok((Value::List(items), pos + 1))
        }
        Some(b'd') => {
            let mut map = BTreeMap::new();
            let mut pos = at + 1;
            while b.get(pos) != Some(&b'e') {
                let (key, next) = parse(b, pos, depth + 1)?;
                let Value::Bytes(key) = key else { return Err(bad("a key that isn't a string")) };
                let (value, next) = parse(b, next, depth + 1)?;
                map.insert(key, value);
                pos = next;
            }
            Ok((Value::Dict(map), pos + 1))
        }
        Some(c) if c.is_ascii_digit() => {
            let colon = find(b, at, b':').ok_or_else(|| bad("unterminated length"))?;
            let len: usize = std::str::from_utf8(&b[at..colon])
                .ok()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| bad("bad length"))?;
            let start = colon + 1;
            let end = start.checked_add(len).filter(|&e| e <= b.len()).ok_or_else(|| bad("a string past the end"))?;
            Ok((Value::Bytes(&b[start..end]), end))
        }
        Some(_) => Err(bad("unexpected byte")),
        None => Err(bad("the end, too soon")),
    }
}

fn find(b: &[u8], from: usize, byte: u8) -> Option<usize> {
    b.get(from..)?.iter().position(|&c| c == byte).map(|i| from + i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_file() {
        let t = b"d8:announce3:url4:infod6:lengthi1000e4:name8:test.iso12:piece lengthi16384e6:pieces20:aaaaaaaaaaaaaaaaaaaaee";
        assert_eq!(
            read_metainfo(t).unwrap(),
            Metainfo { name: "test.iso".into(), total_size: 1000, file_count: 1, private: false }
        );
    }

    #[test]
    fn several_files_and_private() {
        let t = b"d4:infod5:filesld6:lengthi10e4:pathl1:aeed6:lengthi32e4:pathl3:dir1:beee4:name6:folder7:privatei1eee";
        assert_eq!(
            read_metainfo(t).unwrap(),
            Metainfo { name: "folder".into(), total_size: 42, file_count: 2, private: true }
        );
    }

    #[test]
    fn prefers_the_utf8_name() {
        let t = b"d4:infod6:lengthi1e4:name3:abc10:name.utf-85:\xc3\xa9t\xc3\xa9ee";
        assert_eq!(read_metainfo(t).unwrap().name, "été");
    }

    #[test]
    fn garbage_is_refused() {
        assert!(read_metainfo(b"<html>not a torrent</html>").is_err());
        assert!(read_metainfo(b"d4:info").is_err());
        assert!(read_metainfo(b"d4:infod4:name999:xee").is_err(), "a length past the end");
        assert!(read_metainfo(b"d4:infodee").is_ok(), "an empty info is odd but readable");
        let deep = format!("{}{}", "l".repeat(200), "e".repeat(200));
        assert!(read_metainfo(deep.as_bytes()).is_err(), "too deep");
    }
}
