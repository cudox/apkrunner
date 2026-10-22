use std::cmp::Ordering;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use zip::ZipArchive;

use crate::axml;
use crate::error::{Error, Result};
use crate::model::{Activity, ApkInfo, DexEntry, Framework};

const MAX_MANIFEST_SIZE: u64 = 16 * 1024 * 1024;
const ABI_ORDER: [&str; 6] = [
    "arm64-v8a",
    "armeabi-v7a",
    "armeabi",
    "x86_64",
    "x86",
    "riscv64",
];

/// Inspect an APK without extracting it.
pub fn inspect_apk(path: &Path) -> Result<ApkInfo> {
    let file = File::open(path)?;
    let file_size = file.metadata()?.len();
    let mut archive =
        ZipArchive::new(file).map_err(|e| Error::InvalidApk(format!("{}: {e}", path.display())))?;

    let manifest = read_manifest(&mut archive)?;
    let root = axml::parse(&manifest)?;

    if root.name != "manifest" {
        return Err(Error::Manifest(format!(
            "root element is <{}>, expected <manifest>",
            root.name
        )));
    }

    let package = root
        .attr_str("package")
        .filter(|p| !p.is_empty())
        .ok_or_else(|| Error::Manifest("missing package attribute".into()))?
        .to_string();

    let version_name = root.attr_str("versionName").map(str::to_string);
    let version_code = root
        .attr_u32("versionCode")
        .or_else(|| root.attr_str("versionCode").and_then(|s| s.parse().ok()));

    let sdk = root.child("uses-sdk");
    let min_sdk = sdk.and_then(|e| e.attr_u32("minSdkVersion")).or_else(|| {
        sdk.and_then(|e| e.attr_str("minSdkVersion"))
            .and_then(|s| s.parse().ok())
    });
    let target_sdk = sdk
        .and_then(|e| e.attr_u32("targetSdkVersion"))
        .or_else(|| {
            sdk.and_then(|e| e.attr_str("targetSdkVersion"))
                .and_then(|s| s.parse().ok())
        });

    let application = root.child("application");
    let label = application
        .and_then(|a| literal_string(a, "label"))
        .map(str::to_string);
    let icon = application.and_then(|a| resource_string(a, "icon"));
    let debuggable = application
        .and_then(|a| a.attr_bool("debuggable"))
        .unwrap_or(false);

    let mut activities = Vec::new();
    if let Some(app) = application {
        for node in app
            .children_named("activity")
            .chain(app.children_named("activity-alias"))
        {
            let Some(name) = node.attr_str("name") else {
                continue;
            };
            let name = qualify(&package, name);
            let class_name = node
                .attr_str("targetActivity")
                .map(|t| qualify(&package, t))
                .unwrap_or_else(|| name.clone());
            let is_launcher = has_launcher_intent(node);
            activities.push(Activity {
                name,
                class_name,
                exported: node.attr_bool("exported"),
                label: literal_string(node, "label").map(str::to_string),
                is_launcher,
            });
        }
    }

    let launchable_activity = activities
        .iter()
        .find(|a| a.is_launcher)
        .map(|a| a.class_name.clone());

    let permissions = root
        .children_named("uses-permission")
        .filter_map(|p| p.attr_str("name").map(str::to_string))
        .collect();

    let mut abis = Vec::new();
    let mut native_libs = Vec::new();
    let mut dex_files = Vec::new();
    let mut flags = FrameworkFlags::default();

    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        let size = entry.size();

        if is_dex(&name) {
            dex_files.push(DexEntry { name, size });
            continue;
        }

        if let Some(abi) = native_abi(&name) {
            if !abis.iter().any(|a: &String| a == abi) {
                abis.push(abi.to_string());
            }
            native_libs.push(name.clone());
        }

        flags.observe(&name);
    }

    abis.sort_by(|a, b| abi_rank(a).cmp(&abi_rank(b)).then(a.cmp(b)));
    dex_files.sort_by(|a, b| natural_dex_order(&a.name, &b.name));
    native_libs.sort();

    Ok(ApkInfo {
        path: path.display().to_string(),
        file_size,
        package,
        version_name,
        version_code,
        min_sdk,
        target_sdk,
        label,
        icon,
        launchable_activity,
        activities,
        permissions,
        abis,
        dex_files,
        native_libs,
        framework: flags.resolve(),
        debuggable,
    })
}

