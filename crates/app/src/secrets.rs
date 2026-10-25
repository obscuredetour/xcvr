//! RPC passwords: in the system keychain (the macOS Keychain; the Secret Service
//! on Linux, i.e. KWallet or GNOME Keyring), filed under the app's identifier with
//! the connection's id as the account. Where no keychain is reachable, in
//! `secrets.json` beside the settings, readable only by its owner, and Settings
//! says so.
//!
//! Only touched off the main thread (commands call in through
//! `spawn_blocking`): a locked keyring may ask the user to unlock it.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

/// The keychain service every secret is filed under (the bundle identifier).
pub const SERVICE: &str = "com.obscuredetour.xcvr";

const PLACE: &str = if cfg!(target_os = "macos") { "the macOS Keychain" } else { "the system keychain" };

pub struct Secrets {
    /// The fallback file.
    file: PathBuf,
    /// Whether the keychain is reachable, checked on first use.
    keychain: OnceLock<Result<(), String>>,
}

impl Secrets {
    pub fn new(file: PathBuf) -> Self {
        Self { file, keychain: OnceLock::new() }
    }

    fn keychain(&self) -> Result<(), &str> {
        self.keychain
            .get_or_init(|| keyring::Entry::store_status().as_ref().map(|_| ()).map_err(|e| e.to_string()))
            .as_ref()
            .map(|_| ())
            .map_err(String::as_str)
    }

    /// Where passwords are kept, for a sentence in Settings.
    pub fn place(&self) -> String {
        match self.keychain() {
            Ok(()) => PLACE.to_owned(),
            Err(why) => format!("a file only you can read (no keychain: {why})"),
        }
    }

    pub fn get(&self, account: &str) -> Result<Option<String>, String> {
        if self.keychain().is_err() {
            return Ok(self.read_file().remove(account));
        }
        match entry(account)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn set(&self, account: &str, secret: &str) -> Result<(), String> {
        if self.keychain().is_err() {
            let mut all = self.read_file();
            all.insert(account.to_owned(), secret.to_owned());
            return self.write_file(&all);
        }
        entry(account)?.set_password(secret).map_err(|e| e.to_string())
    }

    /// Nothing to delete is fine.
    pub fn delete(&self, account: &str) -> Result<(), String> {
        if self.keychain().is_err() {
            let mut all = self.read_file();
            return if all.remove(account).is_some() { self.write_file(&all) } else { Ok(()) };
        }
        match entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn read_file(&self) -> BTreeMap<String, String> {
        fs::read(&self.file).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
    }

    fn write_file(&self, all: &BTreeMap<String, String>) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(all).map_err(|e| e.to_string())?;
        if let Some(dir) = self.file.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        write_private(&self.file, &json).map_err(|e| format!("Couldn't save the password: {e}"))
    }
}

fn entry(account: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, account).map_err(|e| e.to_string())
}

/// Write `bytes` to `path`, readable and writable by its owner only.
fn write_private(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(path)?.write_all(bytes)
}
