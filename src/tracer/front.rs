use ash::vk;
use std::ffi::c_char;
use winit::window::Window;

#[derive(Debug)]
pub struct TracerSurface {}

impl TracerSurface {
    pub fn new_from_winit(_: &Window) -> anyhow::Result<Self> {
        Ok(Self {})
    }
}

#[derive(Debug)]
pub struct WindowedTracerFront {
    surface: TracerSurface,
}

#[derive(Debug)]
pub(super) enum TracerFront {
    Headless,
    Windowed(WindowedTracerFront),
}

impl TracerFront {
    pub fn new_headless() -> Self {
        Self::Headless
    }

    pub(super) fn new_windowed(surface: TracerSurface) -> Self {
        Self::Windowed(WindowedTracerFront { surface })
    }

    pub(super) fn resize(&mut self, _size: glam::UVec2) -> anyhow::Result<()> {
        // Resize logic for the front-end
        Ok(())
    }

    pub(super) fn present(&mut self) -> anyhow::Result<()> {
        // Presentation logic for the front-end
        Ok(())
    }

    pub(super) unsafe fn get_required_instance_extensions(
        &self,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(match self {
            TracerFront::Windowed(_) => vec![
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
            ],
            _ => vec![],
        })
    }

    pub(super) unsafe fn get_required_instance_layers(&self) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![])
    }
}
