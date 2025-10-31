pub mod config;
pub mod debug_messanger;
pub mod device;
pub mod front;
pub mod instance;
mod shader;

use crate::tracer::config::TracerConfig;
use crate::tracer::debug_messanger::DebugMessenger;
use crate::tracer::device::LogicalDevice;
use crate::tracer::front::headless::{TracerHeadlessFront, TracerHeadlessOutput};
use crate::tracer::front::windowed::TracerWindowedFront;
use crate::tracer::front::Front;
use anyhow::Context;
use ash::{vk, Entry, Instance};
use build_info::BuildInfo;
use glam::UVec2;
use log::{debug, info, warn};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

pub struct Tracer<F: Front> {
    viewport: UVec2,
    pub front: F,

    pub config: TracerConfig,

    pub entry: Entry,
    pub instance: Instance,
    pub debug_messenger: Option<DebugMessenger>,
    pub physical_device: vk::PhysicalDevice,
    pub logical_device: LogicalDevice,
}

impl<F: Front> Tracer<F> {
    unsafe fn new_entry() -> anyhow::Result<ash::Entry> {
        Ok(Entry::load()?)
    }

    pub unsafe fn new_windowed(
        config: TracerConfig,
        viewport: UVec2,
        bi: BuildInfo,
        window: &Window,
    ) -> anyhow::Result<Tracer<TracerWindowedFront>> {
        Self::new(config, viewport, bi, |entry, instance| {
            TracerWindowedFront::new(
                entry,
                instance,
                viewport,
                window.window_handle()?,
                window.display_handle()?,
            )
        })
    }

    pub unsafe fn new_headless<C>(
        config: TracerConfig,
        viewport: UVec2,
        bi: BuildInfo,
        callback: C,
    ) -> anyhow::Result<Tracer<TracerHeadlessFront>>
    where
        C: FnMut(TracerHeadlessOutput) + Send + 'static,
    {
        Self::new(config, viewport, bi, |_, _| {
            Ok(TracerHeadlessFront::new(callback))
        })
    }

    pub unsafe fn new<D: Front>(
        config: TracerConfig,
        viewport: UVec2,
        bi: BuildInfo,
        constructor: impl FnOnce(&ash::Entry, &ash::Instance) -> anyhow::Result<D>,
    ) -> anyhow::Result<Tracer<D>> {
        info!("Creating Vulkan instance");
        let entry = Self::new_entry()?;

        info!("Created Vulkan entry");
        let (instance, instance_compatibilities) = Self::new_instance(&entry, bi)?;

        info!("Created Front");
        let mut front =
            constructor(&entry, &instance).context("Failed to create tracer front-end")?;

        info!("Setting up debug messanger");
        let debug_messenger = if DebugMessenger::available(&instance_compatibilities) {
            Some(
                DebugMessenger::new(&entry, &instance)
                    .context("Failed to create debug messanger")?,
            )
        } else {
            warn!("Debug messanger not supported on this system");
            None
        };

        info!("Creating logical device");
        let (physical_device, logical_device) = LogicalDevice::new(&entry, &instance, &mut front)?;

        Ok(Tracer {
            viewport,
            front,
            config,
            entry,
            instance,
            debug_messenger,
            physical_device,
            logical_device,
        })
    }

    unsafe fn trace_inner(&mut self) -> anyhow::Result<()> {
        info!("Tracing frame");
        Ok(())
    }

    pub unsafe fn trace(&mut self) -> anyhow::Result<()> {
        self.trace_inner()?;

        self.front
            .present(
                &self.entry,
                &self.instance,
                &self.logical_device.device,
                self.physical_device,
            )
            .context("Failed to present tracer front")?;

        Ok(())
    }

    pub unsafe fn resize(&mut self, size: UVec2) -> anyhow::Result<()> {
        self.viewport = size;

        self.front
            .resize(size)
            .with_context(|| format!("Failed to resize tracer front to {:?}", size))?;

        Ok(())
    }
}

impl<F: Front> Drop for Tracer<F> {
    fn drop(&mut self) {
        unsafe {
            debug!("Destroying front-end");
            self.front
                .destroy(&self.entry, &self.instance, &self.logical_device.device);

            debug!("Destroying logical device");
            self.logical_device.destroy();

            debug!("Destroying debug messanger");
            if let Some(mut debug_messenger) = self.debug_messenger.take() {
                debug_messenger.destroy(&self.entry, &self.instance);
            }

            debug!("Destroying Vulkan instance");
            self.instance.destroy_instance(None);
        }
    }
}
