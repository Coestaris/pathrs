use ash::vk;
use std::mem::offset_of;

#[repr(C)]
#[repr(packed)]
pub struct QuadVertex {
    pub pos: [f32; 2],
    pub color: [f32; 3],
}

impl QuadVertex {
    pub fn new(pos: [f32; 2], color: [f32; 3]) -> Self {
        Self { pos, color }
    }

    pub fn get_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<QuadVertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }

    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(QuadVertex, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(QuadVertex, color) as u32,
            },
        ]
    }
}
