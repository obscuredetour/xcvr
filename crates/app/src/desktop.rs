//! Linux: KWin (and other Wayland desktops) find a window's titlebar and
//! taskbar icon only through the desktop entry named after its app_id,
//! "Transceiver" (see main). Packages install that entry; a dev build or an
//! AppImage has none, so the window shows a generic icon. They write their own
//! to `~/.local/share/applications`, hidden from the launcher:
//!
//! - a dev build's points at the dev binary and the repo's icon;
//! - an AppImage's at the AppImage (`$APPIMAGE`) and a copy of the icon in the
//!   app's data folder (the image's own files are only mounted while it runs).
//!
//! Anything else removes it, so it never hides an installed package's entry,
//! and an entry someone else wrote is left alone.

use std::path::{Path, PathBuf};

const FILE_NAME: &str = "Transceiver.desktop";
/// Mark an entry as one of ours.
const DEV_NAME: &str = "Name=Transceiver (dev)";
const APPIMAGE_NAME: &str = "Name=Transceiver (AppImage)";
/// The icon, for an AppImage's entry to point at.
const ICON: &[u8] = include_bytes!("../icons/128x128@2x.png");

/// Which build this is, as far as its desktop entry goes.
enum Build {
    /// Installed by a package, which has its own entry.
    Packaged,
    /// Run from an AppImage at this path.
    AppImage(PathBuf),
    /// A dev build: this binary.
    Dev(PathBuf),
    /// A release build run some other way (from a target folder, say).
    Other,
}

/// Write, update or remove the entry. `data_dir` is `~/.local/share`; `app_dir` the
/// app's own data folder there, for an AppImage's copy of the icon.
pub fn sync_entry(data_dir: &Path, app_dir: &Path) -> std::io::Result<()> {
    let system_dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    let packaged = system_dirs.split(':').any(|d| Path::new(d).join("applications").join(FILE_NAME).is_file());
    let appimage = std::env::var_os("APPIMAGE").map(PathBuf::from).filter(|p| p.is_file());
    let build = if packaged {
        Build::Packaged
    } else if let Some(path) = appimage {
        Build::AppImage(path)
    } else if cfg!(debug_assertions) {
        Build::Dev(std::env::current_exe()?)
    } else {
        Build::Other
    };
    sync(data_dir, app_dir, &build)
}

fn sync(data_dir: &Path, app_dir: &Path, build: &Build) -> std::io::Result<()> {
    let path = data_dir.join("applications").join(FILE_NAME);
    let existing = std::fs::read_to_string(&path).ok();
    let ours = existing.as_ref().is_some_and(|e| e.contains(DEV_NAME) || e.contains(APPIMAGE_NAME));

    let (name, exe, icon) = match build {
        Build::Packaged | Build::Other => {
            if ours {
                std::fs::remove_file(&path)?;
            }
            return Ok(());
        }
        Build::Dev(exe) => (DEV_NAME, exe.clone(), Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/128x128@2x.png")),
        Build::AppImage(appimage) => {
            let icon = app_dir.join("Transceiver.png");
            if std::fs::read(&icon).ok().as_deref() != Some(ICON) {
                std::fs::create_dir_all(app_dir)?;
                std::fs::write(&icon, ICON)?;
            }
            (APPIMAGE_NAME, appimage.clone(), icon)
        }
    };
    // Someone's own entry: leave it alone.
    if existing.is_some() && !ours {
        return Ok(());
    }
    let entry = format!(
        "[Desktop Entry]\nType=Application\n{name}\nExec=\"{}\" %U\nIcon={}\n\
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_for_each_build() {
        let root = std::env::temp_dir().join(format!("xcvr-desktop-{}", std::process::id()));
        let (data, app) = (root.join("share"), root.join("share/com.example.app"));
        let entry = data.join("applications").join(FILE_NAME);
        let read = || std::fs::read_to_string(&entry).ok();

        // An AppImage: an entry pointing at it, and the icon copied out of it.
        let appimage = root.join("Transceiver.AppImage");
        sync(&data, &app, &Build::AppImage(appimage.clone())).unwrap();
        let written = read().unwrap();
        assert!(written.contains(APPIMAGE_NAME));
        assert!(written.contains(&format!("Exec=\"{}\" %U", appimage.display())));
        assert!(written.contains(&format!("Icon={}", app.join("Transceiver.png").display())));
        assert!(written.contains("NoDisplay=true"));
        assert_eq!(std::fs::read(app.join("Transceiver.png")).unwrap(), ICON);

        // Moved: the entry follows it.
        let moved = root.join("Apps/Transceiver.AppImage");
        sync(&data, &app, &Build::AppImage(moved.clone())).unwrap();
        assert!(read().unwrap().contains(&moved.display().to_string()));

        // A package installed since: ours goes, so it doesn't hide the package's.
        sync(&data, &app, &Build::Packaged).unwrap();
        assert_eq!(read(), None);

        // Someone's own entry stays, whatever runs.
        std::fs::write(&entry, "[Desktop Entry]\nName=Mine\n").unwrap();
        sync(&data, &app, &Build::AppImage(appimage)).unwrap();
        sync(&data, &app, &Build::Other).unwrap();
        assert_eq!(read().as_deref(), Some("[Desktop Entry]\nName=Mine\n"));

        std::fs::remove_dir_all(root).unwrap();
    }
}
