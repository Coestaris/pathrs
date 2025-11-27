use crate::common::command_buffer::CommandBuffer;
use crate::tracer::Bundle;
use ash::{vk, Device};
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};
use gpu_allocator::MemoryLocation;

pub unsafe fn create_device_local_buffer_with_data<T: Copy>(
    bundle: Bundle,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    usage: vk::BufferUsageFlags,
    data: &[T],
    name: &'static str,
) -> anyhow::Result<(vk::Buffer, Allocation)> {
    let buffer_size = size_of_val(data) as vk::DeviceSize;
    let buffer_info = vk::BufferCreateInfo::default()
        .size(buffer_size)
        .usage(usage | vk::BufferUsageFlags::TRANSFER_DST)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = bundle.device.create_buffer(&buffer_info, None)?;
    let reqs = bundle.device.get_buffer_memory_requirements(buffer);
    let allocation = bundle.allocator().allocate(&AllocationCreateDesc {
        name,
        requirements: reqs,
        location: MemoryLocation::GpuOnly,
        linear: true,
        allocation_scheme: AllocationScheme::GpuAllocatorManaged,
    })?;
    bundle
        .device
        .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())?;

    let staging_info = vk::BufferCreateInfo::default()
        .size(buffer_size)
        .usage(vk::BufferUsageFlags::TRANSFER_SRC)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let staging_buffer = bundle.device.create_buffer(&staging_info, None)?;
    let staging_reqs = bundle.device.get_buffer_memory_requirements(staging_buffer);
    let staging_alloc = bundle.allocator().allocate(&AllocationCreateDesc {
        name: "Staging buffer",
        requirements: staging_reqs,
        location: MemoryLocation::CpuToGpu,
        linear: true,
        allocation_scheme: AllocationScheme::GpuAllocatorManaged,
    })?;
    bundle.device.bind_buffer_memory(
        staging_buffer,
        staging_alloc.memory(),
        staging_alloc.offset(),
    )?;

    {
        let mapped = staging_alloc
            .mapped_ptr()
            .expect("CpuToGpu allocation must be mappable");
        let dst = mapped.as_ptr() as *mut T;
        dst.copy_from_nonoverlapping(data.as_ptr(), data.len());
        // allocator.flush(&staging_alloc, 0, buffer_size)?;
    }

    let mut command_buffer = CommandBuffer::new_from_pool(bundle, command_pool)?;

    command_buffer.begin(bundle)?;

    let copy_region = vk::BufferCopy::default().size(buffer_size);
    bundle.device.cmd_copy_buffer(
        command_buffer.as_inner(),
        staging_buffer,
        buffer,
        &[copy_region],
    );

    command_buffer.end(bundle)?;

    let submit_info = command_buffer.as_submit_info();

    bundle
        .device
        .queue_submit(queue, &[submit_info], vk::Fence::null())?;
    bundle.device.queue_wait_idle(queue)?;

    command_buffer.destroy(bundle, command_pool);

    bundle.allocator().free(staging_alloc)?;
    bundle.device.destroy_buffer(staging_buffer, None);

    Ok((buffer, allocation))
}
