use crate::common::buffer::create_device_local_buffer_with_data;
use crate::common::command_buffer::CommandBuffer;
use crate::tracer::Bundle;
use ash::{vk, Device};
use gpu_allocator::vulkan::{Allocation, Allocator};
use std::mem::offset_of;

// Fullscreen quad vertices
const VERTICES: [QuadVertex; 4] = [
    QuadVertex::new([-1.0, -1.0], [0.0, 0.0]),
    QuadVertex::new([1.0, -1.0], [1.0, 0.0]),
    QuadVertex::new([1.0, 1.0], [1.0, 1.0]),
    QuadVertex::new([-1.0, 1.0], [0.0, 1.0]),
];

const INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct QuadVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
}

impl QuadVertex {
    pub const fn new(pos: [f32; 2], uv: [f32; 2]) -> Self {
        Self { pos, uv }
    }

    pub fn get_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<QuadVertex>() as u32,
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
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(QuadVertex, uv) as u32,
            },
        ]
    }
}

pub struct QuadBuffer {
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_allocation: Option<Allocation>,
    pub index_buffer: vk::Buffer,
    pub index_buffer_allocation: Option<Allocation>,
    pub destroyed: bool,
}

impl QuadBuffer {
    pub unsafe fn new(
        bundle: Bundle,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
    ) -> anyhow::Result<Self> {
        let (vertex_buffer, vertex_alloc) = create_device_local_buffer_with_data(
            bundle,
            command_pool,
            queue,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            &VERTICES,
            "Quad Vertex Buffer Allocation",
        )?;

        let (index_buffer, index_alloc) = create_device_local_buffer_with_data(
            bundle,
            command_pool,
            queue,
            vk::BufferUsageFlags::INDEX_BUFFER,
            &INDICES,
            "Quad Index Buffer Allocation",
        )?;

        Ok(Self {
            vertex_buffer,
            vertex_buffer_allocation: Some(vertex_alloc),
            index_buffer,
            index_buffer_allocation: Some(index_alloc),
            destroyed: false,
        })
    }

    pub unsafe fn destroy(&mut self, bundle: Bundle) {
        if self.destroyed {
            return;
        }

        if let Some(allocation) = self.vertex_buffer_allocation.take() {
            bundle
                .allocator()
                .free(allocation)
                .expect("Failed to free vertex buffer allocation");
        }
        bundle.device.destroy_buffer(self.vertex_buffer, None);

        if let Some(allocation) = self.index_buffer_allocation.take() {
            bundle
                .allocator()
                .free(allocation)
                .expect("Failed to free index buffer allocation");
        }
        bundle.device.destroy_buffer(self.index_buffer, None);
        self.destroyed = true;
    }

    pub unsafe fn draw(&self, bundle: Bundle, command_buffer: &CommandBuffer) {
        bundle.device.cmd_bind_vertex_buffers(
            command_buffer.as_inner(),
            0,
            &[self.vertex_buffer],
            &[0],
        );
        bundle.device.cmd_bind_index_buffer(
            command_buffer.as_inner(),
            self.index_buffer,
            0,
            vk::IndexType::UINT16,
        );
        bundle.device.cmd_bind_index_buffer(
            command_buffer.as_inner(),
            self.index_buffer,
            0,
            vk::IndexType::UINT16,
        );
        bundle
            .device
            .cmd_draw_indexed(command_buffer.as_inner(), 6, 1, 0, 0, 0);
    }
}

impl Drop for QuadBuffer {
    fn drop(&mut self) {
        if !self.destroyed {
            panic!("QuadBuffer was not destroyed before being dropped");
        }
    }
}
