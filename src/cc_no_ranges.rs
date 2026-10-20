/// Trait for accessing control change number ranges.
pub trait CcNoRangeGetters {
    fn down_cc_no(&self) -> u8;
    fn up_cc_no(&self) -> u8;
    fn in_cc_no(&self) -> u8;
    fn out_cc_no(&self) -> u8;
}

/// Trait for modifying control change number ranges.
pub trait CcNoRangeSetters {
    fn set_down_cc_no(&mut self, value: u8);
    fn set_up_cc_no(&mut self, value: u8);
    fn set_in_cc_no(&mut self, value: u8);
    fn set_out_cc_no(&mut self, value: u8);
}

/// Defaults for control change number ranges.
pub struct CcNoRangeDefaults {
}

impl CcNoRangeDefaults {
    pub fn down_cc_no() -> u8 { 64 }
    pub fn up_cc_no() -> u8 { 66 }
    pub fn in_cc_no() -> u8 { 11 }
    pub fn out_cc_no() -> u8 { 11 }
}