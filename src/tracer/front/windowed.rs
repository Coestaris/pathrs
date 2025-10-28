use crate::tracer::front::Front;
use crate::tracer::InstanceCompatibilities;
use ash::vk;
use log::warn;
use std::ffi::c_char;
use winit::raw_window_handle::{DisplayHandle, RawDisplayHandle, RawWindowHandle, WindowHandle};

pub struct TracerWindowedFront {
    surface: vk::SurfaceKHR,
    destroyed: bool,
}

impl TracerWindowedFront {
    pub unsafe fn new_surface(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: WindowHandle,
        display: DisplayHandle,
    ) -> anyhow::Result<vk::SurfaceKHR> {
        #[cfg(target_os = "linux")]
        {
            match (window.as_raw(), display.as_raw()) {
                (RawWindowHandle::Xlib(xlib), RawDisplayHandle::Xlib(xlib_display)) => {
                    let loader = ash::khr::xlib_surface::Instance::new(entry, instance);
                    let create_info = vk::XlibSurfaceCreateInfoKHR::default()
                        .window(xlib.window)
                        .dpy(xlib_display.display.unwrap().as_ptr() as *mut vk::Display);
                    Ok(loader.create_xlib_surface(&create_info, None)?)
                }

                _ => {
                    anyhow::bail!("Unsupported window/display handle combination for Linux");
                }
            }
        }
        #[cfg(target_os = "windows")]
        unimplemented!();
        #[cfg(target_os = "macos")]
        unimplemented!();
    }

    pub unsafe fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: WindowHandle,
        display: DisplayHandle,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            surface: Self::new_surface(entry, instance, window, display)?,
            destroyed: false,
        })
    }
}

pub struct WindowedFrontFamilyIndices {
    pub present_family: u32,
}

impl Front for TracerWindowedFront {
    unsafe fn get_required_instance_extensions(
        _available: &Vec<String>,
        _compatibilities: &mut InstanceCompatibilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![
            vk::KHR_SURFACE_NAME.as_ptr(),
            #[cfg(target_os = "windows")]
            vk::KHR_WIN32_SURFACE_NAME.as_ptr(),
            #[cfg(target_os = "linux")]
            vk::KHR_XLIB_SURFACE_NAME.as_ptr(),
            #[cfg(target_os = "linux")]
            vk::KHR_XCB_SURFACE_NAME.as_ptr(),
            #[cfg(target_os = "linux")]
            vk::KHR_WAYLAND_SURFACE_NAME.as_ptr(),
            #[cfg(target_os = "macos")]
            vk::EXT_METAL_SURFACE_NAME.as_ptr(),
        ])
    }

    unsafe fn get_required_instance_layers(
        _available: &Vec<String>,
        _compatibilities: &mut InstanceCompatibilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![])
    }

    unsafe fn get_required_device_extensions() -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![vk::KHR_SWAPCHAIN_NAME.as_ptr()])
    }

    unsafe fn get_required_device_layers() -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![])
    }

    unsafe fn is_device_suitable(
        &self,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<bool> {
        let queue_family_properties =
            instance.get_physical_device_queue_family_properties(physical_device);
        for (i, queue_family) in queue_family_properties.iter().enumerate() {
            let is_graphics = queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS);
            let present_support = true; // TODO: Check for actual present support using surface loader

            if is_graphics && present_support {
                return Ok(true);
            }
        }

        Ok(false)
    }

    unsafe fn destroy(&mut self, entry: &ash::Entry, instance: &ash::Instance) {
        if !self.destroyed {
            let loader = ash::khr::surface::Instance::new(entry, instance);
            loader.destroy_surface(self.surface, None);
        } else {
            warn!("Front already destroyed");
        }
    }

    unsafe fn resize(&mut self, _size: glam::UVec2) -> anyhow::Result<()> {
        // Resize logic for the front-end
        Ok(())
    }

    unsafe fn present(&mut self) -> anyhow::Result<()> {
        // Presentation logic for the front-end
        Ok(())
    }
}
