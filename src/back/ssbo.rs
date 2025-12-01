use crate::tracer::Bundle;
use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme};
use gpu_allocator::MemoryLocation;
use log::debug;

#[derive(Default, Clone, Debug)]
#[repr(C)]
#[repr(align(128))]
pub struct ParametersSSBOData {
    pub camera_transform: [[f32; 4]; 4],
    pub camera_fov: [f32; 4],
}

pub struct ParametersSSBO {
    pub buffer: vk::Buffer,
    pub allocation: Option<Allocation>,
    pub destroyed: bool,
}

impl ParametersSSBO {
    pub unsafe fn new(bundle: Bundle) -> anyhow::Result<Self> {
        let buffer_create_info = vk::BufferCreateInfo::default()
            .size(size_of::<ParametersSSBOData>() as vk::DeviceSize)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer = bundle.device.create_buffer(&buffer_create_info, None)?;
        let reqs = bundle.device.get_buffer_memory_requirements(buffer);

        let allocation = bundle.allocator().allocate(&AllocationCreateDesc {
            name: "Parameters SSBO Buffer",
            requirements: reqs,
            location: MemoryLocation::CpuToGpu,
            linear: true,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        })?;
        bundle
            .device
            .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())?;

        Ok(Self {
            buffer,
            allocation: Some(allocation),
            destroyed: false,
        })
    }

    pub unsafe fn destroy(&mut self, bundle: Bundle) {
        if !self.destroyed {
            if let Some(allocation) = self.allocation.take() {
                bundle.allocator().free(allocation).unwrap();
            }
            bundle.device.destroy_buffer(self.buffer, None);

            self.destroyed = true;
        }
    }

    pub unsafe fn update(&mut self, data: ParametersSSBOData) {
        debug!("Updating ParametersSSBO: {:?}", data);
        let mapped = self.allocation.as_ref().unwrap().mapped_ptr().unwrap();
        let dst = mapped.as_ptr() as *mut ParametersSSBOData;
        dst.copy_from_nonoverlapping(&data, 1);
    }
}

impl Drop for ParametersSSBO {
    fn drop(&mut self) {
        if !self.destroyed {
            panic!("ParametersSSBO must be destroyed before being dropped");
        }
    }
}
