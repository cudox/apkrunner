#[derive(Debug)]
pub struct RuntimePlan {
    pub dex_files: Vec<String>,
    pub native_libs: Vec<String>,
    pub main_activity: String,
}