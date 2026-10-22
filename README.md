# apkrunner

Run Android `.apk` files as native Linux applications — no container, no
virtual machine, no emulator.

> **Status: M1 functional.** `apkrunner inspect` is complete and `apkrunner run`
> launches real APKs through ATL (ART + GTK4, no container). Verified with
> `demo_app`, `compose_demo_app`, `AdaptiveIconDemo` and `gles3jni`. JW Library
> (`org.jw.jwlibrary.mobile`) also starts and survives its native initialization,
> but its WebView-based UI is blocked on ATL's missing JavaScript bridge. See
> [docs/ROADMAP.md](docs/ROADMAP.md).

## Why

Projects like Waydroid run a complete Android system next to yours. apkrunner
takes the Wine approach instead: keep the APK, cut the Android stack right
below the app, and reimplement `android.*` on top of the Linux desktop,
reusing the real ART runtime. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Build

```sh
cargo build --release
./target/release/apkrunner --help
```

Requirements: Rust 1.75+ and a Linux host. Nothing else for `inspect`:
the manifest is parsed in-process (no `aapt`, no `unzip`).

## Usage

```console
$ apkrunner inspect app.apk
package:       com.example.hello
version:       1.0 (1)
sdk:           min 21 / target 34
label:         Hello
launchable:    com.example.hello.MainActivity
activities:    2
abis:          arm64-v8a, x86_64
dex:           1 (classes.dex, 1.2 MiB)
native libs:   2
framework:     native (Java/Kotlin)
debuggable:    no
size:          2.4 MiB

$ apkrunner inspect --json app.apk | jq .package
"com.example.hello"
```

Other commands:

```sh
apkrunner list                  # installed apps (empty in M0)
apkrunner remove <package>      # remove app + private data
apkrunner run app.apk           # needs the M1 runtime (see below)
apkrunner install app.apk       # M2: desktop integration
```

`--verbose` adds permissions, activities, dex entries and paths.

## What `inspect` reports

- Package, version, min/target SDK, debuggable — parsed from the **binary**
  `AndroidManifest.xml` (`ResXMLTree`), including obfuscated manifests that
  rely on the resource-ID map.
- Activities and `activity-alias`, with the launcher resolved from
  `MAIN` + `LAUNCHER` intent filters.
- Permissions, DEX files (sorted, with sizes) and native ABIs.
- Framework heuristics: Flutter, Unity, Unreal, React Native, Godot.

## Layout

```
crates/apkrunner-apk     ZIP + binary XML + inspection + safe extraction
crates/apkrunner-core    XDG paths, installed-app registry
crates/apkrunner-cli     the `apkrunner` binary
docs/                    architecture and roadmap
```

## Tests

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The AXML tests build real binary manifests with a small encoder, so the parser
is tested end-to-end without committing APK fixtures.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).

## Runtime (M1)

`run` uses [Android Translation Layer](https://gitlab.com/android_translation_layer/android_translation_layer)
as its first backend: real ART compiled for glibc plus `android.*` on GTK4,
without a container or an Android image.

```sh
./scripts/build-runtime.sh          # wolfSSL + bionic + ART + ATL (no root)
export APKRUNNER_RUNTIME_PREFIX=$HOME/.local/apkrunner-runtime
apkrunner run app.apk
```

The script installs everything under the prefix and never touches system
directories. Building ART takes a while and needs ~3 GB of disk. It also
applies the small [ATL compatibility patches](patches/atl/README.md) and
re-dexes the `core-oj`/`core-libart` boot jars with D8 so Java 8 lambdas in
libcore are desugared; the first build downloads R8 9.5.28 (checksum pinned).

## Development

`master` only receives merges from milestone branches (`feat/mN-*`). CI runs
format, clippy, tests and a release build on every push. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the branch and commit conventions.
