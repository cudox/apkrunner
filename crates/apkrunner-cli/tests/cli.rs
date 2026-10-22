use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> PathBuf {
    let unique = format!(
        "apkrunner-cli-{}-{}-{}",
        std::process::id(),
        name,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let dir = std::env::temp_dir().join(unique);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str], data_dir: &PathBuf) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_apkrunner"))
        .args(args)
        .env("APKRUNNER_DATA_DIR", data_dir)
        .env("APKRUNNER_STATE_DIR", data_dir.join("state"))
        .output()
        .expect("binary runs")
}

#[test]
fn list_is_empty_on_a_fresh_data_dir() {
    let dir = temp_dir("list");
    let output = run(&["list"], &dir);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No applications installed"), "{stdout}");
    assert!(dir.join("apps").is_dir());
    assert!(dir.join("shared").is_dir());

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn inspect_missing_file_fails_cleanly() {
    let dir = temp_dir("missing");
    let output = run(&["inspect", "/nonexistent/app.apk"], &dir);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("apkrunner: "), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn run_reports_m1_stub() {
    let dir = temp_dir("run");
    let apk = dir.join("fake.apk");
    fs::write(&apk, b"not really an apk").unwrap();

    let output = run(
        &[
            "run",
            apk.to_str().unwrap(),
            "--activity",
            "com.example.Main",
        ],
        &dir,
    );

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("M1"), "{stderr}");
    assert!(stderr.contains("com.example.Main"), "{stderr}");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn remove_refuses_invalid_package() {
    let dir = temp_dir("remove");
    let output = run(&["remove", "../../etc"], &dir);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid package name"), "{stderr}");

    fs::remove_dir_all(&dir).ok();
}
