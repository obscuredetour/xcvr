//! macOS menu bar: Tauri's default menu plus "Settings… ⌘," in the app menu,
//! where macOS apps keep it, and Zoom In / Zoom Out / Actual Size in View. (Windows and Linux have no menu bar; the UI
//! handles Ctrl+, itself there.)

use tauri::menu::{Menu, MenuEvent, MenuItem, MenuItemKind, PredefinedMenuItem};
use tauri::{AppHandle, Emitter, Runtime};

const SETTINGS_ID: &str = "settings";

/// Event telling the UI to open the settings dialog. The UI listens for this
/// name in `ui/src/api.ts` (`onOpenSettings`).
pub const OPEN_SETTINGS_EVENT: &str = "open-settings";

/// Event asking the UI to zoom: "in", "out" or "reset". The UI listens for
/// this name in `ui/src/api.ts` (`onZoomMenu`) and owns the zoom setting.
pub const ZOOM_EVENT: &str = "zoom";

const ZOOM_ITEMS: [(&str, &str, &str); 3] = [
    ("zoom-reset", "Actual Size", "CmdOrCtrl+0"),
    ("zoom-in", "Zoom In", "CmdOrCtrl+="),
    ("zoom-out", "Zoom Out", "CmdOrCtrl+-"),
];

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(app)?;
    let settings = MenuItem::with_id(app, SETTINGS_ID, "Settings…", true, Some("CmdOrCtrl+,"))?;
    // The default app menu starts: About, separator. Settings goes next, in
    // its own group, as in other macOS apps.
    if let Some(MenuItemKind::Submenu(app_menu)) = menu.items()?.into_iter().next() {
        app_menu.insert(&settings, 2)?;
        app_menu.insert(&PredefinedMenuItem::separator(app)?, 3)?;
    }
    // The default View menu has only Enter Full Screen; zooming goes above it.
    for item in menu.items()? {
        if let MenuItemKind::Submenu(view) = item
            && view.text()? == "View"
        {
            for (i, (id, label, keys)) in ZOOM_ITEMS.into_iter().enumerate() {
                view.insert(&MenuItem::with_id(app, id, label, true, Some(keys))?, i)?;
            }
            view.insert(&PredefinedMenuItem::separator(app)?, ZOOM_ITEMS.len())?;
        }
    }
    Ok(menu)
}

pub fn handle<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    // Nothing useful to do if the UI isn't listening yet.
    if event.id() == SETTINGS_ID {
        let _ = app.emit(OPEN_SETTINGS_EVENT, ());
    } else if let Some(id) = event.id().as_ref().strip_prefix("zoom-") {
        let _ = app.emit(ZOOM_EVENT, id);
    }
}
