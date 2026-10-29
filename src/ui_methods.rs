use std::rc::Rc;
use std::sync::{Arc, Mutex};
use i_slint_backend_winit::WinitWindowAccessor;
use log::trace;
use slint::{ComponentHandle, PhysicalPosition, Weak};
#[cfg(target_os = "macos")]
use slint::LogicalPosition;
use app_info::APP_TITLE;
use simonor::{DeviceType};
use crate::{MainWindow, NewVersionCallbacks, OptionsModel, SlintMessageType, UiDeviceStrategy,
            UOptionData};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MessageType {
    Error,
    Info,
    Warning
}

/// A trait that defines the interface for methods called by `Presenter`
/// to make changes to the UI.
///
/// The `I` prefix is borrowed from .Net standards, where `I` stands for `interface`,
/// which is the .Net equivalent of Rust's `trait`.
///
/// Rationale for the `I` prefix:
/// `UiMethods` describes the purpose of the struct of that name,
/// which implements `IUiMethods`. The `IUiMethods` trait exists only to allow `UiMethods` to be mocked.
/// Nor do we use `mockall` or similar to create the mocks:  so, for example, we don't need to
/// rename trait `IUiMethods` to `UiMethods` so that a mock called `MockUiMethods` can be automatically
/// generated. And a `T` prefix is avoided because `T` in Rust is a common type parameter.
///
/// Project-wide trait-naming convention (two tiers). This is the key justification for the
/// non-idiomatic `I` prefix, so the two tiers are kept distinct on purpose:
/// * **`I`-prefixed traits are service interfaces**: each abstracts a *single* concrete struct
///   (`ISettings`/`Settings`, `IUiMethods`/`UiMethods`)
///   so it can be injected and mocked. The struct keeps the descriptive name; the trait takes the
///   `I` prefix.
/// * **Role-named traits with no prefix are seams / callbacks**: named for what they do rather than
///   for one concrete type, and may have several implementors.
///
/// So an `I` prefix vs a bare role-name is a deliberate signal of which kind of trait it is. Don't
/// "de-prefix" the service interfaces to match Rust's usual no-`I` convention — that would erase the
/// distinction.
pub trait IUiMethods: Send + Sync {
    fn focus_device(&self, device_strategy: &dyn UiDeviceStrategy);
    fn get_selected_device_index(&self, device_strategy: &dyn UiDeviceStrategy) -> usize;
    fn set_selected_device_index(&self, index: usize, device_strategy: &dyn UiDeviceStrategy);
    fn set_devices_model(&self, device_names: &[String], device_strategy: &dyn UiDeviceStrategy);
    fn show_connected_device_name(&self, name: &str, msg_type: MessageType,
                                  device_strategy: &dyn UiDeviceStrategy);
    fn show_message(&self, msg: &str, msg_type: MessageType);
    fn show_new_version_window(&self, new_version: &str, auto_check_new_versions: bool,
                               callbacks: Arc<Mutex<dyn NewVersionCallbacks>>);
    fn set_main_window_position(&self, x: i32, y: i32);
    fn set_down_cc_no(&self, value: u8);
    fn set_up_cc_no(&self, value: u8);
    fn set_in_cc_no(&self, value: u8);
    fn set_out_cc_no(&self, value: u8);
}

/// This struct contains the methods called by `Presenter` to make changes to the UI.
/// It is part of the view in the Model-View-Presenter (MVP) pattern.
/// See `Presenter`'s doc comment for more information on how the project implements MVP.
#[derive(Clone)]
pub struct UiMethods {
    main_window_weak: Weak<MainWindow>,
}

impl UiMethods {
    pub fn new(main_window_weak: Weak<MainWindow>) -> Self {
        Self { main_window_weak }
    }

