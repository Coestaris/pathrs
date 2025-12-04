use crate::back::ssbo::SSBO;

#[derive(Default, Clone, Debug)]
#[repr(C)]
#[repr(align(128))]
pub struct SSBOConfigData {
    pub camera_transform: [[f32; 4]; 4],
    pub camera_fov: f32,
    pub objects_count: u32,
    pub samples_count: u32,
    pub jitter_strength: f32,
    pub temporal_accumulation: f32,
}

pub type SSBOConfig = SSBO<SSBOConfigData>;
