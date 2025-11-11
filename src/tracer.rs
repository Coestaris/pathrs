use crate::config::TracerConfig;
use crate::front::Front;
use crate::vk::debug_messenger::DebugMessenger;
use crate::vk::device::LogicalDevice;
use anyhow::Context;
use ash::vk::PhysicalDevice;
use ash::{Entry, Instance};
use build_info::BuildInfo;
use egui::Window;
use glam::UVec2;
use log::{debug, info, warn};

pub struct Tracer<F: Front> {
    viewport: UVec2,
    pub front: F,

    pub config: TracerConfig,

    pub entry: Entry,
    pub instance: Instance,
    pub debug_messenger: Option<DebugMessenger>,
    pub physical_device: PhysicalDevice,
    pub logical_device: LogicalDevice,
}

impl<F: Front> Tracer<F> {
    unsafe fn new_entry() -> anyhow::Result<Entry> {
        Ok(Entry::load()?)
    }

    pub(crate) unsafe fn new<D: Front>(
        config: TracerConfig,
        viewport: UVec2,
        bi: BuildInfo,
        constructor: impl FnOnce(&Entry, &Instance) -> anyhow::Result<D>,
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
        Ok(())
    }

    pub unsafe fn trace(&mut self, w: Option<&winit::window::Window>) -> anyhow::Result<()> {
        self.trace_inner()?;

        self.front
            .present(
                w,
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
            .resize(
                &self.entry,
                &self.instance,
                &self.logical_device.device,
                self.physical_device,
                size,
            )
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
