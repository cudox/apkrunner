//! Runtime supervisor: backend discovery, process launch and log capture.

pub mod atl;

pub use atl::AtlBackend;

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use apkrunner_core::backend::{BackendError, LaunchRequest, RuntimeBackend};

/// Launch an app and wait for it, streaming the runtime's stderr into
/// `log_path` (and to our stderr). Returns the child exit code.
pub fn run(
    backend: &dyn RuntimeBackend,
    request: &LaunchRequest,
    log_path: &Path,
) -> Result<i32, BackendError> {
    backend.probe()?;
    let launch = backend.launch(request)?;

    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let log = Arc::new(Mutex::new(File::create(log_path)?));

    let mut command = Command::new(&launch.program);
    command
        .args(&launch.args)
        .envs(launch.env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped());

    let mut child = command.spawn()?;

    let pump_thread = child.stderr.take().map(|stderr| {
        let log = Arc::clone(&log);
        std::thread::spawn(move || pump(stderr, log))
    });

    let status = child.wait()?;

    // The child is gone, so the pipe is closed: drain it before returning or
    // the last lines never reach the log file.
    if let Some(thread) = pump_thread {
        let _ = thread.join();
    }

    Ok(status.code().unwrap_or(1))
}

fn pump(mut input: impl Read, log: Arc<Mutex<File>>) {
    let mut buffer = [0u8; 4096];
    loop {
        match input.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                let chunk = &buffer[..n];
                let _ = std::io::stderr().write_all(chunk);
                let _ = std::io::stderr().flush();
                if let Ok(mut file) = log.lock() {
                    let _ = file.write_all(chunk);
                }
            }
            Err(_) => break,
        }
    }
}

/// Default location used by the build script; overridable for tests.
pub fn default_prefix() -> PathBuf {
    if let Some(prefix) = std::env::var_os("APKRUNNER_RUNTIME_PREFIX") {
        return PathBuf::from(prefix);
    }
    let home = std::env::var_os("HOME").unwrap_or_else(|| ".".into());
    PathBuf::from(home).join(".local/apkrunner-runtime")
}
