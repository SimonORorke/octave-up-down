use std::sync::{Arc};
use log::{trace};
use crate::{IUiMethods, MessageType, UiDeviceStrategy};

/// The view-facing facade the `Presenter` speaks through. If need be, it can be shared with other
/// types via `Clone`.
///
/// It owns the injected `IUiMethods` view seam and every user-facing message string (the `const`s
/// below), exposing intention-named methods so that each
/// string and its severity live in one place rather than being chosen at the call site. It decides
/// *nothing* about when to show a message — that is the Presenter's job; this type only formats and
/// forwards to the view.
///
/// It is a concrete type, not an `I`-prefixed service interface: tests mock the underlying
/// `IUiMethods`, so the facade needs no interface of its own (see `IUiMethods`'s doc comment for the
/// two-tier trait-naming rule).
#[derive(Clone)]
pub struct Presentation {
    ui_methods: Arc<dyn IUiMethods>,
}

impl Presentation {
    pub(crate) fn new(ui_methods: Arc<dyn IUiMethods>) -> Self {
        Self { ui_methods }
    }

    // --- Structural view setup (thin passthroughs to the view) ---

    pub(crate) fn set_main_window_position(&self, x: i32, y: i32) {
        self.ui_methods.set_main_window_position(x, y);
    }

    pub(crate) fn set_devices_model(&self, device_names: &[String],
                                    device_strategy: &dyn UiDeviceStrategy) {
        self.ui_methods.set_devices_model(device_names, device_strategy);
    }

    pub(crate) fn get_selected_device_index(&self, device_strategy: &dyn UiDeviceStrategy) -> usize {
        self.ui_methods.get_selected_device_index(device_strategy)
    }

    pub(crate) fn set_selected_device_index(&self, index: usize,
                                            device_strategy: &dyn UiDeviceStrategy) {
        self.ui_methods.set_selected_device_index(index, device_strategy);
    }

    pub(crate) fn focus_device(&self, device_strategy: &dyn UiDeviceStrategy) {
        self.ui_methods.focus_device(device_strategy);
    }

    pub(crate) fn connected_device(&self, device_name: &str,
                                   device_strategy: &dyn UiDeviceStrategy) {
        self.ui_methods.show_connected_device_name(device_name, MessageType::Info, device_strategy);
    }


    pub(crate) fn no_device_connected(&self, device_strategy: &dyn UiDeviceStrategy) {
        self.ui_methods.show_connected_device_name(
            DEVICE_NONE, MessageType::Warning, device_strategy);
    }

    // --- Generic messages whose text is dynamic (a Result error or a DeviceStrategy string) ---

    pub(crate) fn show_error(&self, message: &str) {
        self.ui_methods.show_message(message, MessageType::Error);
    }

    pub(crate) fn show_info(&self, message: impl AsRef<str>) {
        self.ui_methods.show_message(message.as_ref(), MessageType::Info);
    }

    pub(crate) fn show_warning(&self, message: &str) {
        trace!("show_warning: {message}");
        self.ui_methods.show_message(message, MessageType::Warning);
    }

    // --- Intention-named status messages (string + severity owned here) ---

    pub(crate) fn midi_send_error(&self) { self.show_error(MIDI_SEND_ERROR); }
}

// User-facing message strings. Owned here so every string the player sees lives in one module.
// `DEVICE_NONE` is the sentinel shown (and recorded by tests) when no MIDI device is connected.
pub const DEVICE_NONE: &str = "[None]";
pub const MIDI_SEND_ERROR: &str = "MIDI send error. Try restarting the application and/or editor \
    and/or instrument";