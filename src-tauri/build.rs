use std::{env, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("Cargo provides the manifest directory"),
    );
    let env_path = manifest_dir.join("../.env");
    println!("cargo:rerun-if-changed={}", env_path.display());
    println!("cargo:rerun-if-env-changed=PAISAPILOT_GOOGLE_CLIENT_ID");

    if env_path.exists() {
        dotenvy::from_path(&env_path)
            .unwrap_or_else(|error| panic!("Could not load {}: {error}", env_path.display()));
    }

    if let Ok(client_id) = env::var("PAISAPILOT_GOOGLE_CLIENT_ID") {
        println!("cargo:rustc-env=PAISAPILOT_GOOGLE_CLIENT_ID={client_id}");
    }

    tauri_build::build()
}