fn read_manifest(archive: &mut ZipArchive<File>) -> Result<Vec<u8>> {
    let mut entry = archive
        .by_name("AndroidManifest.xml")
        .map_err(|_| Error::InvalidApk("missing AndroidManifest.xml".into()))?;

    if entry.size() > MAX_MANIFEST_SIZE {
        return Err(Error::Manifest("manifest is suspiciously large".into()));
    }

    let mut data = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut data)?;
    Ok(data)
}

fn literal_string<'a>(element: &'a axml::Element, name: &str) -> Option<&'a str> {
    match &element.attr(name)?.value {
        axml::Value::Str(s) => Some(s),
        _ => None,
    }
}

fn resource_string(element: &axml::Element, name: &str) -> Option<String> {
    match &element.attr(name)?.value {
        axml::Value::Str(s) => Some(s.clone()),
        axml::Value::Reference(id) => Some(format!("@{id:#010x} (unresolved)")),
        _ => None,
    }
}

fn has_launcher_intent(component: &axml::Element) -> bool {
    let mut has_main = false;
    let mut has_launcher = false;
    for filter in component.children_named("intent-filter") {
        for action in filter.children_named("action") {
            if action.attr_str("name") == Some("android.intent.action.MAIN") {
                has_main = true;
            }
        }
        for category in filter.children_named("category") {
            if category.attr_str("name") == Some("android.intent.category.LAUNCHER") {
                has_launcher = true;
            }
        }
    }
    has_main && has_launcher
}

/// Android class names: `.Foo` and `Foo` are relative to the package.
fn qualify(package: &str, name: &str) -> String {
    if name.starts_with('.') {
        format!("{package}{name}")
    } else if name.contains('.') {
        name.to_string()
    } else {
        format!("{package}.{name}")
    }
}

fn is_dex(name: &str) -> bool {
    let base = name.rsplit('/').next().unwrap_or(name);
    base.starts_with("classes") && base.ends_with(".dex")
}

fn native_abi(name: &str) -> Option<&str> {
    let rest = name.strip_prefix("lib/")?;
    let (abi, file) = rest.split_once('/')?;
    if abi.is_empty() || !file.ends_with(".so") {
        return None;
    }
    // Only the first directory level, no `lib/abi/sub/lib.so`.
    if file.contains('/') {
        return None;
    }
    Some(abi)
}

fn abi_rank(abi: &str) -> usize {
    ABI_ORDER
        .iter()
        .position(|a| *a == abi)
        .unwrap_or(ABI_ORDER.len())
}

fn natural_dex_order(a: &str, b: &str) -> Ordering {
    fn number(name: &str) -> u32 {
        let stem = name.trim_start_matches("classes").trim_end_matches(".dex");
        if stem.is_empty() {
            1
        } else {
            stem.parse().unwrap_or(u32::MAX)
        }
    }
    number(a).cmp(&number(b)).then(a.cmp(b))
}

#[derive(Default)]
struct FrameworkFlags {
    flutter: bool,
    unity: bool,
    unreal: bool,
    react_native: bool,
    godot: bool,
}

impl FrameworkFlags {
    fn observe(&mut self, name: &str) {
        let base = name.rsplit('/').next().unwrap_or(name);
        self.flutter |= base == "libflutter.so";
        self.unity |= base == "libunity.so";
        self.unreal |= base == "libUE4.so" || base == "libUnreal.so";
        self.react_native |= matches!(
            base,
            "libreactnativejni.so" | "libhermes.so" | "index.android.bundle"
        );
        self.godot |= base == "libgodot.so" || base.starts_with("libgodot.");
    }

