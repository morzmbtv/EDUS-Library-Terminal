use crate::domain::{AppError, Diagnostics};

#[derive(Clone, Default)]
pub struct DeviceService;
impl DeviceService {
    pub fn new() -> Self {
        Self
    }
    /// Scanner and NFC readers normally operate as keyboard-wedge devices; the
    /// Vue layer forwards their completed input to the same local command path.
    pub fn card_reader_status(&self) -> &'static str {
        "HID_NOT_VERIFIED_ON_PHYSICAL_HARDWARE"
    }
    pub fn diagnostics(&self, mut value: Diagnostics) -> Diagnostics {
        value.card_reader = self.card_reader_status().into();
        value
    }
    pub fn assert_camera_policy(&self) -> Result<(), AppError> {
        Ok(())
    }
}
