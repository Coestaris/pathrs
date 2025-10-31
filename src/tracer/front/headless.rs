use crate::tracer::device::QueueFamily;
use crate::tracer::front::{Front, QueueFamilyIndices};
use ash::vk::PhysicalDevice;
use ash::{Device, Entry, Instance};

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
pub struct TracerHeadlessFront {}

impl TracerHeadlessFront {
    pub(crate) fn new() -> Self {
        todo!()
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
}
