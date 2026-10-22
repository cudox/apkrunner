//! Runtime backend abstraction.
//!
//! The CLI never talks to ART directly: it asks a backend to build a launch
//! command. M1 ships the ATL backend; later milestones can add a native one
//! behind the same trait.

use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct LaunchRequest {
    pub apk: PathBuf,
    pub package: String,
    pub activity: String,
    /// Per-app data root handed to the runtime.
    pub data_dir: PathBuf,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LaunchCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

pub trait RuntimeBackend {
    /// Human-readable backend name, shown by `apkrunner doctor`.
    fn name(&self) -> &'static str;

    /// Check that the backend can actually run an app.
    fn probe(&self) -> Result<(), BackendError>;

    /// Build the command line for one app launch.
    fn launch(&self, request: &LaunchRequest) -> Result<LaunchCommand, BackendError>;
}

#[derive(Debug)]
pub enum BackendError {
    /// The runtime binary or one of its files is missing.
    NotFound(String),
    /// The backend exists but cannot run this APK.
    Unsupported(String),
    Io(std::io::Error),
    Other(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::NotFound(m) => write!(f, "runtime not found: {m}"),
            BackendError::Unsupported(m) => write!(f, "unsupported: {m}"),
            BackendError::Io(e) => write!(f, "runtime I/O error: {e}"),
            BackendError::Other(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for BackendError {}

impl From<std::io::Error> for BackendError {
    fn from(e: std::io::Error) -> Self {
        BackendError::Io(e)
    }
}
