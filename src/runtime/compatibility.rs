use crate::apk::models::{
    ApkInfo,
    ApkInspection,
};

use crate::runtime::backend::{
    RuntimeBackend,
    AndroidRuntime,
};

pub fn detect_backend(
    inspection: &ApkInspection,
) -> RuntimeBackend {

    if inspection.has_x86_64 {

        RuntimeBackend::NativeX86_64

    } else if inspection.has_x86 {

        RuntimeBackend::NativeX86

    } else {

        RuntimeBackend::ArmTranslation
    }
}

pub fn detect_android_runtime(
    info: &ApkInfo,
) -> AndroidRuntime {

    let sdk =
        info.target_sdk
        .parse::<u32>()
        .unwrap_or(0);

    if sdk >= 30 {

        AndroidRuntime::ModernArt

    } else {

        AndroidRuntime::LegacyDalvik
    }
}