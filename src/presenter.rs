use std::error::Error;
use std::sync::{Arc, Mutex, Weak};
use log::{debug, trace};
use simonor::{IMidiHub, IMidiSender, MidiHub, MidiSender, ReleaseInfo,
              SharedMidiHub, SharedMidiSender, SharedReleaseInfo, SharedOutput, VersionChecker};
use app_info::{APP_TITLE, RELEASES_LINK, VERSION};
use crate::{IMidiProcessor, ISettings, IUiMethods, MidiProcessor, Presentation, Settings,
            SharedMidiProcessor, UiDeviceStrategy, UiInputStrategy, UiOutputStrategy};

pub struct Presenter {
    midi_hub: SharedMidiHub,
    midi_processor: SharedMidiProcessor,
    presentation: Presentation,
    presenter_weak: Weak<Mutex<Presenter>>,
    release_info: SharedReleaseInfo,
    settings: Box<dyn ISettings>,
}

impl Presenter {
    pub fn new(callbacks: Arc<dyn IUiMethods>) -> Self {
        let output: SharedOutput = Arc::new(Mutex::new(None));
        let midi_sender: SharedMidiSender =
            Arc::new(Mutex::new(Box::new(MidiSender::new(output.clone()))
                as Box<dyn IMidiSender + Send>));
        let midi_processor: SharedMidiProcessor =
            Arc::new(Mutex::new(Box::new(
                MidiProcessor::new(midi_sender.clone())) as Box<dyn IMidiProcessor>));
        let midi_hub: SharedMidiHub =
            Arc::new(Mutex::new(Box::new(MidiHub::new(
                output.clone(), midi_processor.clone()))
                as Box<dyn IMidiHub + Send>));
        let presentation = Presentation::new(callbacks);
        let presenter_weak: Weak<Mutex<Presenter>> = Weak::new();
        let release_info: SharedReleaseInfo =
            Arc::new(Mutex::new(Box::new(ReleaseInfo::new(
                APP_TITLE.into(), RELEASES_LINK.into()))));
        let settings: Box<dyn ISettings> = Box::new(Settings::new());
        Self {
            midi_hub,
            midi_processor,
            presentation,
            presenter_weak,
            release_info,
            settings,
        }
    }

    pub fn init(&mut self, self_arc: &SharedPresenter) {
        trace!("init");
        self.presenter_weak = Arc::downgrade(self_arc);
        let auto_check_new_versions: bool;
        let ignore_version: String;
        let main_window_x: i32;
        let main_window_y: i32;
        let input_device_name: String;
        let output_device_name: String;
        let down_cc_no: u8;
        let up_cc_no: u8;
        let in_cc_no: u8;
        let out_cc_no: u8;
        trace!("init: Reading settings");
        match self.settings.read_from_file() {
            Ok(_) => {
                auto_check_new_versions = self.settings.auto_check_new_versions();
                ignore_version = self.settings.ignore_version().to_string();
                main_window_x = self.settings.main_window_x();
                main_window_y = self.settings.main_window_y();
                input_device_name = self.settings.midi_input_device().to_string();
                output_device_name = self.settings.midi_output_device().to_string();
                down_cc_no = self.settings.down_cc_no();
                up_cc_no = self.settings.up_cc_no();
                in_cc_no = self.settings.in_cc_no();
                out_cc_no = self.settings.out_cc_no();
            }
            Err(err) => {
                self.presentation.show_error(&err.to_string());
                return;
            }
        }
        self.presentation.set_main_window_position(main_window_x, main_window_y);
        if auto_check_new_versions {
            trace!("init: Checking for new version");
            self.find_new_version(true, &ignore_version);
        }
        trace!("init: Getting midi");
        let mut midi = self.midi_hub.lock().unwrap();
        midi.init(&input_device_name, &output_device_name);
        drop(midi); // Release MIDI lock before calling device_names which needs to acquire it
        let input_strategy = UiInputStrategy::new();
        let output_strategy = UiOutputStrategy::new();
        trace!("init: Getting input device names");
        let input_device_names = self.device_names(&input_strategy);
        trace!("init: Got {} input device names", input_device_names.len());
        trace!("init: About to call callbacks.set_devices_model");
        self.presentation.set_devices_model(&input_device_names, &input_strategy);
        trace!("init: Called callbacks.set_devices_model");
        trace!("init: Setting output devices model");
        self.presentation.set_devices_model(&self.device_names(&output_strategy), &output_strategy);
        trace!("init: Connecting initial MIDI devices");
        self.connect_initial_device(&output_strategy);
        self.connect_initial_device(&input_strategy);
        let mut midi_processor = self.midi_processor.lock().unwrap();
        midi_processor.set_down_cc_no(down_cc_no);
        midi_processor.set_up_cc_no(up_cc_no);
        midi_processor.set_in_cc_no(in_cc_no);
        midi_processor.set_out_cc_no(out_cc_no);
        self.presentation.set_down_cc_no(down_cc_no);
        self.presentation.set_up_cc_no(up_cc_no);
        self.presentation.set_in_cc_no(in_cc_no);
        self.presentation.set_out_cc_no(out_cc_no);
        trace!("init: Done");
    }

