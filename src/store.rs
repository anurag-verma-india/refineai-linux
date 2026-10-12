//! Small JSON-file persistence shared by `config` and `history`. Both keep a
//! single pretty-printed JSON file under `refineai/` in a standard base dir;
//! this module centralizes the path building, read-or-default, and atomic-ish
//! write (create parents, serialize, write, optionally lock down perms).

use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Build `<base>/refineai/<name>`, falling back to the current directory when
/// the base dir can't be resolved (e.g. `dirs::config_dir()` returned `None`).
pub fn app_path(base: Option<PathBuf>, name: &str) -> PathBuf {
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("refineai")
        .join(name)
}

/// Read and deserialize a JSON file, returning `T::default()` when the file is
/// missing, unreadable, or malformed.
pub fn read<T: DeserializeOwned + Default>(path: &Path) -> T {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Serialize `value` as pretty JSON and write it to `path`, creating parent
/// directories first. When `private`, restrict the file to the owner (`0600` on
/// Unix) — used for the config, which holds the API key.
pub fn write<T: Serialize + ?Sized>(path: &Path, value: &T, private: bool) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(value).unwrap_or_default();
    fs::write(path, json)?;
    if private {
        set_private(path);
    }
    Ok(())
}

#[cfg(unix)]
fn set_private(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn set_private(_path: &Path) {}
