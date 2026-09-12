use std::fs::File;

use zip::ZipArchive;

use crate::framework::models::Framework;

pub fn detect_framework(
    path: &str,
) -> Framework {

    let file =
        File::open(path)
        .expect("Cannot open APK");

    let mut archive =
        ZipArchive::new(file)
        .expect("Invalid APK");

    for i in 0..archive.len() {

        let file =
            archive.by_index(i).unwrap();

        let name = file.name();

        // Flutter
        if name.contains("libflutter.so") {

            return Framework::Flutter;
        }

        // Unity
        if name.contains("libunity.so") {

            return Framework::Unity;
        }

        // Unreal
        if name.contains("libUE4.so") {

            return Framework::Unreal;
        }

        // React Native
        if name.contains("react") {

            return Framework::ReactNative;
        }

        // Godot
        if name.contains("godot") {

            return Framework::Godot;
        }
    }

    Framework::Unknown
}