use crate::vk::quad::{QuadBuffer, QuadVertex};
use crate::vk::shader::Shader;
use crate::windowed::front::WindowedQueues;
use crate::windowed::ui::UICompositor;
use anyhow::Context;
use ash::{vk, Device};
use egui::{ClippedPrimitive, FullOutput, TextureId, TexturesDelta};
use gpu_allocator::vulkan::{Allocation, Allocator};
use log::{debug, warn};
use std::cell::RefCell;
use std::ffi::CStr;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::vec;
use winit::window::Window;

const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct Runtime {
    queues: WindowedQueues,
    allocator: Arc<Mutex<Allocator>>,
    destroyed: bool,

    ui_renderer: egui_ash_renderer::Renderer,
    ui: Rc<RefCell<UICompositor>>,
    textures_to_free: Option<Vec<TextureId>>,

    swapchain_loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    chain_images: Vec<vk::Image>,
    chain_image_views: Vec<vk::ImageView>,
    chain_image_format: vk::Format,
    chain_extent: vk::Extent2D,

    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,

    image_available_semaphores: Vec<vk::Semaphore>, // size = MAX_FRAMES_IN_FLIGHT
    render_finished_semaphores: Vec<vk::Semaphore>, // size = chain_images.len()
    in_flight_fences: Vec<vk::Fence>,               // size = MAX_FRAMES_IN_FLIGHT
    images_in_flight: Vec<vk::Fence>,               // size = chain_images.len(),
    current_frame: usize,

    quad: QuadBuffer,

    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    swapchain_framebuffers: Vec<vk::Framebuffer>,

    vert_shader: Shader,
    frag_shader: Shader,
}

