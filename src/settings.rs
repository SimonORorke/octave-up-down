use std::error::Error;
use log::{debug, trace};
use serde::{Serialize, Deserialize};
use simonor::{MidiDesktopSettings, PathFinder, SettingsFileIo};
use app_info::APP_TITLE;
use crate::{CcNoRangeDefaults, CcNoRangeGetters, CcNoRangeSetters};

/// Application settings, serialised to file.
pub trait ISettings: MidiDesktopSettings + CcNoRangeGetters + CcNoRangeSetters + Send + Sync {}

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    auto_check_new_versions: bool,
    ignore_version: String,
    main_window_x: i32,
    main_window_y: i32,
    midi_input_device: String,
    midi_output_device: String,
    down_cc_no: u8,
    up_cc_no: u8,
    in_cc_no: u8,
    out_cc_no: u8,
    #[serde(skip, default = "create_settings_reader_writer")]
    io: SettingsFileIo,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            auto_check_new_versions: true,
            ignore_version: String::new(),
            main_window_x: 0,
            main_window_y: 0,
            midi_input_device: String::new(),
            midi_output_device: String::new(),
            down_cc_no: CcNoRangeDefaults::down_cc_no(),
            up_cc_no: CcNoRangeDefaults::up_cc_no(),
            in_cc_no: CcNoRangeDefaults::in_cc_no(),
            out_cc_no: CcNoRangeDefaults::out_cc_no(),
            io: create_settings_reader_writer(),
        }
    }
}
impl ISettings for Settings {}

impl MidiDesktopSettings for Settings {
    fn auto_check_new_versions(&self) -> bool { self.auto_check_new_versions }

    fn set_auto_check_new_versions(&mut self, value: bool) {
        self.auto_check_new_versions = value;
    }

    fn ignore_version(&self) -> &str { &self.ignore_version }

    fn set_ignore_version(&mut self, value: &str) { self.ignore_version = value.into() }

    fn main_window_x(&self) -> i32 {
        self.main_window_x
    }

    fn set_main_window_x(&mut self, value: i32) {
        self.main_window_x = value;
    }

    fn main_window_y(&self) -> i32 {
        self.main_window_y
    }

    fn set_main_window_y(&mut self, value: i32) {
        self.main_window_y = value;
    }

    fn midi_input_device(&self) -> &str {
        &self.midi_input_device
    }

    fn set_midi_input_device(&mut self, value: &str) {
        self.midi_input_device = value.into();
    }

    fn midi_output_device(&self) -> &str {
        &self.midi_output_device
    }

    fn set_midi_output_device(&mut self, value: &str) {
        self.midi_output_device = value.into();
    }

    fn read_from_file(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(settings) = self.io.read_from_file::<Settings>()? {
            self.auto_check_new_versions = settings.auto_check_new_versions;
            self.ignore_version = settings.ignore_version;
            self.main_window_x = settings.main_window_x;
            self.main_window_y = settings.main_window_y;
            self.midi_input_device = settings.midi_input_device;
            self.midi_output_device = settings.midi_output_device;
            self.down_cc_no = settings.down_cc_no;
            self.up_cc_no = settings.up_cc_no;
            self.in_cc_no = settings.in_cc_no;
            self.out_cc_no = settings.out_cc_no;
        }
        trace!("read_from_file: self.midi_input_device = {}; self.midi_output_device = {}", 
            self.midi_input_device, self.midi_output_device);
        Ok(())
    }

    fn write_to_file(&mut self) -> Result<(), Box<dyn Error>> {
        self.io.write_to_file(&self)?;
        trace!("write_to_file: self.down_cc_no = {}; self.up_cc_no = {}",
            self.down_cc_no, self.up_cc_no);
        Ok(())
    }

    /// Replaces the default system path finder for testing.
    fn set_system_path_finder(&mut self, path_finder: Box<dyn PathFinder>) {
        self.io.set_system_path_finder(path_finder);
    }
}

impl CcNoRangeGetters for Settings {
    fn down_cc_no(&self) -> u8 {
        self.down_cc_no
    }

    fn up_cc_no(&self) -> u8 {
        self.up_cc_no
    }

    fn in_cc_no(&self) -> u8 {
        self.in_cc_no
    }

    fn out_cc_no(&self) -> u8 {
        self.out_cc_no
    }
}

impl CcNoRangeSetters for Settings {
    fn set_down_cc_no(&mut self, value: u8) {
        self.down_cc_no = value;
    }

    fn set_up_cc_no(&mut self, value: u8) { self.up_cc_no = value; }

    fn set_in_cc_no(&mut self, value: u8) {
        self.in_cc_no = value;
    }

    fn set_out_cc_no(&mut self, value: u8) { self.out_cc_no = value; }
}

fn create_settings_reader_writer() -> SettingsFileIo {
    SettingsFileIo::new(APP_TITLE.to_string(), SETTINGS_FILE_NAME.to_string())
}

const SETTINGS_FILE_NAME: &str = "Settings.toml";
