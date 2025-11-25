use crate::back::TracerSlot;
use crate::common::queue::QueueFamily;
use crate::front::headless::TracerHeadlessOutput;
use crate::front::{Front, QueueFamilyIndices};
use ash::vk::PhysicalDevice;
use ash::{Device, Entry, Instance};
use log::info;

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

    unsafe fn find_queue_families(
        &self,
        _entry: &Entry,
        _instance: &Instance,
        _physical_device: PhysicalDevice,
    ) -> anyhow::Result<HeadlessQueueFamilyIndices> {
        Ok(HeadlessQueueFamilyIndices {})
    }

    unsafe fn present(
        &mut self,
        _w: Option<&winit::window::Window>,
        _entry: &Entry,
        _instance: &Instance,
        _device: &Device,
        _physical_device: PhysicalDevice,
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
