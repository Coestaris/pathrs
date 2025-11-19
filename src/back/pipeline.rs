use crate::back::{Back, BackQueues};
use crate::common::command_buffer::CommandBuffer;
use crate::common::shader::Shader;
use anyhow::Context;
use ash::vk::Extent2D;
use ash::{vk, Device};
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};
use log::{debug, warn};
use std::ffi::CStr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const MAX_DEPTH: usize = 2;

pub struct TracerSlot {
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub semaphore: vk::Semaphore,
    pub index: usize,
}

pub struct TracerPipeline {
    queues: BackQueues,
    allocator: Arc<Mutex<Allocator>>,
    destroyed: bool,

    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>, // Size = MAX_DEPTH

    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,

    command_pool: vk::CommandPool,
    command_buffers: Vec<CommandBuffer>, // size = MAX_DEPTH

    images: Vec<vk::Image>,                     // size = MAX_DEPTH
    image_views: Vec<vk::ImageView>,            // size = MAX_DEPTH
    image_samplers: Vec<vk::Sampler>,           // size = MAX_DEPTH
    image_allocations: Vec<Option<Allocation>>, // size = MAX_DEPTH

    fences: Vec<vk::Fence>, // size = MAX_DEPTH
    render_finished_semaphores: Vec<vk::Semaphore>,

    current_frame: usize,
    viewport: glam::UVec2,

    compute_shader: Shader,
}

impl TracerPipeline {
    pub unsafe fn new(
        allocator: Arc<Mutex<Allocator>>,
        viewport: glam::UVec2,
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &Device,
        queues: BackQueues,
    ) -> anyhow::Result<(Self, Vec<TracerSlot>)> {
        let (command_pool, command_buffers) = Self::create_command_buffers(device, &queues)
            .context("Failed to create command buffers")?;

        let (images, image_views, image_samplers, image_allocations) = Self::create_images(
            &mut allocator.lock().unwrap(),
            device,
            &queues,
            command_pool,
            viewport,
        )
        .context("Failed to create images")?;

        let (descriptor_set_layout, descriptor_pool, descriptor_sets) =
            Self::create_descriptor_sets(device, &image_views)
                .context("Failed to create descriptor set layout")?;

        debug!("Creating compute shader");
        let project_root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let assets_dir = project_root.join("assets");
        let compute_shader =
            Shader::new_from_file(device, assets_dir.join("shaders/shader.comp.spv"))
                .context("Failed to create compute shader")?;

        let entrypoint = CStr::from_bytes_with_nul(b"main\0")?;
        let stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(compute_shader.module)
            .name(entrypoint);
        
        debug!("Creating pipeline");
        let (pipeline_layout, pipeline) =
            Self::create_pipeline(device, &descriptor_set_layout, &stage)
                .context("Failed to create pipeline")?;

        debug!("Creating sync objects");
        let (fences, render_finished_semaphores) =
            Self::create_sync_objects(device).context("Failed to create fences")?;

        let slots = (0..MAX_DEPTH)
            .map(|i| TracerSlot {
                image: images[i],
                image_view: image_views[i],
                sampler: image_samplers[i],
                semaphore: render_finished_semaphores[i],
                index: i,
            })
            .collect();

        Ok((
            Self {
                queues,
                allocator,
                destroyed: false,
                descriptor_set_layout,
                descriptor_pool,
                descriptor_sets,
                pipeline_layout,
                pipeline,
                command_pool,
                command_buffers,
                images,
                image_views,
                image_samplers,
                image_allocations: image_allocations.into_iter().map(Some).collect(),
                fences,
                render_finished_semaphores,
                current_frame: 0,
                viewport,
                compute_shader,
            },
            slots,
        ))
    }

    unsafe fn create_sync_objects(
        device: &Device,
    ) -> anyhow::Result<(Vec<vk::Fence>, Vec<vk::Semaphore>)> {
        let mut fences = Vec::with_capacity(MAX_DEPTH);
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        for _ in 0..MAX_DEPTH {
            let fence = device.create_fence(&fence_info, None)?;
            fences.push(fence);
        }

        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let mut semaphores = Vec::with_capacity(MAX_DEPTH);
        for _ in 0..MAX_DEPTH {
            let semaphore = device.create_semaphore(&semaphore_info, None)?;
            semaphores.push(semaphore);
        }

        Ok((fences, semaphores))
    }

