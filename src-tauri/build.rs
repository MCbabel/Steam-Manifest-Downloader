fn main() {
    // Re-run when build-info env vars change so `option_env!` in system.rs
    // picks up new values on cached CI runs.
    println!("cargo:rerun-if-env-changed=SMD_BUILD_CHANNEL");
    println!("cargo:rerun-if-env-changed=SMD_GIT_SHA");
    println!("cargo:rerun-if-env-changed=SMD_BUILD_DATE");

    tauri_build::build()
}
