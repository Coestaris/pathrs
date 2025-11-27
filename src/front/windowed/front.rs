use crate::assets::AssetManager;
use crate::back::TracerSlot;
use crate::common::capabilities::{DeviceCapabilities, InstanceCapabilities};
use crate::common::queue::QueueFamily;
use crate::front::windowed::pipeline::PresentationPipeline;
use crate::front::windowed::ui::UICompositor;
use crate::front::{Front, QueueFamilyIndices};
use crate::tracer::Bundle;
use anyhow::Context;
use ash::{vk, Device};
use gpu_allocator::vulkan::Allocator;
use log::{debug, warn};
use std::cell::RefCell;
use std::ffi::{c_char, c_void};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use winit::raw_window_handle::{
    DisplayHandle, RawDisplayHandle, RawWindowHandle, WindowHandle, XlibDisplayHandle,
    XlibWindowHandle,
};

#[derive(Debug, Clone)]
pub struct WindowedQueueFamilyIndices {
    pub graphics_family: u32,
    pub present_family: u32,
}

#[derive(Clone, Debug)]
pub struct WindowedQueues {
    pub indices: WindowedQueueFamilyIndices,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
}

impl QueueFamilyIndices for WindowedQueueFamilyIndices {
    type Queues = WindowedQueues;

    fn as_families(&self) -> Vec<QueueFamily> {
        vec![
            QueueFamily {
                index: self.graphics_family,
                priorities: vec![1.0],
            },
            QueueFamily {
                index: self.present_family,
                priorities: vec![1.0],
            },
        ]
    }

    unsafe fn into_queues(self, device: &Device) -> anyhow::Result<Self::Queues> {
        let graphics_queue = device.get_device_queue(self.graphics_family, 0);
        let presentation_queue = device.get_device_queue(self.present_family, 0);

        Ok(WindowedQueues {
            indices: self,
            graphics_queue,
            present_queue: presentation_queue,
        })
    }
}

pub enum Mode {
    XLib {
        window: XlibWindowHandle,
        display: XlibDisplayHandle,
    },
    Wayland {
        window: *mut c_void,
        display: *mut c_void,
    },
    Windows {
        hwnd: *mut c_void,
        hinstance: *mut c_void,
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
            (
                RawWindowHandle::Wayland(wayland_window),
                RawDisplayHandle::Wayland(wayland_display),
            ) => Ok(Mode::Wayland {
                window: wayland_window.surface.as_ptr(),
                display: wayland_display.display.as_ptr(),
            }),
            (RawWindowHandle::Win32(windows_window), _) => Ok(Mode::Windows {
                hwnd: windows_window.hwnd.get() as *mut c_void,
                hinstance: windows_window.hinstance.unwrap().get() as *mut c_void,
            }),
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
            Mode::Wayland { window: _, display } => {
                let loader = ash::khr::wayland_surface::Instance::new(entry, instance);
                loader.get_physical_device_wayland_presentation_support(
                    physical_device,
                    queue_family_index,
                    (*display as *mut vk::wl_display).as_mut().unwrap(),
                )
            }
            Mode::Windows {
                hwnd: _,
                hinstance: _,
            } => {
                let loader = ash::khr::win32_surface::Instance::new(entry, instance);
                loader.get_physical_device_win32_presentation_support(
                    physical_device,
                    queue_family_index,
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
            Mode::Wayland { window, display } => {
                let loader = ash::khr::wayland_surface::Instance::new(entry, instance);
                let create_info = vk::WaylandSurfaceCreateInfoKHR::default()
                    .surface((*window as *mut vk::wl_surface).as_mut().unwrap())
                    .display((*display as *mut vk::wl_display).as_mut().unwrap());
                Ok(loader.create_wayland_surface(&create_info, None)?)
            }
            Mode::Windows { hwnd, hinstance } => {
                let loader = ash::khr::win32_surface::Instance::new(entry, instance);
                let create_info = vk::Win32SurfaceCreateInfoKHR::default()
                    .hwnd(*hwnd as isize)
                    .hinstance(*hinstance as isize);
                Ok(loader.create_win32_surface(&create_info, None)?)
            }
        }
    }
}

pub struct TracerWindowedFront {
    asset_manager: AssetManager,
    surface: vk::SurfaceKHR,
    viewport: glam::UVec2,
    platform: Mode,
    runtime: Option<PresentationPipeline>,
    destroyed: bool,
    ui: Rc<RefCell<UICompositor>>,
}

impl TracerWindowedFront {
    pub unsafe fn new(
        asset_manager: AssetManager,
        entry: &ash::Entry,
        instance: &ash::Instance,
        viewport: glam::UVec2,
        window: WindowHandle,
        display: DisplayHandle,
        ui: Rc<RefCell<UICompositor>>,
    ) -> anyhow::Result<Self> {
        let mode = Mode::from_handles(window, display)?;

        Ok(Self {
            asset_manager,
            surface: mode.create_surface(entry, instance)?,
            viewport,
            platform: mode,
            runtime: None,
            destroyed: false,
            ui,
        })
    }

