use ash::{vk, Device};
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};
use gpu_allocator::MemoryLocation;
use std::mem::size_of;

pub unsafe fn create_device_local_buffer_with_data<T: Copy>(
    device: &Device,
    allocator: &mut Allocator,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    usage: vk::BufferUsageFlags,
    data: &[T],
    name: &'static str,
) -> anyhow::Result<(vk::Buffer, Allocation)> {
    let buffer_size = (size_of::<T>() * data.len()) as vk::DeviceSize;
    let buffer_info = vk::BufferCreateInfo::default()
        .size(buffer_size)
        .usage(usage | vk::BufferUsageFlags::TRANSFER_DST)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = device.create_buffer(&buffer_info, None)?;
    let reqs = device.get_buffer_memory_requirements(buffer);
    let allocation = allocator.allocate(&AllocationCreateDesc {
        name,
        requirements: reqs,
        location: MemoryLocation::GpuOnly,
        linear: true,
        allocation_scheme: AllocationScheme::GpuAllocatorManaged,
    })?;
    device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())?;

    let staging_info = vk::BufferCreateInfo::default()
        .size(buffer_size)
        .usage(vk::BufferUsageFlags::TRANSFER_SRC)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let staging_buffer = device.create_buffer(&staging_info, None)?;
    let staging_reqs = device.get_buffer_memory_requirements(staging_buffer);
    let staging_alloc = allocator.allocate(&AllocationCreateDesc {
        name: "Staging buffer",
        requirements: staging_reqs,
        location: MemoryLocation::CpuToGpu,
        linear: true,
        allocation_scheme: AllocationScheme::GpuAllocatorManaged,
    })?;
    device.bind_buffer_memory(
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

    let cmd_alloc_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);
    let cmd_buf = device.allocate_command_buffers(&cmd_alloc_info)?[0];

    let begin_info = vk::CommandBufferBeginInfo::default()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    device.begin_command_buffer(cmd_buf, &begin_info)?;

    let copy_region = vk::BufferCopy {
        src_offset: 0,
        dst_offset: 0,
        size: buffer_size,
    };
    device.cmd_copy_buffer(cmd_buf, staging_buffer, buffer, &[copy_region]);
    device.end_command_buffer(cmd_buf)?;

    let submit_info = vk::SubmitInfo::default()
        .command_buffers(std::slice::from_ref(&cmd_buf));

    device.queue_submit(queue, &[submit_info], vk::Fence::null())?;
    device.queue_wait_idle(queue)?;

    device.free_command_buffers(command_pool, &[cmd_buf]);
    allocator.free(staging_alloc)?;
    device.destroy_buffer(staging_buffer, None);

    Ok((buffer, allocation))
}
