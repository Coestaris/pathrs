use std::sync::{Arc, Mutex};
use ash::Device;
use gpu_allocator::vulkan::Allocator;
use log::{debug, warn};
use crate::back::BackQueues;

pub struct TracerPipeline {
    queues: BackQueues,
    allocator: Arc<Mutex<Allocator>>,
    destroyed: bool,
}

impl TracerPipeline {
    pub unsafe fn new(
        allocator: Arc<Mutex<Allocator>>,
        queues: BackQueues,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            queues,
            allocator,
            destroyed: false,
        })
    }

    pub unsafe fn destroy(
        &mut self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &Device,
    ) {
        if !self.destroyed {
            // Wait for all in-flight frames to finish
            debug!("Waiting for device to be idle before destroying runtime");
            device.device_wait_idle().unwrap();
        } else {
            warn!("TracerPipeline already destroyed");
        }
    }
}

impl Drop for TracerPipeline {
    fn drop(&mut self) {
        if !self.destroyed {
            warn!("Leaked TracerPipeline");
        }
    }
}