    /// Provide the specified closure with a MainWindow instance without returning a result.
    /// This method supports invocation from both the UI event loop and non-UI threads.
    fn with_main_window<F>(&self, f: F)
    where
        F: FnOnce(&MainWindow) + Send + 'static,
    {
        let weak = self.main_window_weak.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(main_window) = weak.upgrade() {
                f(&main_window);
            }
        }).unwrap();
    }

    /// Provide the specified closure with a MainWindow instance and return its result.
    /// Blocks the calling thread until the closure completes on the UI event loop.
    /// Must be called from a non-UI thread to avoid deadlock.
    fn with_main_window_result<T, F>(&self, f: F) -> T
    where
        T: Send + Default + 'static,
        F: FnOnce(&MainWindow) -> T + Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let weak = self.main_window_weak.clone();
        slint::invoke_from_event_loop(move || {
            let result = if let Some(main_window) = weak.upgrade() {
                f(&main_window)
            } else {
                T::default()
            };
            tx.send(result).ok();
        }).unwrap();
        rx.recv().unwrap_or_default()
    }

    /// Provide the specified closure for execution on the UI thread.
    fn with_ui_thread<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        slint::invoke_from_event_loop(f).unwrap();
    }
}

impl IUiMethods for UiMethods {
    fn focus_device(&self, device_strategy: &dyn UiDeviceStrategy) {
        let device_strategy = device_strategy.clone_box();
        self.with_main_window(move |main_window| {
            device_strategy.focus_device(main_window);
        });
    }

    fn get_selected_device_index(&self, device_strategy: &dyn UiDeviceStrategy) -> usize {
        trace!("get_selected_device_index: {:?}", device_strategy.device_type());
        let device_strategy = device_strategy.clone_box();
        let index = self.with_main_window_result(move |main_window| {
            device_strategy.get_selected_device_index(main_window) as usize
        });
        trace!("get_selected_device_index: returning selected device index {}", index);
        index
    }

    fn set_selected_device_index(&self, index: usize, device_strategy: &dyn UiDeviceStrategy) {
        trace!("set_selected_device_index: index = {}, device_strategy = {:?}", index, device_strategy.device_type());
        let device_strategy = device_strategy.clone_box();
        self.with_main_window(move |main_window| {
            trace!("set_selected_device_index: Setting selected device index");
            device_strategy.set_selected_device_index(main_window, index as i32);
        });
    }

    fn set_devices_model(&self, device_names: &[String], device_strategy: &dyn UiDeviceStrategy) {
        trace!("set_devices_model: START");
        trace!("set_devices_model: Creating device items from port names");
        let device_items: Vec<UOptionData> =
            device_names
                .iter()
                .map(|text| UOptionData {
                    enabled: true,
                    text: text.into(),
                    value: text.into(),
                })
                .collect();
        trace!("set_devices_model: Getting device type");
        let device_type = *device_strategy.device_type();
        trace!("set_devices_model: Cloning device_strategy");
        let device_strategy = device_strategy.clone_box();
        trace!("set_devices_model: Calling with_main_window");
        self.with_main_window(move |main_window| {
            trace!("set_devices_model: Inside with_main_window closure");
            let model = match device_type {
                DeviceType::Input => {
                    let input_model = Rc::new(OptionsModel(device_items.clone()));
                    slint::ModelRc::from(input_model)
                },
                DeviceType::Output => {
                    let output_model = Rc::new(OptionsModel(device_items.clone()));
                    slint::ModelRc::from(output_model)
                },
            };
            trace!("set_devices_model: Calling device_strategy.set_devices_model");
            device_strategy.set_devices_model(main_window, model);
            trace!("set_devices_model: Done with device_strategy.set_devices_model");
        });
        trace!("set_devices_model: END");
    }

    fn show_connected_device_name(&self, name: &str, message_type: MessageType,
                                  device_strategy: &dyn UiDeviceStrategy) {
        let device_strategy = device_strategy.clone_box();
        let device_name = name.to_string();
        self.with_main_window(move |main_window| {
            device_strategy.show_connected_device_name(
                main_window, &device_name, slint_message_type(message_type));
        });
    }

    fn show_message(&self, message: &str, message_type: MessageType) {
        trace!("show_message: {}", message);
        let message = message.to_string();
        self.with_main_window(move |main_window| {
            main_window.invoke_show_message(message.into(), slint_message_type(message_type));
        });
    }

