pub mod ui_device_strategy;

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
