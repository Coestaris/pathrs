use crate::tracer::front::windowed::WindowedQueues;
use crate::tracer::shader::Shader;
use anyhow::Context;
use ash::{vk, Device};
use log::{debug, warn};
use std::ffi::CStr;
use std::path::PathBuf;
use std::vec;

pub struct Runtime {
    queues: WindowedQueues,
    destroyed: bool,

    swapchain_loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    chain_images: Vec<vk::Image>,
    chain_image_views: Vec<vk::ImageView>,
    chain_image_format: vk::Format,
    chain_extent: vk::Extent2D,

    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,

    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,

    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    swapchain_framebuffers: Vec<vk::Framebuffer>,

    vert_shader: Shader,
    frag_shader: Shader,
}

impl Runtime {
    pub(crate) unsafe fn new(
        viewport: glam::UVec2,
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &Device,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
        queues: WindowedQueues,
    ) -> anyhow::Result<Self> {
        debug!("Creating swapchain");
        let (swapchain, images, format, extent) = Self::create_swapchain(
            viewport,
            entry,
            instance,
            device,
            surface,
            physical_device,
            &queues,
            None,
        )?;

        debug!("Creating image views");
        let image_views = Self::create_image_views(entry, instance, device, &images, format)?;

        debug!("Creating shaders");
        let entrypoint = CStr::from_bytes_with_nul(b"main\0")?;
        let project_root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let assets_dir = project_root.join("assets");
        let vert_shader =
            Shader::new_from_file(device, assets_dir.join("shaders/triangle.vert.spv"))
                .context("Failed to create vertex shader")?;
        let vert_shader_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_shader.module)
            .name(entrypoint);

