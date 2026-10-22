use std::fs::File;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Result};

/// Join an archive entry name to `output`, refusing anything that could
/// escape it (zip-slip): absolute paths, `..`, drive prefixes, NUL bytes
/// and Windows separators.
pub fn safe_join(output: &Path, entry: &str) -> Result<PathBuf> {
    if entry.is_empty() || entry.contains('\0') || entry.contains('\\') {
        return Err(Error::UnsafeEntry(entry.to_string()));
    }

    let mut path = output.to_path_buf();
    let mut components = 0usize;

    for component in Path::new(entry).components() {
        match component {
            Component::Normal(part) => {
                path.push(part);
                components += 1;
            }
            _ => return Err(Error::UnsafeEntry(entry.to_string())),
        }
    }

    if components == 0 {
        return Err(Error::UnsafeEntry(entry.to_string()));
    }
    Ok(path)
}

/// Extract every regular file of the APK under `output`.
/// Symlinks and unsafe names are rejected, not followed.
pub fn extract_apk(apk: &Path, output: &Path) -> Result<Vec<PathBuf>> {
    let file = File::open(apk)?;
    let mut archive = zip::ZipArchive::new(file)?;
    std::fs::create_dir_all(output)?;

    let mut written = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();

        if entry.is_dir() || name.ends_with('/') {
            continue;
        }

        if is_symlink(&entry) {
            return Err(Error::UnsafeEntry(name));
        }

        let target = safe_join(output, &name)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut out = File::create(&target)?;
        io::copy(&mut entry, &mut out)?;
        written.push(target);
    }

    Ok(written)
}

fn is_symlink(entry: &zip::read::ZipFile<'_>) -> bool {
    matches!(entry.unix_mode(), Some(mode) if mode & 0o170_000 == 0o120_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_entries() {
        let out = Path::new("/tmp/out");
        assert_eq!(
            safe_join(out, "lib/x86_64/libfoo.so").unwrap(),
            PathBuf::from("/tmp/out/lib/x86_64/libfoo.so")
        );
        assert_eq!(
            safe_join(out, "classes.dex").unwrap(),
            PathBuf::from("/tmp/out/classes.dex")
        );
    }

    #[test]
    fn rejects_traversal() {
        let out = Path::new("/tmp/out");
        for evil in [
            "../evil",
            "a/../../evil",
            "/etc/passwd",
            "..",
            ".",
            "",
            "C:\\evil",
            "a\\..\\..\\evil",
            "a/\0b",
        ] {
            assert!(
                safe_join(out, evil).is_err(),
                "entry {evil:?} should be rejected"
            );
        }
    }
}
