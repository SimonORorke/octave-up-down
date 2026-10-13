// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use log::{debug};
use octave_up_down::MainWindow;
use slint::ComponentHandle;
use app_info::{APP_TITLE};

fn main() -> Result<(), slint::PlatformError> {
    // Initialise logging. Levels are chosen at runtime via the RUST_LOG env var
    // (e.g. `RUST_LOG=debug`, or `RUST_LOG=package_name::module_name=trace`); with RUST_LOG unset
    // the default filter is `info`, so the `debug!`/`trace!` lines stay silent.
    // `log` is a facade: this init line is the only place that names `env_logger`, so switching to
    // a file logger later changes just this line, not the call sites.
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis().init();
    let main_window = MainWindow::new()?;
    main_window.set_window_title(APP_TITLE.into());
    debug!("main: Running main_window");
    main_window.run()
}
