use std::process::Command;
use crate::apk::models::ApkInspection;

pub fn inspect_apk(apk_path: &str) -> ApkInspection {

    let output = Command::new("aapt")
        .args(["dump", "badging", apk_path])
        .output()
        .expect("aapt not installed");

    let text = String::from_utf8_lossy(&output.stdout);

    let mut package = "unknown".to_string();
    let mut version = "unknown".to_string();
    let mut main_activity = "unknown".to_string();

    let mut min_sdk = "0".to_string();
    let mut target_sdk = "0".to_string();

    let mut has_manifest = false;
    let mut has_kotlin = false;

    let mut has_x86 = false;
    let mut has_x86_64 = false;
    let mut has_arm32 = false;
    let mut has_arm64 = false;

    for line in text.lines() {

        // PACKAGE + VERSION
        if line.contains("package:") {
            has_manifest = true;

            if let Some(i) = line.find("name='") {
                let rest = &line[i + 6..];
                if let Some(end) = rest.find('\'') {
                    package = rest[..end].to_string();
                }
            }

            if let Some(i) = line.find("versionName='") {
                let rest = &line[i + 13..];
                if let Some(end) = rest.find('\'') {
                    version = rest[..end].to_string();
                }
            }
        }

        // MAIN ACTIVITY
        if line.contains("launchable-activity") {
            if let Some(i) = line.find("name='") {
                let rest = &line[i + 6..];
                if let Some(end) = rest.find('\'') {
                    main_activity = rest[..end].to_string();
                }
            }
        }

        // SDK
        if line.contains("sdkVersion:'") {
            if let Some(i) = line.find("sdkVersion:'") {
                let rest = &line[i + 12..];
                if let Some(end) = rest.find('\'') {
                    min_sdk = rest[..end].to_string();
                }
            }
        }

        if line.contains("targetSdkVersion:'") {
            if let Some(i) = line.find("targetSdkVersion:'") {
                let rest = &line[i + 18..];
                if let Some(end) = rest.find('\'') {
                    target_sdk = rest[..end].to_string();
                }
            }
        }

        // FEATURES — check specific arch before generic
        if line.contains("arm64-v8a") {
            has_arm64 = true;
        }

        if line.contains("armeabi-v7a") {
            has_arm32 = true;
        }

        if line.contains("x86_64") {
            has_x86_64 = true;
        } else if line.contains("x86") {
            has_x86 = true;
        }

        if line.contains("kotlin") {
            has_kotlin = true;
        }
    }

    let dex_count = text.matches(".dex").count();

    ApkInspection {
        dex_count,
        has_manifest,
        has_kotlin,
        has_x86,
        has_x86_64,
        has_arm32,
        has_arm64,
        package,
        version,
        main_activity,
        min_sdk,
        target_sdk,
    }
}