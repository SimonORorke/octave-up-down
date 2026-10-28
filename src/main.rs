// Prevent console window in addition to Slint window in Windows release builds, e.g. when starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use log::{debug, trace};
use slint::{CloseRequestResponse, ComponentHandle, Weak};
use app_info::{APP_TITLE, COPYRIGHT, DOCUMENTATION_LINK, LICENSE, PROJECT_LINK, RELEASES_LINK,
               VERSION};
use octave_up_down::{MainWindow, Presenter, SharedPresenter, SlintDeviceType,
                     UiDeviceStrategy, UiInputStrategy, UiMethods, UiOutputStrategy};

fn main() -> Result<(), slint::PlatformError> {
    // Initialise logging. Levels are chosen at runtime via the RUST_LOG env var
    // (e.g. `RUST_LOG=debug`, or `RUST_LOG=package_name::module_name=trace`); with RUST_LOG unset
    // the default filter is `info`, so the `debug!`/`trace!` lines stay silent.
    // `log` is a facade: this init line is the only place that names `env_logger`, so switching to
    // a file logger later changes just this line, not the call sites.
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis().init();
    install_solid_title_bar_platform();
    let main_window = MainWindow::new()?;
    main_window.set_window_title(APP_TITLE.into());
    // let new_version_window = create_new_version_window();
    let ui_methods =
        Arc::new(UiMethods::new(main_window.as_weak(),
                                // new_version_window.as_weak()
        ));
    let presenter: SharedPresenter = Arc::new(Mutex::new(Presenter::new(ui_methods.clone())));
    // let about_window: Rc<RefCell<Option<AboutWindow>>> = Rc::new(RefCell::new(None));
    init_ui_handlers(&main_window,
                     // &new_version_window,
                     presenter.clone(), ui_methods.clone(),
                     // about_window.clone()
    );
    // Initialise presenter on a background thread so that UI callbacks within
    // init() can use invoke_from_event_loop without deadlocking.
    let self_arc = Arc::clone(&presenter);
    spawn_presenter_action(&presenter, move |c| c.init(&self_arc));
    debug!("main: Running main_window");
    main_window.run()
}

/// Slint's winit backend requests a transparent NSWindow on macOS by default (`with_transparent(true)`,
/// used so rounded window corners can be drawn). A transparent NSWindow is non-opaque with a clear
/// background colour, so any area the Slint renderer doesn't paint over - most notably the native
/// title bar strip - lets the desktop wallpaper show straight through. Disabling both window
/// transparency and title bar transparency here forces macOS to give the window (and its title bar)
/// a solid, opaque background again, matching the behaviour of other native macOS apps. Must run
/// before any Slint window is created.
#[cfg(target_os = "macos")]
fn install_solid_title_bar_platform() {
    use i_slint_backend_winit::winit::platform::macos::WindowAttributesExtMacOS;

    let backend = i_slint_backend_winit::Backend::builder()
        .with_window_attributes_hook(|attributes| {
            attributes.with_transparent(false).with_titlebar_transparent(false)
        })
        .build()
        .unwrap();
    slint::platform::set_platform(Box::new(backend)).unwrap();
}

#[cfg(not(target_os = "macos"))]
fn install_solid_title_bar_platform() {}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn register_app_will_terminate_handler(callback: extern "C" fn());
}

#[cfg(target_os = "macos")]
thread_local! {
    static MAC_TERMINATION_CLOSURE: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
}

#[cfg(target_os = "macos")]
extern "C" fn on_mac_app_will_terminate() {
    MAC_TERMINATION_CLOSURE.with(|closure| {
        if let Some(f) = closure.borrow_mut().take() {
            f();
        }
    });
}

