fn main() {
    // The app defines an ACL manifest for its own commands: without it,
    // Tauri's dispatch-level capability gate never applies to app
    // commands and ANY window (including the untrusted Preview) could
    // invoke them. With it, `capabilities/default.json` grants the
    // generated allow-* permissions to "main" only, and the Preview
    // window's isolation is real by construction. The generated
    // artifacts are exposed to tests so the property is CI-pinned.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "begin_session",
            "start_demo",
            "read_journal",
            "finish_demo",
            "journal_position",
            "stop_session",
            "open_preview",
        ]),
    ))
    .expect("failed to run tauri-build");
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    println!(
        "cargo:rustc-env=SYMBIOTE_ACL_MANIFESTS={}",
        out_dir.join("acl-manifests.json").display()
    );
    println!(
        "cargo:rustc-env=SYMBIOTE_RESOLVED_CAPABILITIES={}",
        out_dir.join("capabilities.json").display()
    );
}
