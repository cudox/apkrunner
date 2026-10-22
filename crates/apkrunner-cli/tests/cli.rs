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
fn run_rejects_an_invalid_apk() {
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

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("apkrunner: "), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");

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

#[test]
fn run_launches_the_manifest_activity_and_propagates_the_exit_code() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("run-ok");
    let apk = dir.join("hello.apk");
    fs::write(&apk, apkrunner_apk::fixtures::minimal_apk()).unwrap();

    let fake = dir.join("fake-atl");
    fs::write(&fake, "#!/bin/sh\necho 'fake runtime log' >&2\nexit 7\n").unwrap();
    let mut perms = fs::metadata(&fake).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&fake, perms).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_apkrunner"))
        .args(["run", apk.to_str().unwrap()])
        .env("APKRUNNER_DATA_DIR", &dir)
        .env("APKRUNNER_STATE_DIR", dir.join("state"))
        .env("APKRUNNER_ATL_BIN", &fake)
        .env("APKRUNNER_RUNTIME_PREFIX", dir.join("runtime"))
        .output()
        .expect("binary runs");

    assert_eq!(output.status.code(), Some(7));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("launching com.example.hello (com.example.hello.MainActivity)"),
        "{stderr}"
    );
    assert!(dir.join("runtime/atl/com.example.hello").is_dir());
    let log = fs::read_to_string(dir.join("state/com.example.hello.log")).unwrap();
    assert!(log.contains("fake runtime log"), "{log}");

    fs::remove_dir_all(&dir).ok();
}
