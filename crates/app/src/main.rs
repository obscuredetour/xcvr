#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod commands;
#[cfg(target_os = "linux")]
mod desktop;
mod general;
mod inbox;
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
mod settings;
mod tray;

use std::path::Path;
use std::sync::Arc;

use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_deep_link::DeepLinkExt;

use crate::inbox::Inbox;
use crate::poller::{Poller, Target};
use crate::profiles::Profiles;
use crate::secrets::Secrets;
use crate::settings::Settings;

/// Launch at login starts with this: straight into the menu bar / tray, no window.
const HIDDEN_ARG: &str = "--hidden";

fn main() {
    // GTK names the window after the program (Wayland app_id, X11 WM_CLASS), and desktops find
    // its icon through the desktop entry of that name. The bundles ship "Transceiver.desktop",
    // after productName, so the window takes that name rather than the binary's "xcvr".
    #[cfg(target_os = "linux")]
    glib::set_prgname(Some("Transceiver"));

    // First: a second launch (a magnet clicked, a .torrent opened, `xcvr --show`) hands
    // its arguments to this one and exits.
    let builder = tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
        inbox::receive(app, inbox::parse_args(&args[1.min(args.len())..], Path::new(&cwd)));
    }));
    #[cfg(target_os = "macos")]
    let builder = builder.menu(menu::build).on_menu_event(menu::handle);

    let app = builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![HIDDEN_ARG])))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(general::shortcut_plugin())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            let secrets = Arc::new(Secrets::new(dir.join("secrets.json")));
            let profiles = Profiles::load(dir.join("connections.json"));
            let active = profiles.active();
            let settings = Settings::load(dir.join("app.json"));
            let shortcut_enabled = settings.get().shortcut_enabled;
            app.manage(secrets.clone());
            app.manage(profiles);
            app.manage(settings);
            app.manage(Inbox::default());
            app.manage(mappings::Mappings::load(dir.join("paths.json")));
            app.manage(originals::Originals::load(dir.join("originals.json")));
            app.manage(Poller::start(app.handle().clone()));
            tray::build(app.handle())?;
            general::apply_shortcut(app.handle(), shortcut_enabled);

            // Linux: dev builds need their own desktop entry for the titlebar icon (packages ship one).
            #[cfg(target_os = "linux")]
            if let Ok(data) = app.path().data_dir() {
                let _ = desktop::sync_dev_entry(&data);
            }

            // The window starts hidden (tauri.conf.json): shown now, unless launched at login.
            let args: Vec<String> = std::env::args().skip(1).collect();
            if !args.iter().any(|a| a == HIDDEN_ARG) {
                inbox::show_window(app.handle());
            }
            // Started by opening a .torrent file or a magnet (Linux passes them as arguments).
            let cwd = std::env::current_dir().unwrap_or_default();
            inbox::receive(app.handle(), inbox::parse_args(&args, &cwd));

            // macOS: links and files opened while running, or that started it, as URLs
            // (magnet:, and file:// for .torrent files).
            // (The inbox drops repeats, so one seen both here and as an event is added once.)
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                let urls: Vec<String> = urls.into_iter().map(|u| u.to_string()).collect();
                inbox::receive(app.handle(), inbox::parse_args(&urls, Path::new("/")));
            }
            let handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                let urls: Vec<String> = event.urls().into_iter().map(|u| u.to_string()).collect();
                inbox::receive(&handle, inbox::parse_args(&urls, Path::new("/")));
            });
            // Linux dev builds: register magnet: for this binary (packages ship a desktop entry).
            #[cfg(all(debug_assertions, target_os = "linux"))]
            let _ = app.deep_link().register_all();

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
        // Closing the window keeps the app in the menu bar / tray (Settings › General), if it has one.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.app_handle().state::<Settings>().get().keep_running
                && tray::is_shown(window.app_handle())
            {
                api.prevent_close();
                let _ = window.hide();
            }
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
            inspect::set_location,
            inspect::path_mappings,
            inspect::set_path_mappings,
            inspect::torrents_folder,
            inspect::open_torrent,
            labels::edit_labels,
            labels::rename_label,
            labels::delete_label,
            prefs::session_settings,
            prefs::set_session,
            prefs::session_brief,
            prefs::test_port,
            prefs::update_blocklist,
            inbox::take_inbox,
            general::general_settings,
            general::set_general_settings,
            general::set_launch_at_login,
        ])
        .build(tauri::generate_context!())
        .expect("error while starting Transceiver");

    app.run(|app, event| {
        // macOS: clicking the Dock icon brings a hidden window back.
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = event {
            inbox::show_window(app);
        }
        let _ = (app, event);
    });
}
