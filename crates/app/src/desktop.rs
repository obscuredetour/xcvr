//! Linux: KWin (and other Wayland desktops) find a window's titlebar icon only
//! through the desktop entry named after its app_id, "Transceiver" (see main).
//! Packages install that entry; a dev build has none, so the titlebar shows a
//! generic icon. Dev builds write one to `~/.local/share/applications`, hidden
//! from the launcher, pointing at the dev binary and the repo's icon. Anything
//! else removes it, so it never hides an installed package's entry.

use std::path::Path;

const FILE_NAME: &str = "Transceiver.desktop";
/// Marks the entry as the dev build's own.
const DEV_NAME: &str = "Name=Transceiver (dev)";

pub fn sync_dev_entry(data_dir: &Path) -> std::io::Result<()> {
    let path = data_dir.join("applications").join(FILE_NAME);
    let system_dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    let packaged = system_dirs.split(':').any(|d| Path::new(d).join("applications").join(FILE_NAME).is_file());
    let existing = std::fs::read_to_string(&path).ok();

    if cfg!(not(debug_assertions)) || packaged {
        if existing.is_some_and(|e| e.contains(DEV_NAME)) {
            std::fs::remove_file(&path)?;
        }
        return Ok(());
    }
    // Someone's own entry: leave it alone.
    if existing.as_ref().is_some_and(|e| !e.contains(DEV_NAME)) {
        return Ok(());
    }
    let exe = std::env::current_exe()?;
    let icon = Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/128x128@2x.png");
    let entry = format!(
        "[Desktop Entry]\nType=Application\n{DEV_NAME}\nExec=\"{}\" %U\nIcon={}\n\
         StartupWMClass=Transceiver\nNoDisplay=true\nTerminal=false\n",
        exe.display(),
        icon.display()
    );
    if existing.as_deref() != Some(entry.as_str()) {
        std::fs::create_dir_all(data_dir.join("applications"))?;
        std::fs::write(&path, entry)?;
    }
    Ok(())
}