    unsafe fn is_swapchain_format_supported(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<bool> {
        let surface_loader = ash::khr::surface::Instance::new(entry, instance);
        let formats =
            surface_loader.get_physical_device_surface_formats(physical_device, self.surface)?;
        let modes = surface_loader
            .get_physical_device_surface_present_modes(physical_device, self.surface)?;

        debug!(
            "Supported swapchain formats: {:?}, modes: {:?}",
            formats, modes
        );

        Ok(!formats.is_empty() && !modes.is_empty())
    }
}

impl Front for TracerWindowedFront {
    type FrontQueueFamilyIndices = WindowedQueueFamilyIndices;

    unsafe fn get_required_instance_extensions(
        _available: &Vec<String>,
        _capabilities: &mut InstanceCapabilities,
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
        _capabilities: &mut InstanceCapabilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![])
    }

    unsafe fn get_required_device_extensions(
        &self,
        _available: &Vec<String>,
        _capabilities: &mut DeviceCapabilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![vk::KHR_SWAPCHAIN_NAME.as_ptr()])
    }

    unsafe fn is_device_suitable(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<bool> {
        let queues_ok = self
            .find_queue_families(entry, instance, physical_device)
            .is_ok();
        let swapchain_format_ok = self
            .is_swapchain_format_supported(entry, instance, physical_device)
            .is_ok();

        debug!(
            "queues_ok: {}, swapchain_format_ok: {}",
            queues_ok, swapchain_format_ok
        );
        Ok(queues_ok && swapchain_format_ok)
    }

    unsafe fn find_queue_families(
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

    unsafe fn init(
        &mut self,
        bundle: Bundle,
        queues: WindowedQueues,
    ) -> anyhow::Result<()> {
        self.runtime = Some(
            PresentationPipeline::new(
                bundle,
                self.asset_manager.clone(),
                self.viewport,
                self.surface,
                queues,
                self.ui.clone(),
            )
            .context("Failed to create windowed runtime")?,
        );
        Ok(())
    }

    unsafe fn destroy(&mut self, bundle: Bundle) {
        if !self.destroyed {
            if let Some(mut runtime) = self.runtime.take() {
                debug!("Destroying windowed runtime");
                runtime.destroy(bundle);
            }

            debug!("Destroying windowed surface");
            let surface = ash::khr::surface::Instance::new(bundle.entry, bundle.instance);
            surface.destroy_surface(self.surface, None);
            self.destroyed = true;
        } else {
            warn!("Front already destroyed");
        }
    }

    unsafe fn resize(&mut self, bundle: Bundle, size: glam::UVec2) -> anyhow::Result<()> {
        if let Some(runtime) = &mut self.runtime {
            runtime
                .resize(bundle, self.surface, size)
                .context("Failed to resize windowed runtime")
        } else {
            Ok(())
        }
    }

    unsafe fn present(
        &mut self,
        bundle: Bundle,
        w: Option<&winit::window::Window>,
        tracer_slot: TracerSlot,
    ) -> anyhow::Result<()> {
        if let Some(runtime) = &mut self.runtime {
            runtime
                .present(bundle, w.unwrap(), self.surface, tracer_slot)
                .context("Failed to present windowed runtime")
        } else {
            Ok(())
        }
    }
}

impl Drop for TracerWindowedFront {
    fn drop(&mut self) {
        if !self.destroyed {
            warn!("Leaked windowed front");
        }
    }
}
