use ash::vk;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
#[repr(align(128))]
pub struct PushConstants {
    time: f32,
}

impl Default for PushConstants {
    fn default() -> Self {
        Self { time: 0.0 }
    }
}

impl PushConstants {
    pub fn get_range() -> vk::PushConstantRange {
        vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            offset: 0,
            size: std::mem::size_of::<PushConstants>() as u32,
        }
    }

    pub fn new(time: f32) -> Self {
        Self { time }
    }
}
