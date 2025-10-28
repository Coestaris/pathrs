use crate::tracer::device::{DeviceCompatibilities, QueueFamily};
use crate::tracer::front::Front;
use crate::tracer::instance::InstanceCompatibilities;
use ash::vk;
use log::{debug, warn};
use std::ffi::c_char;
use winit::raw_window_handle::{
    DisplayHandle, RawDisplayHandle, RawWindowHandle, WindowHandle, XlibDisplayHandle,
    XlibWindowHandle,
};

#[derive(Debug, Clone)]
struct WindowedQueueFamilyIndices {
    pub graphics_family: u32,
    pub present_family: u32,
}

impl WindowedQueueFamilyIndices {
    fn as_vec(&self) -> Vec<QueueFamily> {
        let mut indices = vec![];
        indices.push(QueueFamily {
            index: self.graphics_family,
            priorities: vec![1.0],
        });
        indices.push(QueueFamily {
            index: self.present_family,
            priorities: vec![1.0],
        });
        indices
    }
}

pub enum Mode {
    XLib {
        window: XlibWindowHandle,
        display: XlibDisplayHandle,
    },
}

impl Mode {
    pub fn from_handles(window: WindowHandle, display: DisplayHandle) -> anyhow::Result<Self> {
        match (window.as_raw(), display.as_raw()) {
            (RawWindowHandle::Xlib(xlib_window), RawDisplayHandle::Xlib(xlib_display)) => {
                Ok(Mode::XLib {
                    window: xlib_window,
                    display: xlib_display,
                })
            }
            _ => {
                anyhow::bail!("Unsupported window/display handle combination");
            }
        }
    }

    pub unsafe fn supports_present(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        queue_family_index: u32,
    ) -> bool {
        match self {
            Mode::XLib { window, display } => {
                let loader = ash::khr::xlib_surface::Instance::new(entry, instance);
                loader.get_physical_device_xlib_presentation_support(
                    physical_device,
                    queue_family_index,
                    display.display.unwrap().as_ptr() as *mut vk::Display,
                    window.visual_id as vk::VisualID,
                )
            }
        }
    }

    pub unsafe fn create_surface(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> anyhow::Result<vk::SurfaceKHR> {
        match self {
            Mode::XLib { window, display } => {
                let loader = ash::khr::xlib_surface::Instance::new(entry, instance);
                let create_info = vk::XlibSurfaceCreateInfoKHR::default()
                    .window(window.window as vk::Window)
                    .dpy(display.display.unwrap().as_ptr() as *mut vk::Display);
                Ok(loader.create_xlib_surface(&create_info, None)?)
            }
        }
    }
}

pub struct TracerWindowedFront {
    surface: vk::SurfaceKHR,
    platform: Mode,
    queue_families: Option<WindowedQueueFamilyIndices>,
    destroyed: bool,
}

impl TracerWindowedFront {
    pub unsafe fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: WindowHandle,
        display: DisplayHandle,
    ) -> anyhow::Result<Self> {
        let mode = Mode::from_handles(window, display)?;

        Ok(Self {
            surface: mode.create_surface(entry, instance)?,
            platform: mode,
            queue_families: None,
            destroyed: false,
        })
    }

    unsafe fn queue_families_for_device(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<WindowedQueueFamilyIndices> {
        let mut graphics_family = None;
        let mut present_family = None;

        let queue_family_properties =
            instance.get_physical_device_queue_family_properties(physical_device);

        for (i, queue_family) in queue_family_properties.iter().enumerate() {
            let is_graphics = queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS);
            let present_support =
                self.platform
                    .supports_present(entry, instance, physical_device, i as u32);
            // TODO: Check the presentation mode and formats

            if is_graphics && present_support {
                graphics_family = Some(i as u32);
                present_family = Some(i as u32);
                break;
            }
        }

        Ok(WindowedQueueFamilyIndices {
            graphics_family: graphics_family
                .ok_or_else(|| anyhow::anyhow!("No suitable graphics queue family found"))?,
            present_family: present_family
                .ok_or_else(|| anyhow::anyhow!("No suitable present queue family found"))?,
        })
    }
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

    unsafe fn get_required_device_extensions(
        &self,
        _available: &Vec<String>,
        _compatibilities: &mut DeviceCompatibilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![vk::KHR_SWAPCHAIN_NAME.as_ptr()])
    }

    unsafe fn get_required_device_layers(
        &self,
        _available: &Vec<String>,
        _compatibilities: &mut DeviceCompatibilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![])
    }

    unsafe fn is_device_suitable(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<bool> {
        Ok(self
            .queue_families_for_device(entry, instance, physical_device)
            .is_ok())
    }

    unsafe fn find_queue_families(
        &mut self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<Vec<QueueFamily>> {
        let families = self.queue_families_for_device(entry, instance, physical_device)?;
        debug!("Windowed front queue families: {:?}", families);

        self.queue_families = Some(families.clone());
        Ok(families.as_vec())
    }

    unsafe fn destroy(&mut self, entry: &ash::Entry, instance: &ash::Instance) {
        if !self.destroyed {
            let loader = ash::khr::surface::Instance::new(entry, instance);
            loader.destroy_surface(self.surface, None);
            self.destroyed = true;
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

impl Drop for TracerWindowedFront {
    fn drop(&mut self) {
        if !self.destroyed {
            warn!("Leaked windowed front");
        }
    }
}
