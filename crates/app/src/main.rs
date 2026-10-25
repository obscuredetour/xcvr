#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
#[cfg(target_os = "macos")]
mod menu;
mod poller;
mod profiles;
mod secrets;

use std::sync::Arc;

use tauri::Manager;

use crate::poller::{Poller, Target};
use crate::profiles::Profiles;
use crate::secrets::Secrets;

fn main() {
    // GTK names the window after the program (Wayland app_id, X11 WM_CLASS), and desktops find
    // its icon through the desktop entry of that name. The bundles ship "Transceiver.desktop",
    // after productName, so the window takes that name rather than the binary's "xcvr".
    #[cfg(target_os = "linux")]
    glib::set_prgname(Some("Transceiver"));

    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.menu(menu::build).on_menu_event(menu::handle);

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            let secrets = Arc::new(Secrets::new(dir.join("secrets.json")));
            let profiles = Profiles::load(dir.join("connections.json"));
            let active = profiles.active();
            app.manage(secrets.clone());
            app.manage(profiles);
            app.manage(Poller::start(app.handle().clone()));

            // Connect to the saved host. Its password comes from the keychain, off the
            // main thread (a locked keyring may ask to be unlocked).
            if let Some(profile) = active {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    let password = secrets.get(&profile.id).ok().flatten();
                    let target = Target { label: profile.label(), config: profile.config(password) };
                    handle.state::<Poller>().connect(Some(target));
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connection_settings,
            commands::save_connection,
            commands::remove_connection,
            commands::test_connection,
            commands::connection_state,
            commands::torrents,
            commands::session_stats,
            commands::poll_now,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Transceiver");
}
