pub mod ui_device_strategy;
pub mod ui_methods;

pub use ui_device_strategy::{UiDeviceStrategy, UiInputStrategy, UiOutputStrategy};
pub use ui_methods::{IUiMethods, MessageType, UiMethods};

/// The model for the list of options to be displayed in a `USelect` or `URadio`.
pub struct OptionsModel(pub Vec<UOptionData>);

impl slint::Model for OptionsModel {
    type Data = UOptionData;
    fn row_count(&self) -> usize {
        self.0.len()
    }
    fn row_data(&self, row: usize) -> Option<Self::Data> {
        self.0.get(row).cloned()
    }
    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        &()
    }
}

slint::include_modules!();
