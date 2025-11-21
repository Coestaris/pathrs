#[derive(Clone, Copy, Debug)]
#[repr(C)]
#[repr(align(128))]
pub struct SSBO {
    pub slider: f32,
}

impl Default for SSBO {
    fn default() -> Self {
        Self { slider: 0.0 }
    }
}

impl SSBO {
    pub fn new(slider: f32) -> Self {
        Self { slider }
    }
}