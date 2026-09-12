use std::process::Command;

pub fn load_apk(apk_path: &str) -> String {
    let output_dir = format!("/tmp/apkrunner/{}", extract_name(apk_path));

    let status = Command::new("unzip")
        .arg("-o")
        .arg(apk_path)
        .arg("-d")
        .arg(&output_dir)
        .status()
        .expect("failed to unzip");

    if !status.success() {
        panic!("failed to extract apk");
    }

    format!("{}/classes.dex", output_dir)
}

fn extract_name(path: &str) -> String {
    path.split('/')
        .last()
        .unwrap()
        .replace(".apk", "")
}