    pub fn check_for_updates(&mut self) {
        trace!("check_for_updates");
        let auto_check_new_versions = self.settings.auto_check_new_versions();
        let ignore_version = self.settings.ignore_version().to_string();
        if !self.find_new_version(auto_check_new_versions, &ignore_version) {
            self.presentation.already_running_latest_version();
        }
        trace!("check_for_updates: Done");
    }

    pub fn close(&mut self, main_window_x: i32, main_window_y: i32) -> Result<(), Box<dyn Error>> {
        self.midi_hub.lock().unwrap_or_else(|e| e.into_inner()).close();
        self.settings.set_main_window_x(main_window_x);
        self.settings.set_main_window_y(main_window_y);
        if let Err(err) = self.settings.write_to_file() {
            self.presentation.show_error(&err.to_string());
            return Err(err)
        };
        Ok(())
    }

    fn connect_initial_device(&mut self, device_strategy: &dyn UiDeviceStrategy) {
        trace!("connect_initial_port: {:?}", device_strategy.device_type());
        let shared_midi = self.midi_hub.clone();
        let maybe_index = {
            let midi = shared_midi.lock().unwrap();
            midi.io(device_strategy.midi_strategy()).device().as_ref()
                .map(|device| device.index())
        };
        if let Some(index) = maybe_index {
            trace!("connect_initial_port: Setting selected port index to {}", index);
            self.presentation.set_selected_device_index(index, device_strategy);
            self.connect_selected_device(&shared_midi, device_strategy);
            self.presentation.midi_connected();
        } else {
            self.presentation.no_device_connected(device_strategy);
            self.presentation.show_warning(device_strategy.msg_connect());
            self.presentation.focus_device(device_strategy);
        }
    }

    pub fn connect_device(&mut self, device_strategy: &dyn UiDeviceStrategy) {
        trace!("connect_device");
        let shared_midi = self.midi_hub.clone();
        let device_strategy = device_strategy.clone_box();
        trace!("connect_device: Connecting selected port");
        self.connect_selected_device(&shared_midi, &*device_strategy);
        trace!("connect_device: Getting port");
        let device_name_opt: Option<String> = shared_midi.lock().unwrap()
            .io(device_strategy.midi_strategy())
            .device()
            .map(|p| p.name().to_string());
        trace!("connect_device: Got port");
        if let Some(device_name) = device_name_opt {
            self.presentation.show_info(device_strategy.msg_connected(&device_name));
        }
        trace!("connect_device: Done");
    }

    fn connect_selected_device(&mut self, shared_midi: &SharedMidiHub,
                               device_strategy: &dyn UiDeviceStrategy) {
        trace!("connect_selected_device: {:?}", device_strategy.device_type());
        let index = self.presentation.get_selected_device_index(device_strategy);
        // No selection (e.g. an empty device list) leaves the combobox at -1, which `UiMethods`
        // converts to `usize::MAX`. That is handled silently by the `device_names().get(index)`
        // guard below: clicking Connect with nothing to connect to simply does nothing.
        trace!("connect_selected_device: Selected port index = {}", index);
        let ui_action: Result<String, String> = {
            trace!("connect_selected_device: Getting midi.");
            let mut midi = shared_midi.lock().unwrap();
            trace!("connect_selected_device: Got midi.");
            let Some(name) = midi.io(device_strategy.midi_strategy()).device_names().get(index).cloned()
            else {
                return;
            };
            match midi.connect_device(index, device_strategy.midi_strategy()) {
                Ok(()) => Ok(name),
                Err(err) => Err(err.to_string()),
            }
        };
        match ui_action {
            Ok(name) => {
                self.presentation.connected_device(&name, device_strategy);
                // Persist the connected device so it is reselected on next startup. Saved only
                // here, where a real device has just connected (never DEVICE_NONE), so the player
                // can close the application and reopen it later — when a temporarily unavailable
                // device is available again (for example a USB-MIDI cable that was unplugged) —
                // and still have the same device automatically selected and connected on startup.
                device_strategy.set_device_setting(&mut *self.settings, &name);
            }
            Err(message) => {
                self.presentation.no_device_connected(device_strategy);
                self.presentation.show_error(&message);
            }
        }
    }

