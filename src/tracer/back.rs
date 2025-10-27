use crate::tracer::front::TracerFront;
use anyhow::Context;
use ash::vk::DebugUtilsMessengerEXT;
use ash::{Entry, Instance};
use build_info::BuildInfo;
use glam::UVec2;
use log::{debug, warn};

pub struct InstanceCompatibilities {
    pub debug_utils_ext: bool,
    pub validation_layer: bool,
}

impl Default for InstanceCompatibilities {
    fn default() -> Self {
        Self {
            debug_utils_ext: false,
            validation_layer: false,
        }
    }
}

pub struct TracerBack {
    viewport: UVec2,
    front: TracerFront,

    pub entry: Entry,
    pub instance: Instance,
    pub debug_messenger: Option<DebugUtilsMessengerEXT>,
}

impl TracerBack {
    unsafe fn new_entry() -> anyhow::Result<ash::Entry> {
        Ok(Entry::load()?)
    }

    pub unsafe fn new(viewport: UVec2, bi: BuildInfo, front: TracerFront) -> anyhow::Result<Self> {
        debug!("Creating Vulkan instance");
        let entry = Self::new_entry()?;

        debug!("Created Vulkan entry");
        let (instance, instance_compatibilities) = Self::new_instance(&entry, &front, bi)?;

        debug!("Setting up debug messanger");
        let debug_messenger = if Self::supports_debug_messanger(&instance_compatibilities) {
            Some(Self::new_debug_messanger(&entry, &instance)?)
        } else {
            warn!("Debug messanger not supported on this system");
            None
        };

        Ok(Self {
            viewport,
            front,
            entry,
            instance,
            debug_messenger,
        })
    }

    pub fn trace(&mut self) -> anyhow::Result<()> {
        // Tracing logic for the back-end

        self.front
            .present()
            .context("Failed to present tracer front")?;

        Ok(())
    }

    pub fn resize(&mut self, size: UVec2) -> anyhow::Result<()> {
        self.viewport = size;

        self.front
            .resize(size)
            .with_context(|| format!("Failed to resize tracer front to {:?}", size))?;

        Ok(())
    }
}

impl Drop for TracerBack {
    fn drop(&mut self) {
        unsafe {
            debug!("Destroying debug messanger");
            self.destroy_debug_messanger();

            debug!("Destroying Vulkan instance");
            self.instance.destroy_instance(None);
        }
    }
}
