use crate::tracer::front::windowed::WindowedQueues;
use crate::tracer::shader::Shader;
use anyhow::Context;
use ash::{vk, Device};
use log::{debug, warn};
use std::path::PathBuf;

pub struct Runtime {
    queues: WindowedQueues,
    destroyed: bool,

    swapchain: vk::SwapchainKHR,
    chain_images: Vec<vk::Image>,
    chain_image_views: Vec<vk::ImageView>,
    chain_image_format: vk::Format,
    chain_extent: vk::Extent2D,

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

        let project_root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let assets_dir = project_root.join("assets");
        let vert_shader =
            Shader::new_from_file(device, assets_dir.join("shaders/triangle.vert.spv"))
                .context("Failed to create vertex shader")?;
        let frag_shader =
            Shader::new_from_file(device, assets_dir.join("shaders/triangle.frag.spv"))
                .context("Failed to create fragment shader")?;

        Ok(Runtime {
            queues,
            swapchain,
            chain_images: images,
            chain_image_views: image_views,
            chain_image_format: format,
            chain_extent: extent,

            vert_shader,
            frag_shader,

            destroyed: false,
        })
    }

    pub unsafe fn destroy(
        &mut self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &Device,
    ) {
        if !self.destroyed {
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
}

impl Drop for Runtime {
    fn drop(&mut self) {
        if !self.destroyed {
            warn!("Leaked runtime");
        }
    }
}
