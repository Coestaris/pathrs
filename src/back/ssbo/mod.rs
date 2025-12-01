use crate::tracer::Bundle;
use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme};
use gpu_allocator::MemoryLocation;
use log::debug;
use std::fmt::Debug;

pub mod config;
pub mod objects;
pub struct SSBO<T> {
    pub buffer: vk::Buffer,
    pub allocation: Option<Allocation>,
    pub destroyed: bool,

    _marker: std::marker::PhantomData<T>,
}

impl<T> SSBO<T>
where
    T: Debug,
{
    pub unsafe fn new(bundle: Bundle, option: Option<&str>) -> anyhow::Result<Self> {
        let buffer_create_info = vk::BufferCreateInfo::default()
            .size(size_of::<T>() as vk::DeviceSize)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer = bundle.device.create_buffer(&buffer_create_info, None)?;
        let reqs = bundle.device.get_buffer_memory_requirements(buffer);

        let allocation = bundle.allocator().allocate(&AllocationCreateDesc {
            name: option.as_deref().unwrap_or("SSBO Buffer"),
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
            _marker: std::marker::PhantomData,
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

    pub unsafe fn update(&mut self, data: T) {
        debug!("Updating SSBO: {:?}", data);
        let mapped = self.allocation.as_ref().unwrap().mapped_ptr().unwrap();
        let dst = mapped.as_ptr() as *mut T;
        dst.copy_from_nonoverlapping(&data, 1);
    }
}

impl<T> Drop for SSBO<T> {
    fn drop(&mut self) {
        if !self.destroyed {
            panic!("SSBO must be destroyed before being dropped");
        }
    }
}
