use crate::back::TracerSlot;
use crate::common::capabilities::DeviceCapabilities;
use crate::common::queue::QueueFamily;
use crate::front::headless::TracerHeadlessOutput;
use crate::front::{Front, QueueFamilyIndices};
use crate::tracer::Bundle;
use ash::{vk, Device, Entry, Instance};
use log::info;
use std::ffi::{c_char, CStr};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HeadlessQueueFamilyIndices {}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HeadlessQueues {}

impl QueueFamilyIndices for HeadlessQueueFamilyIndices {
    type Queues = HeadlessQueues;

    fn as_families(&self) -> Vec<QueueFamily> {
        vec![]
    }

    unsafe fn into_queues(self, _device: &Device) -> anyhow::Result<Self::Queues> {
        Ok(HeadlessQueues {})
    }
}

#[allow(dead_code)]
pub struct TracerHeadlessFront {
    callback: Box<dyn FnMut(TracerHeadlessOutput) + Send>,
}

impl TracerHeadlessFront {
    pub(crate) fn new<F>(callback: F) -> Self
    where
        F: FnMut(TracerHeadlessOutput) + Send + 'static,
    {
        Self {
            callback: Box::new(callback),
        }
    }
}

impl Front for TracerHeadlessFront {
    type FrontQueueFamilyIndices = HeadlessQueueFamilyIndices;

    unsafe fn get_required_device_extensions(
        &self,
        available: &Vec<String>,
        capabilities: &mut DeviceCapabilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        const VK_EXT_HOST_IMAGE_COPY_NAME: &CStr = c"VK_EXT_host_image_copy";
        let mut required = vec![];
        if available.contains(&VK_EXT_HOST_IMAGE_COPY_NAME.to_str()?.to_string()) {
            capabilities.host_image_copy = true;
            info!("Image copy extension required");
            required.push(VK_EXT_HOST_IMAGE_COPY_NAME.as_ptr() as *const c_char);
        }

        Ok(required)
    }

    unsafe fn find_queue_families(
        &self,
        _entry: &Entry,
        _instance: &Instance,
        _physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<HeadlessQueueFamilyIndices> {
        Ok(HeadlessQueueFamilyIndices {})
    }

    unsafe fn present(
        &mut self,
        _bundle: Bundle,
        _w: Option<&winit::window::Window>,
        _slot: TracerSlot,
    ) -> anyhow::Result<()> {
        info!("Presenting frame");

        // TODO: Transfer framebuffer from the GPU to the CPU
        (self.callback)(TracerHeadlessOutput {
            width: 800,
            height: 600,
            rgb888: vec![0; 800 * 600 * 3],
        });

        Ok(())
    }
}
