// src/runtime/environment.rs

#[derive(Debug)]
pub struct AndroidEnvironment {

    pub api_level: u32,

    pub package_name: String,

    pub cache_dir: String,

    pub files_dir: String,
}

impl AndroidEnvironment {

    pub fn new(
        package_name: &str,
    ) -> Self {

        Self {

            api_level: 35,

            package_name:
                package_name.to_string(),

            cache_dir:
                "/tmp/apkrunner/cache"
                .to_string(),

            files_dir:
                "/tmp/apkrunner/files"
                .to_string(),
        }
    }
}