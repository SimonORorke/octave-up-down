fn main() {
    // I looked into referencing sleek-ui on GitHub.
    // Though possible, it would be complicated and unreliable.
    // So we are sticking to sleek-ui's author's instruction, to reference a relative path on the
    // same computer. If this project is uploaded to GitHub, sleek-ui should be moved to inside
    // the project, with the path below updated accordingly.
    let manifest_dir =
        std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let library_paths = std::collections::HashMap::from([(
        "sleek-ui".to_string(),
        manifest_dir.join("../sleek-ui/ui/sleek-ui"),
    )]);
    let config =
        slint_build::CompilerConfiguration::new().with_library_paths(library_paths);
    slint_build::compile_with_config("ui/main_window.slint", config).unwrap();

    // Check the target platform instead of using a build directive,
    // in case the build script is run on a non-Windows platform.
    let is_windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    if is_windows {
        // Generate a manifest to specify the application icon and the exe file properties.
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let icon_path = format!("ui/images/{}.ico", app_info::APP_TITLE);
        let version = app_info::VERSION;
        let product_name = app_info::APP_TITLE;
        let copyright = app_info::COPYRIGHT;
        let rc_path = simonor::BuildHelper::configure_app_icon_and_exe_file_properties(
            &manifest_dir, &out_dir, &icon_path, version, product_name, copyright);
        let _ = embed_resource::compile(&rc_path, embed_resource::NONE);
    }
}