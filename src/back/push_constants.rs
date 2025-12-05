use ash::vk;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
#[repr(align(128))]
pub struct PushConstantsData {
    pub frame_index: u32,
    pub invalidate: u32,
}

impl Default for PushConstantsData {
    fn default() -> Self {
        Self {
            frame_index: 0,
            invalidate: 0,
        }
    }
}

impl PushConstantsData {
    pub fn get_range() -> vk::PushConstantRange {
        vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            offset: 0,
            size: std::mem::size_of::<PushConstantsData>() as u32,
        }
    }

    pub fn new(frame_index: u32) -> Self {
        Self {
            frame_index,
            invalidate: 0,
        }
    }
}
