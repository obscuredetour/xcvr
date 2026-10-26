#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod commands;
mod inspect;
mod labels;
mod mappings;
#[cfg(target_os = "macos")]
mod menu;
mod originals;
mod poller;
mod prefs;
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
            app.manage(mappings::Mappings::load(dir.join("paths.json")));
            app.manage(originals::Originals::load(dir.join("originals.json")));
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
            actions::torrent_action,
            actions::remove_torrents,
            actions::magnet_links,
            actions::read_torrent_file,
            actions::add_torrents,
            actions::add_defaults,
            actions::free_space,
            inspect::inspect,
            inspect::set_torrents,
            inspect::rename_torrent,
            inspect::original_name,
            labels::edit_labels,
            labels::rename_label,
            labels::delete_label,
            prefs::session_settings,
            prefs::set_session,
            prefs::session_brief,
            prefs::test_port,
            prefs::update_blocklist,
            inspect::set_location,
            inspect::path_mappings,
            inspect::set_path_mappings,
            inspect::open_torrent,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Transceiver");
}
