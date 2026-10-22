use std::path::{Path, PathBuf};
use std::process::ExitCode;

use apkrunner_apk::{inspect_apk, ApkInfo};
use apkrunner_core::backend::LaunchRequest;
use apkrunner_core::{installed, paths, RuntimeBackend};
use apkrunner_runtime::AtlBackend;
use clap::{Parser, Subcommand};

/// Exit code used for "this milestone does not implement it yet".
const EXIT_NOT_IMPLEMENTED: u8 = 3;

#[derive(Parser, Debug)]
#[command(
    name = "apkrunner",
    version,
    about = "Run Android APKs as native Linux applications",
    long_about = "apkrunner runs Android applications on Linux without a container or a \
                  virtual machine. Inspect APKs today; the runtime backend lands in M1."
)]
struct Cli {
    /// Print extra information (permissions, native libraries, paths).
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Show package, SDK, activities, ABIs and framework of an APK.
    Inspect {
        /// Path to the .apk file.
        apk: PathBuf,
        /// Machine-readable output (stable JSON schema).
        #[arg(long)]
        json: bool,
    },

    /// Launch an APK (needs the runtime backend, M1).
    Run {
        /// Path to the .apk file.
        apk: PathBuf,
        /// Activity class to launch (defaults to the manifest's launcher).
        #[arg(long)]
        activity: Option<String>,
        /// Initial window width in pixels.
        #[arg(long)]
        width: Option<u32>,
        /// Initial window height in pixels.
        #[arg(long)]
        height: Option<u32>,
        /// Extra arguments passed to the runtime after `--`.
        #[arg(last = true)]
        extra: Vec<String>,
    },

    /// Install an APK so it appears in the application menu (M2).
    Install {
        /// Path to the .apk file.
        apk: PathBuf,
    },

    /// List installed applications.
    List,

