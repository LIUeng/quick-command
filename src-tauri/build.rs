fn main() {
    println!("cargo:rerun-if-env-changed=QUICK_COMMAND_UPDATER_PUBKEY");
    tauri_build::build()
}