    fn show_new_version_window(&self, new_version: &str, auto_check_new_versions: bool,
                               callbacks: Arc<Mutex<dyn NewVersionCallbacks>>) {
        // let new_version_string = new_version.to_string();
        // let new_version_window_weak = self.new_version_window_weak.clone();
        // let self_clone = self.clone();
        // self.with_ui_thread(move || {
        //     let dialog_borrow = new_version_window_weak.upgrade().unwrap();
        //     // Re-setup callbacks each time as the window might be reused
        //     dialog_borrow.set_auto_check_new_versions(auto_check_new_versions);
        //     dialog_borrow.set_message(
        //         format!("Version {} of {} is available.", new_version_string, APP_TITLE).into());
        //
        //     let callbacks_clone_1 = callbacks.clone();
        //     dialog_borrow.on_auto_check_new_versions_changed(move |auto_check_new_versions: bool| {
        //         callbacks_clone_1.lock().unwrap().on_auto_check_new_versions_changed(
        //             auto_check_new_versions);
        //     });
        //     let callbacks_clone_2 = callbacks.clone();
        //     dialog_borrow.on_ignore_this_version(move || {
        //         callbacks_clone_2.lock().unwrap().on_ignore_this_version(
        //             new_version_string.clone());
        //     });
        //     dialog_borrow.on_close_window({
        //         let dialog_weak = dialog_borrow.as_weak();
        //         move || { dialog_weak.unwrap().hide().unwrap(); }
        //     });
        //
        //     self_clone.show_dialog_in_centre_of_main_window(&dialog_borrow);
        // });
    }

    fn set_main_window_position(&self, x: i32, y: i32) {
        self.with_main_window(move |main_window| {
            // Refrain from setting the window's position if the saved position is not in the
            // working area of an available monitor.
            let is_in_working_area = main_window.window().with_winit_window(|winit_window| {
                winit_window.available_monitors().any(|monitor| {
                    #[cfg(target_os = "macos")]
                    {
                        let scale = monitor.scale_factor();
                        let pos = monitor.position().to_logical::<f64>(scale);
                        let size = monitor.size().to_logical::<f64>(scale);
                        let x_min = pos.x.round() as i32;
                        let x_max = (pos.x + size.width).round() as i32;
                        let y_min = pos.y.round() as i32;
                        let y_max = (pos.y + size.height).round() as i32;
                        x >= x_min && x < x_max && y >= y_min && y < y_max
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        let pos = monitor.position();
                        let size = monitor.size();
                        let x_min = pos.x;
                        let x_max = pos.x + size.width as i32;
                        let y_min = pos.y;
                        let y_max = pos.y + size.height as i32;
                        x >= x_min && x < x_max && y >= y_min && y < y_max
                    }
                })
            }).unwrap_or(false);
            if is_in_working_area {
                #[cfg(target_os = "macos")]
                main_window.window().set_position(LogicalPosition::new(x as f32, y as f32));
                #[cfg(not(target_os = "macos"))]
                main_window.window().set_position(PhysicalPosition { x, y });
            }
        });
    }

    fn set_down_cc_no(&self, value: u8) {
        self.with_main_window(move |main_window| {
            main_window.set_down_cc_no(value as i32);
        });
    }

    fn set_up_cc_no(&self, value: u8) {
        self.with_main_window(move |main_window| {
            main_window.set_up_cc_no(value as i32);
        });
    }

    fn set_in_cc_no(&self, value: u8) {
        self.with_main_window(move |main_window| {
            main_window.set_in_cc_no(value as i32);
        });
    }

    fn set_out_cc_no(&self, value: u8) {
        self.with_main_window(move |main_window| {
            main_window.set_out_cc_no(value as i32);
        });
    }
}

fn slint_message_type(message_type: MessageType) -> SlintMessageType {
    match message_type {
        MessageType::Info => SlintMessageType::Info,
        MessageType::Warning => SlintMessageType::Warning,
        MessageType::Error => SlintMessageType::Error,
    }
}