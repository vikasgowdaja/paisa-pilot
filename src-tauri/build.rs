use std::{env, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("Cargo provides the manifest directory"),
    );
    let env_path = manifest_dir.join("../.env");
    println!("cargo:rerun-if-changed={}", env_path.display());

    if env_path.exists() {
        dotenvy::from_path(&env_path)
            .unwrap_or_else(|error| panic!("Could not load {}: {error}", env_path.display()));
    }

    let target_os =
        env::var("CARGO_CFG_TARGET_OS").expect("Cargo provides the target operating system");
    if target_os == "windows" {
        for key in [
            "PAISAPILOT_GOOGLE_CLIENT_ID",
            "PAISAPILOT_GOOGLE_CLIENT_SECRET",
        ] {
            println!("cargo:rerun-if-env-changed={key}");
            if let Ok(value) = env::var(key) {
                assert!(
                    !value.contains(['\r', '\n']),
                    "{key} must be a single-line value"
                );
                println!("cargo:rustc-env={key}={value}");
            }
        }
    }

    tauri_build::build()
}
