#[allow(dead_code)]
pub struct InstanceCapabilities {
    pub debug_utils_ext: bool,
    pub validation_layer: bool,
}

impl Default for InstanceCapabilities {
    fn default() -> Self {
        Self {
            debug_utils_ext: false,
            validation_layer: false,
        }
    }
}

pub struct DeviceCapabilities {}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {}
    }
}
