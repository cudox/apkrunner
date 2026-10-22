//! Registry of installed applications.
//!
//! M0 only reads the on-disk directory layout; M1 will add `meta.json`
//! and a SQLite index when installation actually copies APKs.

use std::fs;
use std::io;
use std::path::PathBuf;

use crate::paths;

#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub package: String,
    pub versions: Vec<String>,
    pub directory: PathBuf,
}

/// List installed packages, sorted by package name. Missing directories
/// are treated as "nothing installed".
pub fn list_installed() -> io::Result<Vec<InstalledApp>> {
    let dir = paths::apps_dir();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };

    let mut apps = Vec::new();
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let package = entry.file_name().to_string_lossy().into_owned();
        if !paths::is_valid_package(&package) {
            continue;
        }

        let mut versions: Vec<String> = fs::read_dir(entry.path())?
            .filter_map(|v| v.ok())
            .filter(|v| v.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|v| v.file_name().to_string_lossy().into_owned())
            .collect();
        versions.sort();

        apps.push(InstalledApp {
            package,
            versions,
            directory: entry.path(),
        });
    }

    apps.sort_by(|a, b| a.package.cmp(&b.package));
    Ok(apps)
}

/// Remove an installed package and its private data. Refuses invalid
/// package names so it can never remove anything outside the data dir.
pub fn remove_installed(package: &str) -> io::Result<bool> {
    if !paths::is_valid_package(package) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid package name: {package}"),
        ));
    }

    let app = paths::app_dir(package);
    let data = paths::app_data_dir(package);
    let mut existed = false;

    for dir in [&app, &data] {
        match fs::remove_dir_all(dir) {
            Ok(()) => existed = true,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(existed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_package_on_remove() {
        assert!(remove_installed("../../etc").is_err());
        assert!(remove_installed("").is_err());
    }
}
