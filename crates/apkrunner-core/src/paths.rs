//! Shared paths and installed-app bookkeeping.
//!
//! Everything lives under XDG directories. `APKRUNNER_DATA_DIR` overrides
//! the root, which is what the integration tests use.

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Root of apkrunner's persistent data.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = env::var_os("APKRUNNER_DATA_DIR") {
        return PathBuf::from(dir);
    }
    xdg_dir("XDG_DATA_HOME", ".local/share").join("apkrunner")
}

/// Logs and other throwaway state.
pub fn state_dir() -> PathBuf {
    if let Some(dir) = env::var_os("APKRUNNER_STATE_DIR") {
        return PathBuf::from(dir);
    }
    xdg_dir("XDG_STATE_HOME", ".local/state").join("apkrunner")
}

pub fn apps_dir() -> PathBuf {
    data_dir().join("apps")
}

pub fn app_dir(package: &str) -> PathBuf {
    apps_dir().join(package)
}

pub fn app_version_dir(package: &str, version: &str) -> PathBuf {
    app_dir(package).join(version)
}

/// Per-app private storage (the equivalent of `/data/data/<package>`).
pub fn app_data_dir(package: &str) -> PathBuf {
    data_dir().join("data").join(package)
}

/// Shared storage visible to apps and the user (`/sdcard` equivalent).
pub fn shared_dir() -> PathBuf {
    data_dir().join("shared")
}

pub fn log_path(package: &str) -> PathBuf {
    state_dir().join(format!("{package}.log"))
}

/// Create the basic directory layout if missing.
pub fn ensure_layout() -> io::Result<()> {
    fs::create_dir_all(apps_dir())?;
    fs::create_dir_all(data_dir().join("data"))?;
    fs::create_dir_all(shared_dir())?;
    fs::create_dir_all(state_dir())?;
    Ok(())
}

/// Android package names: dot-separated segments of ASCII letters, digits
/// and underscores. Validating matters because the name becomes a path.
pub fn is_valid_package(package: &str) -> bool {
    if package.is_empty() || package.len() > 255 || package.starts_with('.') {
        return false;
    }
    if package.split('.').any(|segment| {
        segment.is_empty()
            || !segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
    }) {
        return false;
    }
    // Android requires at least one dot, but be lenient with fixtures.
    true
}

fn xdg_dir(variable: &str, fallback: &str) -> PathBuf {
    match env::var_os(variable) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => {
            let home = env::var_os("HOME").unwrap_or_else(|| ".".into());
            PathBuf::from(home).join(fallback)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_packages() {
        assert!(is_valid_package("com.example.hello"));
        assert!(is_valid_package("a.b_c.d1"));
        assert!(!is_valid_package(""));
        assert!(!is_valid_package(".."));
        assert!(!is_valid_package("com..example"));
        assert!(!is_valid_package("com.example/evil"));
        assert!(!is_valid_package("com example"));
    }
}
