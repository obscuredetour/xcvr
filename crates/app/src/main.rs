#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
mod menu;

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
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running Transceiver");
}
