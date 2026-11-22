use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // The manifest is embedded below rather than by tauri-build, which gives
    // it to the app only: the unit tests use Tauri too (its mock runtime),
    // and don't start without Common Controls v6.
    let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("failed to run tauri-build");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("app.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }
    player();
}

/// Puts the release build of the player (crates/relay-player) in OUT_DIR,
/// for the standalone-program export to embed. RELAY_PLAYER_EXE uses a
/// prebuilt one instead.
fn player() {
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("relay-player.exe");
    println!("cargo:rerun-if-env-changed=RELAY_PLAYER_EXE");
    if let Ok(prebuilt) = std::env::var("RELAY_PLAYER_EXE") {
        std::fs::copy(&prebuilt, &out).unwrap_or_else(|e| panic!("RELAY_PLAYER_EXE ({prebuilt}): {e}"));
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    for p in ["crates/relay-player", "crates/relay-playback", "crates/relay-core", "crates/relay-platform"] {
        for part in ["src", "Cargo.toml"] {
            println!("cargo:rerun-if-changed={}", root.join(p).join(part).display());
        }
    }
    for p in ["crates/relay-player/build.rs", "crates/relay-player/player.manifest", "Cargo.toml", "Cargo.lock"] {
        println!("cargo:rerun-if-changed={}", root.join(p).display());
    }
    // A target folder of its own: cargo holds a lock on this build's.
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from).join("player");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut build = Command::new(cargo);
    build.args(["build", "-p", "relay-player", "--bin", "relay-player", "--release", "--target-dir"]);
    build.arg(&target).current_dir(&root);
    // Not the wrappers and flags of this build (clippy, coverage): a plain release build.
    for var in [
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTC_WRAPPER",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTFLAGS",
        "CARGO_TARGET_DIR",
        "CARGO_BUILD_TARGET",
    ] {
        build.env_remove(var);
    }
    let result = build.output().expect("run cargo to build the player");
    if !result.status.success() {
        panic!("building the player failed:\n{}", String::from_utf8_lossy(&result.stderr));
    }
    let exe = target.join("release").join(if cfg!(windows) { "relay-player.exe" } else { "relay-player" });
    std::fs::copy(&exe, &out).unwrap_or_else(|e| panic!("{}: {e}", exe.display()));
}
