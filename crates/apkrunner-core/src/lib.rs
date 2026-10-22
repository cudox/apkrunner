//! Core types shared by the CLI and (from M1 on) the runtime supervisor.

pub mod backend;
pub mod installed;
pub mod paths;

pub use backend::{BackendError, LaunchCommand, LaunchRequest, RuntimeBackend};
pub use installed::{list_installed, remove_installed, InstalledApp};
