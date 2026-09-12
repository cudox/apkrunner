#[derive(Debug, Clone)]
pub struct ApkInspection {
    pub dex_count: usize,
    pub has_manifest: bool,
    pub has_kotlin: bool,
    pub has_x86_64: bool,
    pub has_arm64: bool,
    pub has_x86: bool,
    pub has_arm32: bool,
    pub package: String,
    pub version: String,
    pub main_activity: String,
    pub min_sdk: String,
    pub target_sdk: String,
}

#[derive(Debug, Clone)]
pub struct ApkInfo {
    pub target_sdk: String,
}