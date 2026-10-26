//! The menu bar (macOS) / tray (Linux) icon: speeds and counts, and what can be
//! done without the window. Kept current by the poller (`update`). On macOS the
//! speeds also sit beside the icon; Linux trays mostly can't show text, so there
//! they're in the menu and the tooltip.

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;
use transmission::{Ids, Session, SessionSet, SessionStats};

use crate::inbox::{self, Asked, Incoming};
use crate::poller::Poller;

const TRAY_ID: &str = "main";
/// A short message for the window's toast (from the tray, which has no UI of its own).
pub const NOTICE_EVENT: &str = "notice";

/// The menu items that change.
pub struct TrayItems<R: Runtime> {
    speeds: MenuItem<R>,
    counts: MenuItem<R>,
    turtle: CheckMenuItem<R>,
}

/// Linux trays need libayatana-appindicator (or the older libappindicator), loaded at run
/// time; building the tray without it panics. Arch: `pacman -S libayatana-appindicator`.
#[cfg(target_os = "linux")]
pub fn available() -> bool {
    ["libayatana-appindicator3.so.1", "libappindicator3.so.1"]
        .iter()
        // SAFETY: loading a system library runs its initialisers, as the tray would anyway.
        .any(|name| unsafe { libloading::Library::new(name) }.is_ok())
}

#[cfg(not(target_os = "linux"))]
pub fn available() -> bool {
    true
}

/// Why there's no tray icon, for Settings › General.
pub const MISSING_NOTE: &str = "No tray icon: the system tray library isn't installed, so closing the window quits \
     Transceiver. Install libayatana-appindicator (on Arch: sudo pacman -S libayatana-appindicator) and \
     restart Transceiver.";

/// Whether the tray icon is there (so closing the window can leave the app running).
pub fn is_shown<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.try_state::<TrayItems<R>>().is_some()
}

/// Build the tray, unless its library is missing (Linux).
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if !available() {
        return Ok(());
    }
    let speeds = MenuItem::with_id(app, "speeds", "Not connected", false, None::<&str>)?;
    let counts = MenuItem::with_id(app, "counts", "", false, None::<&str>)?;
    let turtle = CheckMenuItem::with_id(app, "turtle", "Slower Speeds", true, false, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &speeds,
            &counts,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "resume-all", "Resume All", true, None::<&str>)?,
            &MenuItem::with_id(app, "pause-all", "Pause All", true, None::<&str>)?,
            &turtle,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "add-clipboard", "Add Magnet from Clipboard", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "show", "Show Transceiver", true, None::<&str>)?,
            &MenuItem::with_id(app, "quit", "Quit Transceiver", true, None::<&str>)?,
        ],
    )?;

    // macOS: a template image it tints for light and dark menu bars. Linux: the app's
    // own colours, which read on light and dark panels alike.
    let icon = if cfg!(target_os = "macos") {
        Image::from_bytes(include_bytes!("../icons/tray.png"))?
    } else {
        app.default_window_icon().cloned().ok_or_else(|| tauri::Error::AssetNotFound("icon".into()))?
    };
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Transceiver")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()))
        .build(app)?;

    app.manage(TrayItems { speeds, counts, turtle });
    Ok(())
}

fn on_menu<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "show" => inbox::show_window(app),
        "quit" => app.exit(0),
        "add-clipboard" => {
            let text = app.clipboard().read_text().unwrap_or_default();
            let links: Vec<String> = text
                .split_whitespace()
                .filter(|w| w.to_ascii_lowercase().starts_with("magnet:"))
                .map(str::to_owned)
                .collect();
            if links.is_empty() {
                inbox::show_window(app);
                let _ = app.emit(NOTICE_EVENT, "There’s no magnet link on the clipboard.");
            } else {
                inbox::receive(app, Asked { incoming: Incoming { links, files: Vec::new() }, show: true });
            }
        }
        "pause-all" | "resume-all" | "turtle" => {
            let app = app.clone();
            let id = id.to_owned();
            tauri::async_runtime::spawn(async move {
                let poller = app.state::<Poller>();
                let Some(client) = poller.client() else { return };
                let result = match id.as_str() {
                    "pause-all" => client.torrent_stop(&Ids::All).await,
                    "resume-all" => client.torrent_start(&Ids::All).await,
                    _ => {
                        let on = !poller.session().and_then(|s| s.alt_speed_enabled).unwrap_or(false);
                        client.session_set(&SessionSet { alt_speed_enabled: Some(on), ..Default::default() }).await
                    }
                };
                poller.poll_now();
                if let Err(e) = result {
                    let _ = app.emit(NOTICE_EVENT, e.to_string());
                }
            });
        }
        _ => {}
    }
}

/// Show the latest: speeds and counts, the turtle, or that it isn't connected.
pub fn update<R: Runtime>(app: &AppHandle<R>, stats: Option<&SessionStats>, session: Option<&Session>) {
    let Some(items) = app.try_state::<TrayItems<R>>() else { return };
    let tray = app.tray_by_id(TRAY_ID);
    match stats {
        Some(s) => {
            let speeds = format!("↓ {}   ↑ {}", speed(s.download_speed), speed(s.upload_speed));
            let counts = format!(
                "{} active · {} {}",
                s.active_torrent_count,
                s.torrent_count,
                if s.torrent_count == 1 { "torrent" } else { "torrents" }
            );
            let _ = items.speeds.set_text(&speeds);
            let _ = items.counts.set_text(&counts);
            if let Some(tray) = &tray {
                let _ = tray.set_tooltip(Some(format!("Transceiver\n{speeds}\n{counts}")));
                // Beside the icon (macOS): only while something's moving, and short.
                if cfg!(target_os = "macos") {
                    let moving = s.download_speed > 0 || s.upload_speed > 0;
                    let title = moving.then(|| format!("↓{} ↑{}", short(s.download_speed), short(s.upload_speed)));
                    let _ = tray.set_title(title);
                }
            }
        }
        None => {
            let _ = items.speeds.set_text("Not connected");
            let _ = items.counts.set_text("");
            if let Some(tray) = &tray {
                let _ = tray.set_tooltip(Some("Transceiver: not connected"));
                let _ = tray.set_title(None::<&str>);
            }
        }
    }
    if let Some(on) = session.and_then(|s| s.alt_speed_enabled) {
        let _ = items.turtle.set_checked(on);
    }
}

/// "1.2 MB/s", "86 kB/s" (kB = 1000 bytes, as Transmission counts them).
fn speed(bps: i64) -> String {
    if bps >= 1_000_000 { format!("{:.1} MB/s", bps as f64 / 1e6) } else { format!("{} kB/s", (bps + 500) / 1000) }
}

/// "1.2M", "86k", "0": for the menu bar, where room is short.
fn short(bps: i64) -> String {
    match bps {
        0 => "0".into(),
        b if b >= 1_000_000 => format!("{:.1}M", b as f64 / 1e6),
        b => format!("{}k", ((b + 500) / 1000).max(1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speeds_read_short() {
        assert_eq!(speed(1_234_567), "1.2 MB/s");
        assert_eq!(speed(86_400), "86 kB/s");
        assert_eq!(short(0), "0");
        assert_eq!(short(200), "1k");
        assert_eq!(short(86_400), "86k");
        assert_eq!(short(3_400_000), "3.4M");
    }
}