        let frag_shader =
            Shader::new_from_file(device, assets_dir.join("shaders/triangle.frag.spv"))
                .context("Failed to create fragment shader")?;
        let frag_shader_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_shader.module)
            .name(entrypoint);

        debug!("Creating pipeline layout and render pass");
        let (pipeline_layout, render_pass, pipeline) =
            Self::create_pipeline(device, format, extent, vert_shader_stage, frag_shader_stage)
                .context("Failed to create pipeline")?;

        debug!("Creating framebuffers");
        let swapchain_framebuffers =
            Self::create_framebuffers(&image_views, render_pass, extent, device)
                .context("Failed to create framebuffers")?;

        debug!("Creating command pool and buffers");
        let (command_pool, command_buffer) = Self::create_command_buffer(device, &queues)
            .context("Failed to create command buffer")?;

        debug!("Creating synchronization objects");
        let (image_available_semaphore, render_finished_semaphore, in_flight_fence) =
            Self::create_sync_objects(device, &queues)
                .context("Failed to create synchronization objects")?;

        Ok(Runtime {
            queues,
            swapchain,
            chain_images: images,
            chain_image_views: image_views,
            chain_image_format: format,
            chain_extent: extent,

            pipeline_layout,
            render_pass,
            pipeline,

            image_available_semaphore,
            render_finished_semaphore,
            in_flight_fence,
            command_pool,
            command_buffer,

            swapchain_framebuffers,
            vert_shader,
            frag_shader,

            destroyed: false,
            swapchain_loader: ash::khr::swapchain::Device::new(instance, device),
        })
    }

    pub unsafe fn destroy(
        &mut self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &Device,
    ) {
        if !self.destroyed {
            debug!("Destroying synchronization objects");
            device.destroy_semaphore(self.image_available_semaphore, None);
            device.destroy_semaphore(self.render_finished_semaphore, None);
            device.destroy_fence(self.in_flight_fence, None);

            debug!("Destroying command pool and buffers");
            device.free_command_buffers(self.command_pool, &[self.command_buffer]);
            device.destroy_command_pool(self.command_pool, None);

            debug!("Destroying swapchain framebuffers");
            for framebuffer in &self.swapchain_framebuffers {
                device.destroy_framebuffer(*framebuffer, None);
            }

            debug!("Destroying pipeline");
            device.destroy_pipeline(self.pipeline, None);

            debug!("Destroying render pass");
            device.destroy_render_pass(self.render_pass, None);

            debug!("Destroying pipeline layout");
            device.destroy_pipeline_layout(self.pipeline_layout, None);

            debug!("Destroying shaders");
            self.vert_shader.destroy(device);
            self.frag_shader.destroy(device);

            debug!("Destroying swapchain image views");
            for view in &self.chain_image_views {
                device.destroy_image_view(*view, None);
            }

            debug!("Destroying swapchain");
            let swapchain_loader = ash::khr::swapchain::Device::new(instance, device);
            swapchain_loader.destroy_swapchain(self.swapchain, None);
            self.destroyed = true;
        } else {
            warn!("Runtime already destroyed");
        }
    }

    fn choose_surface_format(formats: &[vk::SurfaceFormatKHR]) -> Option<usize> {
        let mut best_format = None;
        let mut best_score = 0;

        for (i, format) in formats.iter().enumerate() {
            let score = match (format.format, format.color_space) {
                (vk::Format::R8G8B8A8_SRGB, vk::ColorSpaceKHR::SRGB_NONLINEAR) => 10,
                (vk::Format::B8G8R8A8_SRGB, vk::ColorSpaceKHR::SRGB_NONLINEAR) => 8,
                (vk::Format::R8G8B8A8_UNORM, vk::ColorSpaceKHR::SRGB_NONLINEAR) => 6,
                (vk::Format::B8G8R8A8_UNORM, vk::ColorSpaceKHR::SRGB_NONLINEAR) => 4,
                (_, _) => {
                    warn!(
                        "Cannot score: {:?}, {:?}",
                        format.format, format.color_space
                    );
                    0
                }
            };
            if score > best_score {
                best_score = score;
                best_format = Some(i);
            }
        }

        best_format
    }

    fn choose_present_mode(modes: &[vk::PresentModeKHR]) -> Option<usize> {
        // Support only FIFO for now
        for (i, mode) in modes.iter().enumerate() {
            if *mode == vk::PresentModeKHR::FIFO {
                return Some(i);
            }
        }
        None
    }

    fn choose_extent(
        viewport: glam::UVec2,
        capabilities: &vk::SurfaceCapabilitiesKHR,
    ) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::MAX {
            capabilities.current_extent
        } else {
            vk::Extent2D {
                width: viewport.x.clamp(
                    capabilities.min_image_extent.width,
                    capabilities.max_image_extent.width,
                ),
                height: viewport.y.clamp(
                    capabilities.min_image_extent.height,
                    capabilities.max_image_extent.height,
                ),
            }
        }
    }

    unsafe fn create_swapchain(
        viewport: glam::UVec2,
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &Device,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
        queues: &WindowedQueues,
        old_swapchain: Option<vk::SwapchainKHR>,
    ) -> anyhow::Result<(vk::SwapchainKHR, Vec<vk::Image>, vk::Format, vk::Extent2D)> {
        let surface_loader = ash::khr::surface::Instance::new(entry, instance);
        let swapchain_loader = ash::khr::swapchain::Device::new(instance, device);

        // Fetch information about the surface
        let capabilities =
            surface_loader.get_physical_device_surface_capabilities(physical_device, surface)?;
        let formats =
            surface_loader.get_physical_device_surface_formats(physical_device, surface)?;
        let present_modes =
            surface_loader.get_physical_device_surface_present_modes(physical_device, surface)?;

        // Select the best format and present mode
        let format =
            Self::choose_surface_format(&formats).context("No suitable surface format found")?;
        debug!("Chosen surface format: {:?}", formats[format]);
        let present_mode =
            Self::choose_present_mode(&present_modes).context("No suitable present mode found")?;
        debug!("Chosen present mode: {:?}", present_modes[present_mode]);
        let extent = Self::choose_extent(viewport, &capabilities);
        debug!("Chosen swapchain extent: {:?}", extent);

        let images_count = capabilities.min_image_count + 1;
        let images_count =
            if capabilities.max_image_count > 0 && images_count > capabilities.max_image_count {
                capabilities.max_image_count
            } else {
                images_count
            };
        debug!("Chosen swapchain image count: {}", images_count);

        let mut create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(images_count)
            .image_format(formats[format].format)
            .image_color_space(formats[format].color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_modes[present_mode])
            .clipped(true)
            .old_swapchain(old_swapchain.unwrap_or(vk::SwapchainKHR::null()));

        let queue_families = if queues.indices.graphics_family == queues.indices.present_family {
            vec![]
        } else {
            vec![
                queues.indices.graphics_family,
                queues.indices.present_family,
            ]
        };
        if !queue_families.is_empty() {
            create_info = create_info
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&queue_families);
        } else {
            create_info = create_info
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .queue_family_indices(&[]);
        }

        let swapchain = swapchain_loader.create_swapchain(&create_info, None)?;
        Ok((
            swapchain,
            Self::get_swapchain_images(entry, instance, device, swapchain)?,
            formats[format].format,
            extent,
        ))
    }

    unsafe fn get_swapchain_images(
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &Device,
        swapchain: vk::SwapchainKHR,
    ) -> anyhow::Result<Vec<vk::Image>> {
        let swapchain_loader = ash::khr::swapchain::Device::new(instance, device);
        let images = swapchain_loader.get_swapchain_images(swapchain)?;
        Ok(images)
    }

    unsafe fn create_image_views(
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &Device,
        images: &[vk::Image],
        format: vk::Format,
    ) -> anyhow::Result<Vec<vk::ImageView>> {
        let mut views = Vec::with_capacity(images.len());
        for image in images {
            let create_info = vk::ImageViewCreateInfo::default()
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image(*image);

            let view = device
                .create_image_view(&create_info, None)
                .with_context(|| format!("Failed to create image view for image {:?}", image))?;
            views.push(view);
        }

        Ok(views)
    }

    unsafe fn create_pipeline(
        device: &Device,
        format: vk::Format,
        extent: vk::Extent2D,
        vertex_shader_stage: vk::PipelineShaderStageCreateInfo,
        fragment_shader_stage: vk::PipelineShaderStageCreateInfo,
    ) -> anyhow::Result<(vk::PipelineLayout, vk::RenderPass, vk::Pipeline)> {
        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&[])
            .vertex_binding_descriptions(&[]);
        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);
        let viewports = vec![vk::Viewport::default()
            .x(0.0)
            .y(0.0)
            .width(extent.width as f32)
            .height(extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)];
        let scissors = vec![vk::Rect2D::default()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(extent)];
        let viewport_info = vk::PipelineViewportStateCreateInfo::default()
            .viewports(&viewports)
            .scissors(&scissors);
        let rasterization_info = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .line_width(1.0);
        let multisample_info = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .sample_shading_enable(false);
        let color_blend_attachments = vec![vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B
                    | vk::ColorComponentFlags::A,
            )
            .blend_enable(false)];
        let color_blend_info = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(&color_blend_attachments);
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default();
        let pipline_layout = device.create_pipeline_layout(&pipeline_layout_info, None)?;

        let color_attachments = vec![vk::AttachmentDescription::default()
            .format(format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)];
        let color_attachments_refs =
            vec![vk::AttachmentReference::default()
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
        let subpass_dependencies = vec![vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)];
        let subpasses = vec![vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments_refs)];
        let render_pass_info = vk::RenderPassCreateInfo::default()
            .attachments(&color_attachments)
            .dependencies(&subpass_dependencies)
            .subpasses(&subpasses);
        let render_pass = device.create_render_pass(&render_pass_info, None)?;

        let stages = vec![vertex_shader_stage, fragment_shader_stage];
        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_info)
            .color_blend_state(&color_blend_info)
            .dynamic_state(&dynamic_state_info)
            .layout(pipline_layout)
            .render_pass(render_pass)
            .subpass(0);
        let pipeline = device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
            .map_err(|(_, e)| e)?
            .remove(0);

        Ok((pipline_layout, render_pass, pipeline))
    }

    unsafe fn create_framebuffers(
        swapchain_image_views: &[vk::ImageView],
        render_pass: vk::RenderPass,
        extent: vk::Extent2D,
        device: &Device,
    ) -> anyhow::Result<Vec<vk::Framebuffer>> {
        let mut framebuffers = Vec::with_capacity(swapchain_image_views.len());
        for view in swapchain_image_views {
            let attachments = vec![*view];
            let framebuffer_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(extent.width)
                .height(extent.height)
                .layers(1);
            let framebuffer = device.create_framebuffer(&framebuffer_info, None)?;
            framebuffers.push(framebuffer);
        }

        Ok(framebuffers)
    }

    unsafe fn create_command_buffer(
        device: &Device,
        queues: &WindowedQueues,
    ) -> anyhow::Result<(vk::CommandPool, vk::CommandBuffer)> {
        let command_pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queues.indices.graphics_family);
        let command_pool = device.create_command_pool(&command_pool_info, None)?;

        let command_buffer_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let command_buffer = device.allocate_command_buffers(&command_buffer_info)?;
        let command_buffer = command_buffer[0];

        Ok((command_pool, command_buffer))
    }

    unsafe fn create_sync_objects(
        device: &Device,
        queues: &WindowedQueues,
    ) -> anyhow::Result<(vk::Semaphore, vk::Semaphore, vk::Fence)> {
        let image_available_semaphore_info = vk::SemaphoreCreateInfo::default();
        let image_available_semaphore =
            device.create_semaphore(&image_available_semaphore_info, None)?;

        let render_finished_semaphore_info = vk::SemaphoreCreateInfo::default();
        let render_finished_semaphore =
            device.create_semaphore(&render_finished_semaphore_info, None)?;

        let in_flight_fence_info =
            vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let in_flight_fence = device.create_fence(&in_flight_fence_info, None)?;

        Ok((
            image_available_semaphore,
            render_finished_semaphore,
            in_flight_fence,
        ))
    }

    unsafe fn record_command_buffer(
        &self,
        device: &Device,
        image_index: usize,
    ) -> anyhow::Result<()> {
        device.reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty())?;

        let begin_info = vk::CommandBufferBeginInfo::default();
        device.begin_command_buffer(self.command_buffer, &begin_info)?;

        let clear_values = vec![vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.4, 0.0, 1.0],
            },
        }];
        let render_pass_info = vk::RenderPassBeginInfo::default()
            .render_pass(self.render_pass)
            .framebuffer(self.swapchain_framebuffers[image_index])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.chain_extent,
            })
            .clear_values(&clear_values);
        device.cmd_begin_render_pass(
            self.command_buffer,
            &render_pass_info,
            vk::SubpassContents::INLINE,
        );

        let viewport = vk::Viewport::default()
            .x(0.0)
            .y(0.0)
            .width(self.chain_extent.width as f32)
            .height(self.chain_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);
        device.cmd_set_viewport(self.command_buffer, 0, &[viewport]);
        let scissor = vk::Rect2D::default()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(self.chain_extent);
        device.cmd_set_scissor(self.command_buffer, 0, &[scissor]);

        device.cmd_bind_pipeline(
            self.command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline,
        );

        device.cmd_draw(self.command_buffer, 3, 1, 0, 0);

        device.cmd_end_render_pass(self.command_buffer);
        device.end_command_buffer(self.command_buffer)?;

        Ok(())
    }

    pub unsafe fn on_suboptimal() -> anyhow::Result<()> {
        debug!("Swapchain is suboptimal, needs recreation");
        Ok(())
    }

    pub unsafe fn present(&self, device: &Device) -> anyhow::Result<()> {
        // Wait for the fence to be signaled
        device.wait_for_fences(&[self.in_flight_fence], true, u64::MAX)?;
        device.reset_fences(&[self.in_flight_fence])?;

        // Acquire next image
        let index = match self.swapchain_loader.acquire_next_image(
            self.swapchain,
            u64::MAX,
            self.image_available_semaphore,
            vk::Fence::null(),
        ) {
            Ok((index, false)) => index,
            Ok((_, true)) => {
                return Self::on_suboptimal();
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to acquire next swapchain image: {:?}",
                    e
                ));
            }
        };

        self.record_command_buffer(device, index as usize)?;

        // Submit
        let wait_semaphores = vec![self.image_available_semaphore];
        let wait_stages = vec![vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = vec![self.render_finished_semaphore];
        let command_buffers = vec![self.command_buffer];
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);
        device.queue_submit(
            self.queues.graphics_queue,
            &[submit_info],
            self.in_flight_fence,
        )?;

        // Present
        let swapchains = vec![self.swapchain];
        let image_indices = vec![index];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        if self
            .swapchain_loader
            .queue_present(self.queues.present_queue, &present_info)?
        {
            return Self::on_suboptimal();
        }

        Ok(())
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        if !self.destroyed {
            warn!("Leaked runtime");
        }
    }
}
