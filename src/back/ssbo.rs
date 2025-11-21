use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};
use gpu_allocator::MemoryLocation;
use log::debug;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
#[repr(align(128))]
pub struct ParametersSSBOData {
    pub slider: f32,
}

impl PartialEq for ParametersSSBOData {
    fn eq(&self, other: &Self) -> bool {
        (self.slider - other.slider).abs() < f32::EPSILON
    }
}

impl Default for ParametersSSBOData {
    fn default() -> Self {
        Self { slider: 0.0 }
    }
}

impl ParametersSSBOData {
    pub fn new(slider: f32) -> Self {
        Self { slider }
    }
}

pub struct ParametersSSBO {
    pub data: ParametersSSBOData,
    pub buffer: vk::Buffer,
    pub allocation: Option<Allocation>,
    pub destroyed: bool,
}

impl ParametersSSBO {
    pub unsafe fn new(device: &ash::Device, allocator: &mut Allocator) -> anyhow::Result<Self> {
        let buffer_create_info = vk::BufferCreateInfo::default()
            .size(size_of::<ParametersSSBOData>() as vk::DeviceSize)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer = device.create_buffer(&buffer_create_info, None)?;
        let reqs = device.get_buffer_memory_requirements(buffer);

        let allocation = allocator.allocate(&AllocationCreateDesc {
            name: "Parameters SSBO Buffer",
            requirements: reqs,
            location: MemoryLocation::CpuToGpu,
            linear: true,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        })?;
        device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())?;

        Ok(Self {
            data: ParametersSSBOData::default(),
            buffer,
            allocation: Some(allocation),
            destroyed: false,
        })
    }

    pub unsafe fn destroy(&mut self, device: &ash::Device, allocator: &mut Allocator) {
        if !self.destroyed {
            if let Some(allocation) = self.allocation.take() {
                allocator.free(allocation).unwrap();
            }
            device.destroy_buffer(self.buffer, None);

            self.destroyed = true;
        }
    }

    pub unsafe fn update(&mut self, data: ParametersSSBOData) {
        if self.data != data {
            self.data = data;
            debug!("Updating ParametersSSBO: {:?}", self.data);
            let mapped = self.allocation.as_ref().unwrap().mapped_ptr().unwrap();
            let dst = mapped.as_ptr() as *mut ParametersSSBOData;
            dst.copy_from_nonoverlapping(&self.data, 1);
        }
    }
}

impl Drop for ParametersSSBO {
    fn drop(&mut self) {
        if !self.destroyed {
            panic!("ParametersSSBO must be destroyed before being dropped");
        }
    }
}