fn init_ui_handlers(main_window: &MainWindow,
                    // new_version_window: &NewVersionWindow,
                    presenter: SharedPresenter, ui_methods: Arc<UiMethods>,
                    // about_window: Rc<RefCell<Option<AboutWindow>>>
) {
//     main_window.on_open_documentation(move || {
//         open::that(DOCUMENTATION_LINK).unwrap();
//     });
//     {
//         let about_window = Rc::clone(&about_window);
//         let ui_methods_clone = ui_methods.clone();
//         main_window.on_show_about_window(move || {
//             show_about_window(about_window.clone(), ui_methods_clone.clone());
//         });
//     }
    {
        let presenter: SharedPresenter = Arc::clone(&presenter);
        let main_window_weak = main_window.as_weak();
        // let about_window = Rc::clone(&about_window);
        // let new_version_window_weak = new_version_window.as_weak();
        main_window.window().on_close_requested(move || {
            // handle_close_request(&main_window_weak, &presenter, &about_window,
            //                      new_version_window_weak.clone())
            handle_close_request(&main_window_weak, &presenter)
        });
    }
    #[cfg(target_os = "macos")]
    {
        // Handle Cmd+Q to call perform_close, the same method as the close button.
        let main_window_weak = main_window.as_weak();
        let presenter = Arc::clone(&presenter);
        let about_window = Rc::clone(&about_window);
        let new_version_window_weak = new_version_window.as_weak();
        MAC_TERMINATION_CLOSURE.with(|closure| {
            *closure.borrow_mut() = Some(Box::new(move || {
                let _ = perform_close(&main_window_weak, &presenter, &about_window, &new_version_window_weak);
            }));
        });
        unsafe {
            register_app_will_terminate_handler(on_mac_app_will_terminate);
        }
    }
    // All Presenter methods must be called from non-UI threads to avoid deadlock.
    // See the UiMethods.with_main_window_result doc comment for more information.
    // {
    //     let presenter: SharedPresenter = Arc::clone(&presenter);
    //     main_window.on_check_for_updates(move || {
    //         spawn_presenter_action(&presenter, move |c| c.check_for_updates());
    //     });
    // }
    {
        let presenter: SharedPresenter = Arc::clone(&presenter);
        main_window.on_connect_device(move |device_type: SlintDeviceType| {
            let device_strategy = create_device_strategy(device_type);
            spawn_presenter_action(&presenter, move |c| c.connect_device(&*device_strategy));
        });
    }
    {
        let presenter: SharedPresenter = Arc::clone(&presenter);
        main_window.on_refresh_devices(move |device_type: SlintDeviceType| {
            let device_strategy = create_device_strategy(device_type);
            spawn_presenter_action(&presenter, move |c| c.refresh_devices(&*device_strategy));
        });
    }
    {
        let presenter: SharedPresenter = Arc::clone(&presenter);
        main_window.on_down_cc_no_changed(move |value| {
            trace!("init_ui_handlers on_down_cc_no_changed: {}", value);
            spawn_presenter_action(
                &presenter, move |c| c.set_down_cc_no(value as u8));
        });
    }
    {
        let presenter: SharedPresenter = Arc::clone(&presenter);
        main_window.on_up_cc_no_changed(move |value| {
            spawn_presenter_action(
                &presenter, move |c| c.set_up_cc_no(value as u8));
        });
    }
    {
        let presenter: SharedPresenter = Arc::clone(&presenter);
        main_window.on_in_cc_no_changed(move |value| {
            spawn_presenter_action(
                &presenter, move |c| c.set_in_cc_no(value as u8));
        });
    }
    {
        let presenter: SharedPresenter = Arc::clone(&presenter);
        main_window.on_out_cc_no_changed(move |value| {
            spawn_presenter_action(
                &presenter, move |c| c.set_out_cc_no(value as u8));
        });
    }
}

fn perform_close<'a>(
    main_window_weak: &'a Weak<MainWindow>,
    presenter: &'a SharedPresenter,
    // about_window: &'a Rc<RefCell<Option<AboutWindow>>>,
    // new_version_window_weak: &'a Weak<NewVersionWindow>,
) -> Result<(), Box<dyn std::error::Error + 'a>> {
    if IS_CLOSED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    trace!("main.perform_close");
    // if let Some(dialog) = about_window.borrow().as_ref()
    //     && ComponentHandle::window(dialog).is_visible() {
    //     dialog.hide().ok();
    // }
    // if let Some(new_version_window) = new_version_window_weak.upgrade()
    //     && ComponentHandle::window(&new_version_window).is_visible() {
    //     new_version_window.hide().ok();
    // }
    let (x, y) = if let Some(main_window) = main_window_weak.upgrade() {
        let pos = main_window.window().position();
        #[cfg(target_os = "macos")]
        {
            let scale_factor = main_window.window().scale_factor();
            let logical_x = (pos.x as f32 / scale_factor).round() as i32;
            let logical_y = (pos.y as f32 / scale_factor).round() as i32;
            (logical_x, logical_y)
        }
        #[cfg(not(target_os = "macos"))]
        {
            (pos.x, pos.y)
        }
    } else {
        (0, 0)
    };
    presenter.lock()?.close(x, y)
}

fn handle_close_request(main_window_weak: &Weak<MainWindow>, presenter: &SharedPresenter,
                        // about_window: &Rc<RefCell<Option<AboutWindow>>>,
                        // new_version_window_weak: Weak<NewVersionWindow>
) -> CloseRequestResponse {
    trace!("main.handle_close_request");
    if IS_CLOSE_ERROR_SHOWN.load(Ordering::Relaxed) {
        // If a close error message is already shown, allow the window to be closed.
        return CloseRequestResponse::HideWindow;
    }
    if perform_close(main_window_weak, presenter,
                     // about_window, &new_version_window_weak
    ).is_err() {
        IS_CLOSE_ERROR_SHOWN.store(true, Ordering::Relaxed);
        IS_CLOSED.store(false, Ordering::SeqCst);
        return CloseRequestResponse::KeepWindowShown;
    }
    CloseRequestResponse::HideWindow
}

fn create_device_strategy(device_type: SlintDeviceType)
                          -> Box<dyn UiDeviceStrategy> {
    match device_type {
        SlintDeviceType::Input => UiInputStrategy::new().clone_box(),
        SlintDeviceType::Output => UiOutputStrategy::new().clone_box(),
    }
}

/// Runs `action` against the `Presenter` on a rayon worker thread.
///
/// Every UI-event handler funnels through here: `Presenter` methods must run off the Slint
/// UI thread (they re-enter the UI via `invoke_from_event_loop`, which deadlocks if called from
/// the UI thread — see `UiMethods::with_main_window_result`). Centralising clone → spawn → lock
/// also keeps the lock policy in one place.
fn spawn_presenter_action(
    presenter: &SharedPresenter,
    action: impl FnOnce(&mut Presenter) + Send + 'static,
) {
    let presenter = Arc::clone(presenter);
    rayon::spawn(move || {
        let mut guard = presenter.lock().unwrap();
        action(&mut guard);
    });
}

static IS_CLOSED: AtomicBool = AtomicBool::new(false);
static IS_CLOSE_ERROR_SHOWN: AtomicBool = AtomicBool::new(false);