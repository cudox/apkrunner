//! APK inspection: ZIP layout, binary `AndroidManifest.xml`, ABIs and
//! native libraries. No external tools (`aapt`, `unzip`) required.

pub mod axml;
pub mod error;
pub mod extract;
#[cfg(test)]
mod fixtures;
pub mod inspect;
pub mod model;

pub use error::{Error, Result};
pub use inspect::inspect_apk;
pub use model::{Activity, ApkInfo, DexEntry, Framework};
