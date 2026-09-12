use std::path::PathBuf;

pub fn runtime_root() -> PathBuf {

    let home =
        std::env::var("HOME")
        .unwrap();

    PathBuf::from(home)
        .join(".apkrunner")
}

pub fn app_dir(
    package: &str,
) -> PathBuf {

    runtime_root()
        .join("apps")
        .join(package)
}