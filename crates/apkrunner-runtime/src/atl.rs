//! ATL ("Android Translation Layer") backend.
//!
//! ATL is the M1 engine: ART compiled for glibc plus an implementation of
//! `android.*` on GTK4, with no container and no Android image at runtime.

use std::env;
use std::path::{Path, PathBuf};

use apkrunner_core::backend::{BackendError, LaunchCommand, LaunchRequest, RuntimeBackend};

pub struct AtlBackend {
    binary: PathBuf,
    prefix: PathBuf,
}

impl AtlBackend {
    /// Find `android-translation-layer`, in this order:
    /// `APKRUNNER_ATL_BIN`, `$PATH`, the local build prefix, `/usr/bin`.
    pub fn discover() -> Result<Self, BackendError> {
        let prefix = crate::default_prefix();

        if let Some(path) = env::var_os("APKRUNNER_ATL_BIN") {
            let binary = PathBuf::from(path);
            if !binary.is_file() {
                return Err(BackendError::NotFound(format!(
                    "APKRUNNER_ATL_BIN points to {}, which is not a file",
                    binary.display()
                )));
            }
            return Ok(Self { binary, prefix });
        }

        let mut candidates = Vec::new();
        if let Some(path) = env::var_os("PATH") {
            candidates
                .extend(env::split_paths(&path).map(|dir| dir.join("android-translation-layer")));
        }
        candidates.push(prefix.join("bin/android-translation-layer"));
        candidates.push(PathBuf::from("/usr/bin/android-translation-layer"));
        candidates.push(PathBuf::from("/usr/local/bin/android-translation-layer"));

        for candidate in candidates {
            if candidate.is_file() {
                return Ok(Self {
                    binary: candidate,
                    prefix,
                });
            }
        }

        Err(BackendError::NotFound(
            "android-translation-layer is not installed. Build it with \
             scripts/build-runtime.sh or set APKRUNNER_ATL_BIN."
                .to_string(),
        ))
    }

    pub fn binary(&self) -> &Path {
        &self.binary
    }

    pub fn prefix(&self) -> &Path {
        &self.prefix
    }

    fn library_path(&self) -> Option<String> {
        let lib = self.prefix.join("lib");
        if !lib.is_dir() {
            return None;
        }
        let mut paths = vec![lib];
        if let Some(existing) = env::var_os("LD_LIBRARY_PATH") {
            paths.extend(env::split_paths(&existing));
        }
        env::join_paths(paths)
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    }
}

impl RuntimeBackend for AtlBackend {
    fn name(&self) -> &'static str {
        "ATL (ART + GTK4, no container)"
    }

    fn probe(&self) -> Result<(), BackendError> {
        if !self.binary.is_file() {
            return Err(BackendError::NotFound(format!(
                "{} does not exist",
                self.binary.display()
            )));
        }
        Ok(())
    }

    fn launch(&self, request: &LaunchRequest) -> Result<LaunchCommand, BackendError> {
        let mut args = vec![
            request.apk.display().to_string(),
            "-l".to_string(),
            request.activity.clone(),
        ];

        if let Some(width) = request.width {
            args.extend(["-w".to_string(), width.to_string()]);
        }
        if let Some(height) = request.height {
            args.extend(["-h".to_string(), height.to_string()]);
        }
        args.extend(request.extra_args.iter().cloned());

        let mut env = vec![
            (
                "ANDROID_APP_DATA_DIR".to_string(),
                request.data_dir.display().to_string(),
            ),
            ("APKRUNNER_PACKAGE".to_string(), request.package.clone()),
        ];

        if let Some(library_path) = self.library_path() {
            env.push(("LD_LIBRARY_PATH".to_string(), library_path));
        }

        // GTK/EGL on X11: force EGL, otherwise GTK may pick GLX and break
        // ATL's EGL-based rendering.
        if env::var_os("GDK_DEBUG").is_none()
            && env::var("XDG_SESSION_TYPE").as_deref() == Ok("x11")
        {
            env.push(("GDK_DEBUG".to_string(), "gl-egl".to_string()));
        }

        Ok(LaunchCommand {
            program: self.binary.clone(),
            args,
            env,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let unique = format!(
            "apkrunner-atl-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(unique);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn request() -> LaunchRequest {
        LaunchRequest {
            apk: PathBuf::from("/tmp/app.apk"),
            package: "com.example.hello".to_string(),
            activity: "com.example.hello.MainActivity".to_string(),
            data_dir: PathBuf::from("/tmp/apkrunner-data/com.example.hello"),
            width: Some(540),
            height: Some(960),
            extra_args: vec!["-X".to_string(), "-Xusejit:false".to_string()],
        }
    }

    #[test]
    fn builds_the_expected_command_line() {
        let dir = temp_dir();
        let binary = dir.join("android-translation-layer");
        fs::write(&binary, b"#!/bin/sh\n").unwrap();

        let backend = AtlBackend {
            binary: binary.clone(),
            prefix: dir.clone(),
        };
        let command = backend.launch(&request()).unwrap();

        assert_eq!(command.program, binary);
        assert_eq!(
            command.args,
            vec![
                "/tmp/app.apk",
                "-l",
                "com.example.hello.MainActivity",
                "-w",
                "540",
                "-h",
                "960",
                "-X",
                "-Xusejit:false",
            ]
        );

        let env: std::collections::HashMap<_, _> = command.env.into_iter().collect();
        assert_eq!(
            env.get("ANDROID_APP_DATA_DIR").map(String::as_str),
            Some("/tmp/apkrunner-data/com.example.hello")
        );
        assert_eq!(
            env.get("APKRUNNER_PACKAGE").map(String::as_str),
            Some("com.example.hello")
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn probe_rejects_a_missing_binary() {
        let backend = AtlBackend {
            binary: PathBuf::from("/nonexistent/atl"),
            prefix: PathBuf::from("/tmp"),
        };
        assert!(matches!(backend.probe(), Err(BackendError::NotFound(_))));
    }
}