    unsafe fn create_images(
        allocator: &mut Allocator,
        device: &Device,
        queues: &BackQueues,
        command_pool: vk::CommandPool,
        viewport: glam::UVec2,
    ) -> anyhow::Result<(
        Vec<vk::Image>,
        Vec<vk::ImageView>,
        Vec<vk::Sampler>,
        Vec<Allocation>,
    )> {
        let mut images = Vec::with_capacity(MAX_DEPTH);
        let mut image_views = Vec::with_capacity(MAX_DEPTH);
        let mut image_samplers = Vec::with_capacity(MAX_DEPTH);
        let mut image_allocations = Vec::with_capacity(MAX_DEPTH);

        for depth in 0..MAX_DEPTH {
            let queue_family_indices = [
                queues.indices.graphics_family,
                queues.indices.compute_family,
            ];
            let create_image_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::R8G8B8A8_UNORM)
                .extent(vk::Extent3D {
                    width: viewport.x,
                    height: viewport.y,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&queue_family_indices)
                .initial_layout(vk::ImageLayout::UNDEFINED);
            let image = device.create_image(&create_image_info, None)?;

            let mem_requirements = device.get_image_memory_requirements(image);
            let allocation = allocator.allocate(&AllocationCreateDesc {
                name: format!("Tracer Pipeline Image Allocation {}", depth).as_str(),
                requirements: mem_requirements,
                location: gpu_allocator::MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })?;

            device.bind_image_memory(image, allocation.memory(), allocation.offset())?;
            images.push(image);
            image_allocations.push(allocation);

            let image_view_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::R8G8B8A8_UNORM)
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1),
                );
            let image_view = device.create_image_view(&image_view_info, None)?;
            image_views.push(image_view);

            // Transition undefined memory layout to the general
            let mut command_buffer = CommandBuffer::new_from_pool(device, command_pool)?;
            command_buffer.begin(device)?;
            let barrier = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::GENERAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1),
                )
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::SHADER_WRITE);
            device.cmd_pipeline_barrier(
                command_buffer.as_inner(),
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
            command_buffer.end(device)?;
            let submit_info = command_buffer.as_submit_info();
            device.queue_submit(queues.compute_queue, &[submit_info], vk::Fence::null())?;
            device.queue_wait_idle(queues.compute_queue)?;
            command_buffer.destroy(command_pool, device);

            let sampler_info = vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .mip_lod_bias(0.0)
                .compare_op(vk::CompareOp::NEVER)
                .min_lod(0.0);
            let sampler = device.create_sampler(&sampler_info, None)?;
            image_samplers.push(sampler);
        }

        Ok((images, image_views, image_samplers, image_allocations))
    }

    unsafe fn create_command_buffers(
        device: &Device,
        queues: &BackQueues,
    ) -> anyhow::Result<(vk::CommandPool, Vec<CommandBuffer>)> {
        let command_pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queues.indices.compute_family);
        let command_pool = device.create_command_pool(&command_pool_info, None)?;

        let command_buffer = (0..MAX_DEPTH)
            .map(|_| CommandBuffer::new_from_pool(device, command_pool))
            .collect::<anyhow::Result<Vec<CommandBuffer>>>()?;

        Ok((command_pool, command_buffer))
    }

    unsafe fn create_descriptor_sets(
        device: &Device,
        image_views: &[vk::ImageView],
    ) -> anyhow::Result<(
        vk::DescriptorSetLayout,
        vk::DescriptorPool,
        Vec<vk::DescriptorSet>,
    )> {
        let bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        let descriptor_set_layout = device.create_descriptor_set_layout(&layout_info, None)?;

        let descriptor_pool_sizes = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(MAX_DEPTH as u32)];
        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&descriptor_pool_sizes)
            .max_sets(MAX_DEPTH as u32);
        let descriptor_pool = device.create_descriptor_pool(&descriptor_pool_info, None)?;

        let layout_handles = vec![descriptor_set_layout; MAX_DEPTH];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layout_handles);
        let descriptor_sets = device.allocate_descriptor_sets(&alloc_info)?;

        for (i, descriptor_set) in descriptor_sets.iter().enumerate() {
            let out_image_info = vk::DescriptorImageInfo::default()
                .image_view(image_views[i])
                .image_layout(vk::ImageLayout::GENERAL);
            let writes = [vk::WriteDescriptorSet::default()
                .dst_set(*descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .image_info(std::slice::from_ref(&out_image_info))];
            device.update_descriptor_sets(&writes, &[]);
        }

        Ok((descriptor_set_layout, descriptor_pool, descriptor_sets))
    }

    unsafe fn create_pipeline(
        device: &Device,
        descriptor_set_layout: &vk::DescriptorSetLayout,
        shader_stage: &vk::PipelineShaderStageCreateInfo,
    ) -> anyhow::Result<(vk::PipelineLayout, vk::Pipeline)> {
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(std::slice::from_ref(descriptor_set_layout));
        let pipline_layout = device.create_pipeline_layout(&pipeline_layout_info, None)?;

        let pipeline_info = vk::ComputePipelineCreateInfo::default()
            .stage(*shader_stage)
            .layout(pipline_layout);
        let pipeline = device
            .create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
            .map_err(|(_, e)| e)?
            .remove(0);
        Ok((pipline_layout, pipeline))
    }

    unsafe fn record_command_buffer(
        &self,
        device: &Device,
        command_buffer: &CommandBuffer,
        descriptor_set: vk::DescriptorSet,
        image: vk::Image,
        extent: Extent2D,
    ) -> anyhow::Result<()> {
        command_buffer.reset(device)?;
        command_buffer.begin(device)?;

        command_buffer.bind_pipeline(device, vk::PipelineBindPoint::COMPUTE, self.pipeline);
        command_buffer.bind_descriptor_sets(
            device,
            vk::PipelineBindPoint::COMPUTE,
            self.pipeline_layout,
            0,
            &[descriptor_set],
            &[],
        );
        command_buffer.dispatch(
            device,
            (extent.width + 15) / 16,
            (extent.height + 15) / 16,
            1,
        );

        let barrier = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::SHADER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::GENERAL)
            .new_layout(vk::ImageLayout::GENERAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );

        device.cmd_pipeline_barrier(
            command_buffer.as_inner(),
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );

        command_buffer.end(device)?;

        Ok(())
    }

    pub unsafe fn present(
        &mut self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &Device,
    ) -> anyhow::Result<TracerSlot> {
        // If the current frame is not ready, return early
        let status = device.get_fence_status(self.fences[self.current_frame])?;
        let (sem, idx) = if status {
            device.reset_fences(&[self.fences[self.current_frame]])?;

            let buffer_ptr: *mut CommandBuffer = &mut self.command_buffers[self.current_frame];
            self.record_command_buffer(
                device,
                &*buffer_ptr,
                self.descriptor_sets[self.current_frame],
                self.images[self.current_frame],
                vk::Extent2D {
                    width: self.viewport.x,
                    height: self.viewport.y,
                },
            )?;

            // Submit
            let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];
            let command_buffers = vec![self.command_buffers[self.current_frame].as_inner()];

            let submit_info = vk::SubmitInfo::default()
                .signal_semaphores(&signal_semaphores)
                .command_buffers(&command_buffers);
            device.queue_submit(
                self.queues.compute_queue,
                &[submit_info],
                self.fences[self.current_frame],
            )?;

            let frame = self.current_frame;
            self.current_frame = (self.current_frame + 1) % MAX_DEPTH;
            (self.render_finished_semaphores[frame], frame)
        } else {
            let idx = (self.current_frame + MAX_DEPTH - 1) % MAX_DEPTH;
            (vk::Semaphore::null(), idx)
        };

        Ok(TracerSlot {
            image: self.images[idx],
            image_view: self.image_views[idx],
            sampler: self.image_samplers[idx],
            semaphore: sem,
            index: idx,
        })
    }

    pub unsafe fn destroy(
        &mut self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &Device,
    ) {
        if !self.destroyed {
            debug!("Waiting for device to be idle before destroying runtime");
            device.device_wait_idle().unwrap();

            debug!("Destroying fences");
            for fence in &self.fences {
                device.destroy_fence(*fence, None);
            }
            for semaphore in &self.render_finished_semaphores {
                device.destroy_semaphore(*semaphore, None);
            }

            debug!("Destroying command pool");
            for cmd_buf in &mut self.command_buffers {
                cmd_buf.destroy(self.command_pool, device);
            }
            device.destroy_command_pool(self.command_pool, None);

            debug!("Destroying pipeline");
            device.destroy_pipeline(self.pipeline, None);

            debug!("Destroying pipeline layout");
            device.destroy_pipeline_layout(self.pipeline_layout, None);

            debug!("Destroying compute shader");
            self.compute_shader.destroy(device);

            debug!("Destroying images");
            for (i, image) in self.images.iter().enumerate() {
                if let Some(allocation) = self.image_allocations[i].take() {
                    self.allocator
                        .lock()
                        .unwrap()
                        .free(allocation)
                        .expect("Failed to free image allocation");
                }
                device.destroy_image_view(self.image_views[i], None);
                device.destroy_sampler(self.image_samplers[i], None);
                device.destroy_image(*image, None);
            }

            debug!("Destroying descriptor set layout");
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            debug!("Destroying descriptor pool");
            device.destroy_descriptor_pool(self.descriptor_pool, None);

            self.destroyed = true;
        } else {
            warn!("TracerPipeline already destroyed");
        }
    }
}

impl Drop for TracerPipeline {
    fn drop(&mut self) {
        if !self.destroyed {
            warn!("Leaked TracerPipeline");
        }
    }
}
