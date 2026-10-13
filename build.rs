fn main() {
    let manifest_dir = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let library_paths = std::collections::HashMap::from([(
        "sleek-ui".to_string(),
        manifest_dir.join("../sleek-ui/ui/sleek-ui"),
    )]);
    let config = slint_build::CompilerConfiguration::new().with_library_paths(library_paths);
    slint_build::compile_with_config("ui/main_window.slint", config).unwrap();
}