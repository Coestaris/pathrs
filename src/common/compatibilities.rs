pub struct InstanceCompatibilities {
    pub debug_utils_ext: bool,
    pub validation_layer: bool,
}

impl Default for InstanceCompatibilities {
    fn default() -> Self {
        Self {
            debug_utils_ext: false,
            validation_layer: false,
        }
    }
}

pub struct DeviceCompatibilities {}

impl Default for DeviceCompatibilities {
    fn default() -> Self {
        Self {}
    }
}
