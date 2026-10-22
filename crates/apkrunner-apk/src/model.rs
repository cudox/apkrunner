use serde::Serialize;

/// Everything `apkrunner inspect` knows about an APK. Stable JSON shape.
#[derive(Debug, Clone, Serialize)]
pub struct ApkInfo {
    pub path: String,
    pub file_size: u64,
    pub package: String,
    pub version_name: Option<String>,
    pub version_code: Option<u32>,
    pub min_sdk: Option<u32>,
    pub target_sdk: Option<u32>,
    pub label: Option<String>,
    pub icon: Option<String>,
    pub launchable_activity: Option<String>,
    pub activities: Vec<Activity>,
    pub permissions: Vec<String>,
    pub abis: Vec<String>,
    pub dex_files: Vec<DexEntry>,
    pub native_libs: Vec<String>,
    pub framework: Framework,
    pub debuggable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Activity {
    /// Component name as declared (may be an `activity-alias`).
    pub name: String,
    /// Concrete class to instantiate (`targetActivity` for aliases).
    pub class_name: String,
    pub exported: Option<bool>,
    pub label: Option<String>,
    pub is_launcher: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DexEntry {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Framework {
    Native,
    Flutter,
    Unity,
    Unreal,
    ReactNative,
    Godot,
}

impl Framework {
    pub fn display_name(self) -> &'static str {
        match self {
            Framework::Native => "native (Java/Kotlin)",
            Framework::Flutter => "Flutter",
            Framework::Unity => "Unity",
            Framework::Unreal => "Unreal Engine",
            Framework::ReactNative => "React Native",
            Framework::Godot => "Godot",
        }
    }
}
