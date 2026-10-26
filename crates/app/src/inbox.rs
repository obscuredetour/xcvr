//! What arrives from outside the window: magnet links clicked elsewhere,
//! .torrent files opened from Finder or the file manager, and `--show`. They
//! come as command-line arguments (a second launch, forwarded by
//! single-instance; Linux opens files and links that way) or as URLs (macOS,
//! through the deep-link plugin, `file://` ones included). They wait here until
//! the UI is connected and takes them (`take_inbox`), so one clicked while the
//! app is starting isn't lost.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use ts_rs::TS;

pub const INBOX_EVENT: &str = "inbox";

/// Links and files waiting for the add dialog.
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Incoming {
    pub links: Vec<String>,
    pub files: Vec<String>,
}

impl Incoming {
    fn is_empty(&self) -> bool {
        self.links.is_empty() && self.files.is_empty()
    }
}

#[derive(Default)]
pub struct Inbox(Mutex<Incoming>);

/// What a list of arguments asks for: links, files, and whether to show the window.
#[derive(Debug, Default, PartialEq)]
pub struct Asked {
    pub incoming: Incoming,
    pub show: bool,
}

/// Read command-line arguments (without the program's own name). Relative paths
/// are taken from `cwd`; files that aren't .torrent files, and paths that don't
/// exist, are skipped.
pub fn parse_args(args: &[String], cwd: &Path) -> Asked {
    let mut asked = Asked::default();
    for arg in args {
        if arg == "--show" {
            asked.show = true;
        } else if arg.starts_with('-') {
            continue;
        } else if arg.to_ascii_lowercase().starts_with("magnet:") {
            asked.incoming.links.push(arg.clone());
        } else if let Some(path) = torrent_path(arg, cwd) {
            asked.incoming.files.push(path.to_string_lossy().into_owned());
        }
    }
    asked
}

/// A `.torrent` file named by an argument or a `file://` URL, if it's there.
fn torrent_path(arg: &str, cwd: &Path) -> Option<PathBuf> {
    let path = match arg.strip_prefix("file://") {
        Some(rest) => PathBuf::from(percent_decode(rest)),
        None => cwd.join(arg),
    };
    let is_torrent = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("torrent"));
    (is_torrent && path.is_file()).then_some(path)
}

/// `%20` and friends in a file URL's path.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(byte) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Queue what arrived and tell the UI; show the window when asked to, or when
/// there's something to add.
pub fn receive<R: Runtime>(app: &AppHandle<R>, asked: Asked) {
    let has_items = !asked.incoming.is_empty();
    if has_items && let Some(inbox) = app.try_state::<Inbox>() {
        let mut waiting = inbox.0.lock().unwrap_or_else(|e| e.into_inner());
        for link in asked.incoming.links {
            if !waiting.links.contains(&link) {
                waiting.links.push(link);
            }
        }
        for file in asked.incoming.files {
            if !waiting.files.contains(&file) {
                waiting.files.push(file);
            }
        }
    }
    if asked.show || has_items {
        show_window(app);
    }
    if has_items {
        let _ = app.emit(INBOX_EVENT, ());
    }
}

/// Bring the main window forward (it may be hidden in the tray, or minimized).
pub fn show_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Everything waiting, handed over and cleared.
#[tauri::command]
pub fn take_inbox(inbox: State<'_, Inbox>) -> Incoming {
    std::mem::take(&mut *inbox.0.lock().unwrap_or_else(|e| e.into_inner()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn links_files_and_show() {
        let dir = std::env::temp_dir().join(format!("xcvr-inbox-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a b.torrent"), b"d4:infodee").unwrap();
        std::fs::write(dir.join("notes.txt"), b"").unwrap();
        let file_url = format!("file://{}", dir.join("a%20b.torrent").display());

        let asked = parse_args(
            &args(&[
                "--hidden",
                "magnet:?xt=urn:btih:abc",
                "MAGNET:?xt=urn:btih:DEF",
                "a b.torrent",
                "notes.txt",
                "missing.torrent",
                &file_url,
                "--show",
            ]),
            &dir,
        );
        assert!(asked.show);
        assert_eq!(asked.incoming.links, ["magnet:?xt=urn:btih:abc", "MAGNET:?xt=urn:btih:DEF"]);
        let expected = dir.join("a b.torrent").to_string_lossy().into_owned();
        assert_eq!(asked.incoming.files, [expected.clone(), expected], "by path, and by file URL");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn nothing_asked() {
        assert_eq!(parse_args(&args(&["--hidden"]), Path::new("/")), Asked::default());
    }
}