    fn resolve(self) -> Framework {
        if self.flutter {
            Framework::Flutter
        } else if self.unity {
            Framework::Unity
        } else if self.unreal {
            Framework::Unreal
        } else if self.react_native {
            Framework::ReactNative
        } else if self.godot {
            Framework::Godot
        } else {
            Framework::Native
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{el, encode, with, AttrValue};
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let unique = format!(
            "apkrunner-test-{}-{}",
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

    fn sample_manifest() -> Vec<u8> {
        let manifest = with(
            el(
                "manifest",
                vec![
                    ("package", AttrValue::Str("com.example.hello")),
                    ("versionCode", AttrValue::Int(7)),
                    ("versionName", AttrValue::Str("1.2")),
                ],
            ),
            vec![
                el(
                    "uses-sdk",
                    vec![
                        ("minSdkVersion", AttrValue::Int(21)),
                        ("targetSdkVersion", AttrValue::Int(34)),
                    ],
                ),
                el(
                    "uses-permission",
                    vec![("name", AttrValue::Str("android.permission.INTERNET"))],
                ),
                with(
                    el(
                        "application",
                        vec![
                            ("label", AttrValue::Str("Hello")),
                            ("icon", AttrValue::Ref(0x7f02_0001)),
                            ("debuggable", AttrValue::Bool(false)),
                        ],
                    ),
                    vec![
                        with(
                            el(
                                "activity",
                                vec![
                                    ("name", AttrValue::Str(".MainActivity")),
                                    ("exported", AttrValue::Bool(true)),
                                ],
                            ),
                            vec![with(
                                el("intent-filter", vec![]),
                                vec![
                                    el(
                                        "action",
                                        vec![(
                                            "name",
                                            AttrValue::Str("android.intent.action.MAIN"),
                                        )],
                                    ),
                                    el(
                                        "category",
                                        vec![(
                                            "name",
                                            AttrValue::Str("android.intent.category.LAUNCHER"),
                                        )],
                                    ),
                                ],
                            )],
                        ),
                        el(
                            "activity",
                            vec![
                                ("name", AttrValue::Str("Settings")),
                                ("exported", AttrValue::Bool(false)),
                            ],
                        ),
                    ],
                ),
            ],
        );
        encode(&manifest)
    }

    fn write_apk(path: &Path, manifest: &[u8], extra: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        zip.start_file("AndroidManifest.xml", options).unwrap();
        zip.write_all(manifest).unwrap();

        zip.start_file("classes.dex", options).unwrap();
        zip.write_all(&[0u8; 128]).unwrap();
        zip.start_file("classes2.dex", options).unwrap();
        zip.write_all(&[0u8; 32]).unwrap();
        zip.start_file("lib/x86_64/libfoo.so", options).unwrap();
        zip.write_all(&[0u8; 16]).unwrap();
        zip.start_file("lib/arm64-v8a/libfoo.so", options).unwrap();
        zip.write_all(&[0u8; 16]).unwrap();

        for (name, bytes) in extra {
            zip.start_file(*name, options).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn inspects_a_real_zip() {
        let dir = temp_dir();
        let apk = dir.join("hello.apk");
        write_apk(&apk, &sample_manifest(), &[]);

        let info = inspect_apk(&apk).expect("inspection works");
        assert_eq!(info.package, "com.example.hello");
        assert_eq!(info.version_name.as_deref(), Some("1.2"));
        assert_eq!(info.version_code, Some(7));
        assert_eq!(info.min_sdk, Some(21));
        assert_eq!(info.target_sdk, Some(34));
        assert_eq!(info.label.as_deref(), Some("Hello"));
        assert_eq!(
            info.launchable_activity.as_deref(),
            Some("com.example.hello.MainActivity")
        );
        assert_eq!(info.activities.len(), 2);
        assert_eq!(info.activities[1].name, "com.example.hello.Settings");
        assert_eq!(info.activities[1].exported, Some(false));
        assert_eq!(info.permissions, vec!["android.permission.INTERNET"]);
        assert_eq!(info.abis, vec!["arm64-v8a", "x86_64"]);
        assert_eq!(
            info.dex_files
                .iter()
                .map(|d| d.name.as_str())
                .collect::<Vec<_>>(),
            vec!["classes.dex", "classes2.dex"]
        );
        assert_eq!(info.native_libs.len(), 2);
        assert_eq!(info.framework, Framework::Native);
        assert!(!info.debuggable);
        assert_eq!(info.file_size, fs::metadata(&apk).unwrap().len());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detects_flutter() {
        let dir = temp_dir();
        let apk = dir.join("flutter.apk");
        write_apk(
            &apk,
            &sample_manifest(),
            &[("lib/arm64-v8a/libflutter.so", &[0u8; 8])],
        );

        let info = inspect_apk(&apk).unwrap();
        assert_eq!(info.framework, Framework::Flutter);
        assert!(info.abis.contains(&"arm64-v8a".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_apk_without_manifest() {
        let dir = temp_dir();
        let apk = dir.join("broken.apk");
        let file = File::create(&apk).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("classes.dex", options).unwrap();
        zip.write_all(&[0u8; 4]).unwrap();
        zip.finish().unwrap();

        let error = inspect_apk(&apk).expect_err("must fail");
        assert!(matches!(error, Error::InvalidApk(_)), "{error}");

        fs::remove_dir_all(&dir).ok();
    }
}
