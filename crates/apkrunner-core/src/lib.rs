//! Core types shared by the CLI and (from M1 on) the runtime supervisor.

pub mod installed;
pub mod paths;

pub use installed::{list_installed, remove_installed, InstalledApp};