    fn device_names(&self, device_strategy: &dyn UiDeviceStrategy) -> Vec<String> {
        self.midi_hub.lock().unwrap().io(device_strategy.midi_strategy()).device_names()
    }

    fn find_new_version(&mut self, auto_check_new_versions: bool, ignore_version: &str) -> bool {
        let version_checker = VersionChecker::new(
            self.release_info.clone(), VERSION.into());
        if let Some(new_version) = version_checker.check_for_new_version(ignore_version) {
            let callbacks =
                Arc::new(Mutex::new(NewVersionCallbacksWrapper::new(self.presenter_weak.clone())));
            self.presentation.show_new_version_window(
                &new_version, auto_check_new_versions, callbacks);
            return true;
        }
        false
    }

    pub fn refresh_devices(&mut self, device_strategy: &dyn UiDeviceStrategy) {
        let device_strategy = device_strategy.clone_box();
        let device_name = device_strategy.device_setting(&*self.settings).to_string();
        self.midi_hub.lock().unwrap().refresh_devices(&device_name, device_strategy.midi_strategy());
        self.presentation.set_devices_model(&self.device_names(&*device_strategy), &*device_strategy);
        self.presentation.no_device_connected(&*device_strategy);
        self.presentation.show_warning(device_strategy.msg_refreshed_reconnect());
    }

    pub fn set_down_cc_no(&mut self, value: u8) {
        self.midi_processor.lock().unwrap().set_down_cc_no(value);
        self.settings.set_down_cc_no(value);
    }

    pub fn set_up_cc_no(&mut self, value: u8) {
        self.midi_processor.lock().unwrap().set_up_cc_no(value);
        self.settings.set_up_cc_no(value);
    }

    pub fn set_in_cc_no(&mut self, value: u8) {
        self.midi_processor.lock().unwrap().set_in_cc_no(value);
        self.settings.set_in_cc_no(value);
    }

    pub fn set_out_cc_no(&mut self, value: u8) {
        self.midi_processor.lock().unwrap().set_out_cc_no(value);
        self.settings.set_out_cc_no(value);
    }
}

pub trait NewVersionCallbacks : Send + Sync {
    fn on_auto_check_new_versions_changed(&mut self, auto_check_new_versions: bool);
    fn on_ignore_this_version(&mut self, new_version: String);
}

impl NewVersionCallbacks for Presenter {
    fn on_auto_check_new_versions_changed(&mut self, auto_check_new_versions: bool) {
        self.settings.set_auto_check_new_versions(auto_check_new_versions);
    }
    fn on_ignore_this_version(&mut self, new_version: String) {
        self.settings.set_ignore_version(&new_version);
    }
}

pub struct NewVersionCallbacksWrapper(Weak<Mutex<Presenter>>);

impl NewVersionCallbacksWrapper {
    pub fn new(presenter_weak: Weak<Mutex<Presenter>>) -> Self {
        Self(presenter_weak)
    }
}

impl NewVersionCallbacks for NewVersionCallbacksWrapper {
    fn on_auto_check_new_versions_changed(&mut self, auto_check_new_versions: bool) {
        if let Some(presenter) = self.0.upgrade() {
            presenter.lock().unwrap().on_auto_check_new_versions_changed(auto_check_new_versions);
        }
    }
    fn on_ignore_this_version(&mut self, new_version: String) {
        if let Some(presenter) = self.0.upgrade() {
            presenter.lock().unwrap().on_ignore_this_version(new_version);
        }
    }
}

pub type SharedPresenter = Arc<Mutex<Presenter>>;
