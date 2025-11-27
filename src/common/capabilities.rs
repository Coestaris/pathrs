#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
pub struct InstanceCapabilities {
    pub debug_utils_ext: bool,
    pub validation_layer: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DeviceCapabilities {
    pub host_image_copy: bool,
}