    /// Remove an installed application and its private data.
    Remove {
        /// Android package name (for example com.example.hello).
        package: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("apkrunner: {message}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, String> {
    match cli.command {
        Command::Inspect { apk, json } => {
            let info = inspect_apk(&apk).map_err(|e| e.to_string())?;
            if json {
                let rendered = serde_json::to_string_pretty(&info).map_err(|e| e.to_string())?;
                println!("{rendered}");
            } else {
                print_human(&info, cli.verbose);
            }
            Ok(ExitCode::SUCCESS)
        }

        Command::Run {
            apk,
            activity,
            width,
            height,
            extra,
        } => {
            let info = inspect_apk(&apk).map_err(|e| e.to_string())?;

            let activity = activity
                .or_else(|| info.launchable_activity.clone())
                .ok_or_else(|| {
                    format!(
                        "{} has no launcher activity; pass --activity <class>",
                        info.package
                    )
                })?;

            let backend = AtlBackend::discover().map_err(|e| e.to_string())?;
            paths::ensure_layout().map_err(|e| e.to_string())?;

            let data_dir = paths::data_dir().join("runtime/atl").join(&info.package);
            std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
            let log_path = paths::log_path(&info.package);

            eprintln!("apkrunner: launching {} ({activity})", info.package);
            eprintln!("runtime:   {}", backend.name());
            eprintln!("log:       {}", log_path.display());

            let request = LaunchRequest {
                apk,
                package: info.package,
                activity,
                data_dir,
                width,
                height,
                extra_args: extra,
            };

            let code =
                apkrunner_runtime::run(&backend, &request, &log_path).map_err(|e| e.to_string())?;
            Ok(ExitCode::from(code.clamp(0, 255) as u8))
        }

        Command::Install { apk } => {
            ensure_file(&apk)?;
            eprintln!(
                "apkrunner: install is not implemented yet (roadmap M2).\n\
                 would install: {}",
                apk.display()
            );
            Ok(ExitCode::from(EXIT_NOT_IMPLEMENTED))
        }

        Command::List => {
            paths::ensure_layout().map_err(|e| e.to_string())?;
            let apps = installed::list_installed().map_err(|e| e.to_string())?;
            if apps.is_empty() {
                println!("No applications installed.");
                println!("Data directory: {}", paths::apps_dir().display());
                return Ok(ExitCode::SUCCESS);
            }

            println!("{:<40} VERSIONS", "PACKAGE");
            for app in apps {
                let versions = if app.versions.is_empty() {
                    "-".to_string()
                } else {
                    app.versions.join(", ")
                };
                println!("{:<40} {}", app.package, versions);
            }
            Ok(ExitCode::SUCCESS)
        }

        Command::Remove { package } => match installed::remove_installed(&package) {
            Ok(true) => {
                println!("Removed {package}");
                Ok(ExitCode::SUCCESS)
            }
            Ok(false) => {
                eprintln!("apkrunner: {package} is not installed");
                Ok(ExitCode::from(1))
            }
            Err(e) => Err(e.to_string()),
        },
    }
}

fn ensure_file(path: &Path) -> Result<(), String> {
    if path.is_file() {
        Ok(())
    } else {
        Err(format!("file not found: {}", path.display()))
    }
}

fn print_human(info: &ApkInfo, verbose: bool) {
    println!("package:       {}", info.package);
    println!("version:       {}", version_line(info));
    println!("sdk:           {}", sdk_line(info));
    if let Some(label) = &info.label {
        println!("label:         {label}");
    }
    if let Some(icon) = &info.icon {
        println!("icon:          {icon}");
    }
    println!(
        "launchable:    {}",
        info.launchable_activity.as_deref().unwrap_or("(none)")
    );
    println!("activities:    {}", info.activities.len());
    println!("abis:          {}", join_or_none(&info.abis));
    println!("dex:           {}", dex_line(info));
    println!("native libs:   {}", info.native_libs.len());
    println!("framework:     {}", info.framework.display_name());
    println!(
        "debuggable:    {}",
        if info.debuggable { "yes" } else { "no" }
    );
    println!("size:          {}", human_size(info.file_size));

    if verbose {
        if !info.permissions.is_empty() {
            println!("\npermissions:");
            let mut permissions = info.permissions.clone();
            permissions.sort();
            for permission in permissions {
                println!("  {permission}");
            }
        }
        if !info.activities.is_empty() {
            println!("\nactivities detail:");
            for activity in &info.activities {
                let marker = if activity.is_launcher { "*" } else { " " };
                println!(" {marker} {}", activity.name);
                if activity.class_name != activity.name {
                    println!("     class: {}", activity.class_name);
                }
            }
        }
        println!("\ndex files:");
        for dex in &info.dex_files {
            println!("  {} ({})", dex.name, human_size(dex.size));
        }
        println!("\ndata directory: {}", paths::data_dir().display());
    }
}

fn version_line(info: &ApkInfo) -> String {
    match (&info.version_name, info.version_code) {
        (Some(name), Some(code)) => format!("{name} ({code})"),
        (Some(name), None) => name.clone(),
        (None, Some(code)) => format!("({code})"),
        (None, None) => "unknown".to_string(),
    }
}

fn sdk_line(info: &ApkInfo) -> String {
    let min = info
        .min_sdk
        .map(|v| v.to_string())
        .unwrap_or_else(|| "?".into());
    let target = info
        .target_sdk
        .map(|v| v.to_string())
        .unwrap_or_else(|| "?".into());
    format!("min {min} / target {target}")
}

fn dex_line(info: &ApkInfo) -> String {
    if info.dex_files.is_empty() {
        return "none".to_string();
    }
    let total: u64 = info.dex_files.iter().map(|d| d.size).sum();
    let names: Vec<&str> = info.dex_files.iter().map(|d| d.name.as_str()).collect();
    format!(
        "{} ({}, {})",
        names.len(),
        names.join(", "),
        human_size(total)
    )
}

fn join_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(", ")
    }
}

fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
