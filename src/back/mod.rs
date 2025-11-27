pub mod pipeline;
mod push_constants;
mod ssbo;

use crate::assets::AssetManager;
use crate::back::pipeline::TracerPipeline;
use crate::back::push_constants::PushConstantsData;
use crate::back::ssbo::ParametersSSBOData;
use crate::common::capabilities::{DeviceCapabilities, InstanceCapabilities};
use crate::common::queue::QueueFamily;
use crate::config::TracerConfig;
use crate::front::QueueFamilyIndices;
use crate::tracer::{Bundle, TracerProfile};
use ash::{vk, Device, Entry, Instance};
use std::ffi::c_char;

#[allow(dead_code)]
pub struct TracerSlot {
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub descriptor_set: vk::DescriptorSet,
    pub index: usize,
}

impl QueueFamilyIndices for BackQueueFamilyIndices {
    type Queues = BackQueues;

    fn as_families(&self) -> Vec<QueueFamily> {
        vec![
            QueueFamily {
                index: self.graphics_family,
                priorities: vec![1.0],
            },
            QueueFamily {
                index: self.compute_family,
                priorities: vec![1.0],
            },
        ]
    }

    unsafe fn into_queues(self, device: &Device) -> anyhow::Result<BackQueues> {
        let graphics_queue = device.get_device_queue(self.graphics_family, 0);
        let compute_queue = device.get_device_queue(self.compute_family, 0);

        Ok(BackQueues {
            indices: self,
            graphics_queue,
            compute_queue,
        })
    }
}

#[derive(Debug)]
pub struct BackQueueFamilyIndices {
    pub graphics_family: u32,
    pub compute_family: u32,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct BackQueues {
    pub indices: BackQueueFamilyIndices,
    pub graphics_queue: vk::Queue,
    pub compute_queue: vk::Queue,
}

pub struct Back {
    pipeline: TracerPipeline,

    config: TracerConfig,
    time: f32,
}

impl Back {
    pub unsafe fn get_required_instance_extensions(
        _available: &Vec<String>,
        _capabilities: &mut InstanceCapabilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![])
    }

    pub unsafe fn get_required_instance_layers(
        _available: &Vec<String>,
        _capabilities: &mut InstanceCapabilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![])
    }

    pub unsafe fn get_required_device_extensions(
        _available: &Vec<String>,
        _capabilities: &mut DeviceCapabilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![ash::ext::buffer_device_address::NAME.as_ptr()])
    }

    pub unsafe fn is_device_suitable(
        _entry: &Entry,
        _instance: &Instance,
        _physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<bool> {
        Ok(true)
    }

    pub unsafe fn patch_create_device_info(
        _entry: &Entry,
        _instance: &Instance,
        _physical_device: vk::PhysicalDevice,
        create_info: vk::DeviceCreateInfo,
        on_patched: &mut impl FnMut(vk::DeviceCreateInfo) -> anyhow::Result<Device>,
    ) -> anyhow::Result<Device> {
        let mut device_address_info =
            vk::PhysicalDeviceBufferDeviceAddressFeatures::default().buffer_device_address(true);
        let mut host_query_reset_info =
            vk::PhysicalDeviceHostQueryResetFeatures::default().host_query_reset(true);
        let create_info = create_info
            .push_next(&mut device_address_info)
            .push_next(&mut host_query_reset_info);
        on_patched(create_info)
    }

    pub unsafe fn find_queue_families(
        _entry: &Entry,
        instance: &Instance,
        device: vk::PhysicalDevice,
    ) -> anyhow::Result<BackQueueFamilyIndices> {
        let mut graphics_queue_index = None;
        let mut compute_queue_index = None;

        let queue_family_properties = instance.get_physical_device_queue_family_properties(device);
        for (i, queue_family) in queue_family_properties.iter().enumerate() {
            if queue_family.queue_count > 0 {
                if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    graphics_queue_index = Some(i as u32);
                }

                if queue_family.queue_flags.contains(vk::QueueFlags::COMPUTE)
                    && queue_family.timestamp_valid_bits > 0
                {
                    compute_queue_index = Some(i as u32);
                }
            }
        }

        Ok(BackQueueFamilyIndices {
            graphics_family: graphics_queue_index
                .ok_or_else(|| anyhow::anyhow!("No graphics queue family found"))?,
            compute_family: compute_queue_index
                .ok_or_else(|| anyhow::anyhow!("No compute queue family found"))?,
        })
    }

    pub unsafe fn new(
        bundle: Bundle,
        asset_manager: AssetManager,
        viewport: glam::UVec2,
        queues: BackQueues,
        config: TracerConfig,
    ) -> anyhow::Result<Self> {
        let pipeline = TracerPipeline::new(bundle, asset_manager, viewport, queues)?;

        Ok(Self {
            pipeline,
            config,
            time: 0.0,
        })
    }

    pub unsafe fn present(&mut self, bundle: Bundle) -> anyhow::Result<TracerSlot> {
        self.time += 1.0 / 60.0; // For debugging purposes, assume its always 60 fps

        let push_constants = PushConstantsData::new(self.time);
        let config = self.config.0.borrow();
        let parameters = ParametersSSBOData::new(config.slider);

        self.pipeline.present(bundle, parameters, push_constants)
    }

    pub unsafe fn destroy(&mut self, bundle: Bundle) {
        self.pipeline.destroy(bundle);
    }

    pub unsafe fn resize(&mut self, bundle: Bundle, size: glam::UVec2) -> anyhow::Result<()> {
        self.pipeline.resize(bundle, size)
    }

    pub fn get_profile(&self) -> TracerProfile {
        self.pipeline.get_profile()
    }
}
