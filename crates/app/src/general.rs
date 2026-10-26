//! Settings › General: launch at login, keeping running in the menu bar / tray,
//! notifications, and the shortcut that shows the window.

use serde::Serialize;
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use ts_rs::TS;

use crate::commands::Problem;
use crate::inbox;
use crate::settings::{AppSettings, Settings};

/// The global shortcut that shows the window: ⌥⌘X on macOS, Ctrl+Alt+X elsewhere.
pub const SHOW_SHORTCUT: &str = "CommandOrControl+Alt+X";

/// Wayland leaves global shortcuts to the desktop: an app can't register one.
pub fn is_wayland() -> bool {
    cfg!(target_os = "linux") && std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSettings {
    pub settings: AppSettings,
    pub launch_at_login: bool,
    /// The shortcut, for `keys()` in the UI ("mod+alt+X").
    pub shortcut: String,
    /// Why the shortcut can't be used here, and what to do instead.
    pub shortcut_note: Option<String>,
}

/// The global-shortcut plugin, with the handler that shows the window.
pub fn shortcut_plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                inbox::show_window(app);
            }
        })
        .build()
}

/// Register or drop the show-window shortcut to match the settings; a note when it can't be used.
pub fn apply_shortcut<R: Runtime>(app: &AppHandle<R>, enabled: bool) -> Option<String> {
    if is_wayland() {
        return Some(
            "On Wayland the desktop owns global shortcuts. Add one that runs `xcvr --show` in System Settings › \
             Keyboard › Shortcuts."
                .to_owned(),
        );
    }
    let shortcuts = app.global_shortcut();
    let registered = shortcuts.is_registered(SHOW_SHORTCUT);
    let result = match (enabled, registered) {
        (true, false) => shortcuts.register(SHOW_SHORTCUT),
        (false, true) => shortcuts.unregister(SHOW_SHORTCUT),
        _ => Ok(()),
    };
    result.err().map(|e| format!("Couldn't set up the shortcut: {e}. Another app may be using it."))
}

fn view<R: Runtime>(app: &AppHandle<R>, settings: AppSettings) -> GeneralSettings {
    let shortcut_note = apply_shortcut(app, settings.shortcut_enabled);
    GeneralSettings {
        launch_at_login: app.autolaunch().is_enabled().unwrap_or(false),
        shortcut: "mod+alt+X".into(),
        shortcut_note,
        settings,
    }
}

#[tauri::command]
pub fn general_settings(app: AppHandle, settings: State<'_, Settings>) -> GeneralSettings {
    view(&app, settings.get())
}

#[tauri::command]
pub fn set_general_settings(
    next: AppSettings,
    app: AppHandle,
    settings: State<'_, Settings>,
) -> Result<GeneralSettings, Problem> {
    settings.set(next.clone())?;
    Ok(view(&app, next))
}

/// Start at login, hidden in the menu bar / tray (`--hidden`).
#[tauri::command]
pub fn set_launch_at_login(
    on: bool,
    app: AppHandle,
    settings: State<'_, Settings>,
) -> Result<GeneralSettings, Problem> {
    let autostart = app.autolaunch();
    let result = if on { autostart.enable() } else { autostart.disable() };
    result.map_err(|e| Problem::from(format!("Couldn't change launch at login: {e}")))?;
    Ok(view(&app, settings.get()))
}
