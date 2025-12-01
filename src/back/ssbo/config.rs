use crate::back::ssbo::SSBO;

#[derive(Default, Clone, Debug)]
#[repr(C)]
#[repr(align(128))]
pub struct SSBOConfigData {
    pub camera_transform: [[f32; 4]; 4],
    pub camera_fov: f32,
    pub objects_count: [u32; 4],
}

pub type SSBOConfig = SSBO<SSBOConfigData>;