impl Runtime {
    pub(crate) unsafe fn new(
        allocator: Arc<Mutex<Allocator>>,
        viewport: glam::UVec2,
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &Device,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
        queues: WindowedQueues,
        ui: Rc<RefCell<UICompositor>>,
    ) -> anyhow::Result<Self> {
        debug!("Creating swapchain");
        let (swapchain, images, format, extent) = Self::create_swapchain(
            Some(viewport),
            entry,
            instance,
            device,
            surface,
            physical_device,
            &queues,
            None,
        )?;

        debug!("Creating image views");
        let image_views = Self::create_image_views(device, &images, format)?;

        debug!("Creating shaders");
        let project_root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let assets_dir = project_root.join("assets");
        let vert_shader =
            Shader::new_from_file(device, assets_dir.join("shaders/triangle.vert.spv"))
                .context("Failed to create vertex shader")?;
        let frag_shader =
            Shader::new_from_file(device, assets_dir.join("shaders/triangle.frag.spv"))
                .context("Failed to create fragment shader")?;

        debug!("Creating pipeline layout and render pass");
        let render_pass =
            Self::create_render_pass(device, format).context("Failed to create render pass")?;

        let entrypoint = CStr::from_bytes_with_nul(b"main\0")?;
        let stages = vec![
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_shader.module)
                .name(entrypoint),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_shader.module)
                .name(entrypoint),
        ];
        let (pipeline_layout, pipeline) =
            Self::create_pipeline(device, extent, render_pass, &stages)
                .context("Failed to create pipeline")?;

        debug!("Creating framebuffers");
        let swapchain_framebuffers =
            Self::create_framebuffers(&image_views, render_pass, extent, device)
                .context("Failed to create framebuffers")?;

        debug!("Creating command pool and buffers");
        let (command_pool, command_buffers) = Self::create_command_buffers(device, &queues)
            .context("Failed to create command buffer")?;

        debug!("Creating quad buffers");
        let quad_buffer = QuadBuffer::new(
            device,
            &mut allocator.lock().unwrap(),
            command_pool,
            queues.graphics_queue,
        )
        .context("Failed to create quad buffers")?;

        debug!("Creating synchronization objects");
        let (
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight,
        ) = Self::create_sync_objects(device, images.len())
            .context("Failed to create synchronization objects")?;

        Ok(Runtime {
            swapchain_loader: ash::khr::swapchain::Device::new(instance, device),

            queues,
            swapchain,
            chain_images: images,
            chain_image_views: image_views,
            chain_image_format: format,
            chain_extent: extent,

            pipeline_layout,
            render_pass,
            pipeline,

            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight,
            current_frame: 0,

            quad: quad_buffer,

            command_pool,
            command_buffers,

            swapchain_framebuffers,
            vert_shader,
            frag_shader,

            destroyed: false,
            ui_renderer: egui_ash_renderer::Renderer::with_gpu_allocator(
                allocator.clone(),
                device.clone(),
                render_pass,
                egui_ash_renderer::Options {
                    in_flight_frames: MAX_FRAMES_IN_FLIGHT,
                    ..Default::default()
                },
            )?,
            ui,
            textures_to_free: None,
            allocator,
        })
    }

    pub unsafe fn swapchain_cleanup(&mut self, device: &Device) {
        device.device_wait_idle().unwrap();

        for fb in &self.swapchain_framebuffers {
            device.destroy_framebuffer(*fb, None);
        }
        self.swapchain_framebuffers.clear();

        for view in &self.chain_image_views {
            device.destroy_image_view(*view, None);
        }
        self.chain_image_views.clear();

        for s in &self.render_finished_semaphores {
            device.destroy_semaphore(*s, None);
        }
        self.render_finished_semaphores.clear();
        self.images_in_flight.clear();
    }

    pub unsafe fn destroy(&mut self, instance: &ash::Instance, device: &Device) {
        if !self.destroyed {
            // Wait for all in-flight frames to finish
            debug!("Waiting for device to be idle before destroying runtime");
            device.device_wait_idle().unwrap();

            debug!("Destroying synchronization objects");
            for semaphore in &self.image_available_semaphores {
                device.destroy_semaphore(*semaphore, None);
            }
            for semaphore in &self.render_finished_semaphores {
                device.destroy_semaphore(*semaphore, None);
            }
            for fence in &self.in_flight_fences {
                device.destroy_fence(*fence, None);
            }

            debug!("Destroying command pool and buffers");
            device.free_command_buffers(self.command_pool, &self.command_buffers);
            device.destroy_command_pool(self.command_pool, None);

            debug!("Destroying buffers");
            self.quad
                .destroy(device, &mut self.allocator.lock().unwrap());

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
        let mut best_mode = None;
        let mut best_score = 0;

        // Support only FIFO for now
        for (i, mode) in modes.iter().enumerate() {
            let score = match *mode {
                vk::PresentModeKHR::IMMEDIATE => 10,
                vk::PresentModeKHR::MAILBOX => 8,
                vk::PresentModeKHR::FIFO => 6,
                vk::PresentModeKHR::FIFO_RELAXED => 5,
                _ => 0,
            };

            if score > best_score {
                best_score = score;
                best_mode = Some(i);
            }
        }

        best_mode
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
        viewport: Option<glam::UVec2>,
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
        println!("Surface capabilities: {:?}", capabilities);
        let formats =
            surface_loader.get_physical_device_surface_formats(physical_device, surface)?;
        let present_modes =
            surface_loader.get_physical_device_surface_present_modes(physical_device, surface)?;

        let viewport = viewport.unwrap_or(glam::UVec2::new(
            capabilities.current_extent.width,
            capabilities.current_extent.height,
        ));

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
            Self::get_swapchain_images(instance, device, swapchain)?,
            formats[format].format,
            extent,
        ))
    }

    unsafe fn get_swapchain_images(
        instance: &ash::Instance,
        device: &Device,
        swapchain: vk::SwapchainKHR,
    ) -> anyhow::Result<Vec<vk::Image>> {
        let swapchain_loader = ash::khr::swapchain::Device::new(instance, device);
        let images = swapchain_loader.get_swapchain_images(swapchain)?;
        Ok(images)
    }

    unsafe fn create_image_views(
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

    unsafe fn create_render_pass(
        device: &Device,
        format: vk::Format,
    ) -> anyhow::Result<vk::RenderPass> {
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

        Ok(device.create_render_pass(&render_pass_info, None)?)
    }

    unsafe fn create_pipeline(
        device: &Device,
        extent: vk::Extent2D,
        render_pass: vk::RenderPass,
        shader_stages: &[vk::PipelineShaderStageCreateInfo],
    ) -> anyhow::Result<(vk::PipelineLayout, vk::Pipeline)> {
        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);
        let vertex_binding_descriptors = vec![QuadVertex::get_binding_description()];
        let vertex_attribute_descriptors = QuadVertex::get_attribute_descriptions().to_vec();
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&vertex_attribute_descriptors)
            .vertex_binding_descriptions(&vertex_binding_descriptors);
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
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_DST_ALPHA)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE)
            .alpha_blend_op(vk::BlendOp::ADD)];
        let color_blend_info = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&color_blend_attachments);
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default();
        let pipline_layout = device.create_pipeline_layout(&pipeline_layout_info, None)?;

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
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

        Ok((pipline_layout, pipeline))
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

    unsafe fn create_command_buffers(
        device: &Device,
        queues: &WindowedQueues,
    ) -> anyhow::Result<(vk::CommandPool, Vec<vk::CommandBuffer>)> {
        let command_pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queues.indices.graphics_family);
        let command_pool = device.create_command_pool(&command_pool_info, None)?;

        let command_buffer_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(MAX_FRAMES_IN_FLIGHT as u32);
        let command_buffer = device.allocate_command_buffers(&command_buffer_info)?;
        Ok((command_pool, command_buffer))
    }

    unsafe fn create_sync_objects(
        device: &Device,
        chain_images_len: usize,
    ) -> anyhow::Result<(
        Vec<vk::Semaphore>, // image_available_semaphores
        Vec<vk::Semaphore>, // render_finished_semaphores (per-image)
        Vec<vk::Fence>,     // in_flight_fences
        Vec<vk::Fence>,     // images_in_flight
    )> {
        let sem_info = vk::SemaphoreCreateInfo::default();
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        // Per frame
        let mut image_available = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut in_flight = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            image_available.push(device.create_semaphore(&sem_info, None)?);
            in_flight.push(device.create_fence(&fence_info, None)?);
        }

        // Per image
        let mut render_finished = Vec::with_capacity(chain_images_len);
        for _ in 0..chain_images_len {
            render_finished.push(device.create_semaphore(&sem_info, None)?);
        }
        let images_in_flight = vec![vk::Fence::null(); chain_images_len];

        Ok((
            image_available,
            render_finished,
            in_flight,
            images_in_flight,
        ))
    }

    unsafe fn record_egui_buffer(
        &mut self,
        w: &Window,
        command_buffer: vk::CommandBuffer,
        device: &Device,
        image_index: usize,
    ) -> anyhow::Result<()> {
        // Free last frames textures after the previous frame is done rendering
        if let Some(textures) = self.textures_to_free.take() {
            self.ui_renderer
                .free_textures(&textures)
                .expect("Failed to free textures");
        }

        let ui = self.ui.as_ptr();

        let raw_input = (*ui).egui.take_egui_input(&w);
        let FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            ..
        } = (*ui).egui.egui_ctx().run(raw_input, |ctx| {
            (*ui).render(ctx, self.allocator.lock().unwrap().deref_mut());
        });

        if !textures_delta.free.is_empty() {
            self.textures_to_free = Some(textures_delta.free.clone());
        }

        if !textures_delta.set.is_empty() {
            self.ui_renderer
                .set_textures(
                    self.queues.graphics_queue,
                    self.command_pool,
                    textures_delta.set.as_slice(),
                )
                .expect("Failed to update texture");
        }

        (*ui).egui.handle_platform_output(&w, platform_output);
        let clipped_meshes = (*ui).egui.egui_ctx().tessellate(shapes, pixels_per_point);

        let extent = vk::Extent2D {
            width: self.chain_extent.width,
            height: self.chain_extent.height,
        };
        Ok(self
            .ui_renderer
            .cmd_draw(command_buffer, extent, pixels_per_point, &clipped_meshes)?)
    }

    unsafe fn record_command_buffer(
        &self,
        command_buffer: vk::CommandBuffer,
        device: &Device,
        image_index: usize,
    ) -> anyhow::Result<()> {
        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline,
        );

        self.quad.draw(device, command_buffer);

        Ok(())
    }

    pub(crate) unsafe fn resize(
        &mut self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &Device,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
        viewport: glam::UVec2,
    ) -> anyhow::Result<()> {
        // if self.chain_extent.width != viewport.x || self.chain_extent.height != viewport.y {
        //     debug!(
        //         "Resizing swapchain from {:?} to {:?}",
        //         self.chain_extent, viewport
        //     );
        //
        //     return self.on_suboptimal(entry, instance, device, surface, physical_device, None);
        // }

        Ok(())
    }

    unsafe fn render(
        &mut self,
        w: &Window,
        command_buffer: vk::CommandBuffer,
        device: &Device,
        image_index: usize,
    ) -> anyhow::Result<()> {
        device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;

        let begin_info = vk::CommandBufferBeginInfo::default();
        device.begin_command_buffer(command_buffer, &begin_info)?;

        let clear_values = vec![vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
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
            command_buffer,
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
        device.cmd_set_viewport(command_buffer, 0, &[viewport]);
        let scissor = vk::Rect2D::default()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(self.chain_extent);
        device.cmd_set_scissor(command_buffer, 0, &[scissor]);

        self.record_command_buffer(command_buffer, device, image_index)?;
        self.record_egui_buffer(&w, command_buffer, device, image_index)?;

        device.cmd_end_render_pass(command_buffer);
        device.end_command_buffer(command_buffer)?;

        Ok(())
    }

    pub unsafe fn on_suboptimal(
        &mut self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &Device,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
        viewport: Option<glam::UVec2>,
    ) -> anyhow::Result<()> {
        debug!("Swapchain is suboptimal, needs recreation");

        // Cleanup old swapchain
        self.swapchain_cleanup(device);

        // Create new swapchain
        let old_swapchain = self.swapchain;
        let (swapchain, images, format, extent) = Self::create_swapchain(
            viewport,
            entry,
            instance,
            device,
            surface,
            physical_device,
            &self.queues,
            Some(old_swapchain),
        )?;

        let format_changed = format != self.chain_image_format;
        self.swapchain = swapchain;
        self.chain_images = images;
        self.chain_image_format = format;
        self.chain_extent = extent;

        // Destroy old swapchain
        self.swapchain_loader.destroy_swapchain(old_swapchain, None);

        // Create new swapchain image views
        self.chain_image_views =
            Self::create_image_views(device, &self.chain_images, self.chain_image_format)?;

        // If format changed, recreate pipeline
        if format_changed {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_render_pass(self.render_pass, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);

            let render_pass =
                Self::create_render_pass(device, format).context("Failed to create render pass")?;

            let entrypoint = CStr::from_bytes_with_nul(b"main\0")?;
            let stages = vec![
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(self.vert_shader.module)
                    .name(entrypoint),
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(self.frag_shader.module)
                    .name(entrypoint),
            ];
            let (pipeline_layout, pipeline) =
                Self::create_pipeline(device, extent, render_pass, &stages)
                    .context("Failed to create pipeline")?;

            self.pipeline_layout = pipeline_layout;
            self.render_pass = render_pass;
            self.pipeline = pipeline;
        }

        // New framebuffers
        self.swapchain_framebuffers = Self::create_framebuffers(
            &self.chain_image_views,
            self.render_pass,
            self.chain_extent,
            device,
        )?;

        // Recreate per-image semaphores
        let sem_info = vk::SemaphoreCreateInfo::default();
        self.render_finished_semaphores = (0..self.chain_images.len())
            .map(|_| device.create_semaphore(&sem_info, None))
            .collect::<Result<_, _>>()?;
        self.images_in_flight = vec![vk::Fence::null(); self.chain_images.len()];
        self.current_frame = 0;

        Ok(())
    }

    pub unsafe fn present(
        &mut self,
        w: &Window,
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &Device,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<()> {
        // Wait for the fence to be signaled
        device.wait_for_fences(&[self.in_flight_fences[self.current_frame]], true, u64::MAX)?;

        // Acquire next image
        let index = match self.swapchain_loader.acquire_next_image(
            self.swapchain,
            u64::MAX,
            self.image_available_semaphores[self.current_frame],
            vk::Fence::null(),
        ) {
            Ok((index, false)) => index as usize,
            Ok((_, true)) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                return self.on_suboptimal(entry, instance, device, surface, physical_device, None);
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to acquire next swapchain image: {:?}",
                    e
                ));
            }
        };

        // Wait for the image to be available
        if self.images_in_flight[index] != vk::Fence::null() {
            if self.images_in_flight[index] != self.in_flight_fences[self.current_frame] {
                device.wait_for_fences(&[self.images_in_flight[index]], true, u64::MAX)?;
            }
        }
        self.images_in_flight[index] = self.in_flight_fences[self.current_frame];

        // Record command buffer
        self.render(w, self.command_buffers[self.current_frame], device, index)?;
        device.reset_fences(&[self.in_flight_fences[self.current_frame]])?;

        // Submit
        let wait_semaphores = [self.image_available_semaphores[self.current_frame]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished_semaphores[index]];
        let command_buffers = vec![self.command_buffers[self.current_frame]];
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);
        device.queue_submit(
            self.queues.graphics_queue,
            &[submit_info],
            self.in_flight_fences[self.current_frame],
        )?;

        // Present
        let swapchains = vec![self.swapchain];
        let image_indices = [index as u32];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        match self
            .swapchain_loader
            .queue_present(self.queues.present_queue, &present_info)
        {
            Ok(false) => {}
            Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                return self.on_suboptimal(entry, instance, device, surface, physical_device, None);
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to present swapchain image: {:?}",
                    e
                ));
            }
        };

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
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
