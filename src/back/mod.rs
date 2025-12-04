pub mod pipeline;
mod push_constants;
mod ssbo;

use crate::assets::AssetManager;
use crate::back::pipeline::TracerPipeline;
use crate::back::push_constants::PushConstantsData;
use crate::back::ssbo::config::SSBOConfigData;
use crate::back::ssbo::objects::{SSBOObjectData, SSBOObjectsData, MAX_OBJECTS};
use crate::common::capabilities::{DeviceCapabilities, InstanceCapabilities};
use crate::common::queue::QueueFamily;
use crate::config::{TracerConfig, TracerConfigInner};
use crate::front::QueueFamilyIndices;
use crate::tracer::{Bundle, TracerProfile};
use ash::{vk, Device, Entry, Instance};
use std::ffi::c_char;

#[allow(dead_code)]
pub struct TracerSlotImage {
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,

    pub dimensions: glam::UVec2,
    pub byte_size: usize,
    pub layout: vk::ImageLayout,
    pub format: vk::Format,
}

#[allow(dead_code)]
pub struct TracerSlot {
    pub image: TracerSlotImage,
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
    frame_index: u64,
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
        images_custom_usage: vk::ImageUsageFlags,
    ) -> anyhow::Result<Self> {
        let pipeline =
            TracerPipeline::new(bundle, asset_manager, viewport, queues, images_custom_usage)?;

        Ok(Self {
            pipeline,
            config,
            frame_index: 0,
        })
    }

    pub unsafe fn present(&mut self, bundle: Bundle) -> anyhow::Result<TracerSlot> {
        let mut config = self.config.0.borrow_mut();

        let time = self.frame_index as f32 / 60.0;
        let invalidate = config.updated;
        let push_constants = PushConstantsData::new(time);

        // For now do not support changing objects in runtime
        let objects_data = if self.frame_index == 0 {
            Some(config.as_objects())
        } else {
            None
        };

        let config_data = if config.updated {
            config.updated = false;
            Some(config.as_config())
        } else {
            None
        };

        self.frame_index += 1;

        self.pipeline.present(
            bundle,
            config_data,
            objects_data,
            push_constants,
            invalidate,
        )
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

impl TracerConfigInner {
    fn as_objects(&self) -> SSBOObjectsData {
        let mut objects = [SSBOObjectData::default(); MAX_OBJECTS];
        for (i, object) in self.objects.iter().enumerate() {
            if i >= MAX_OBJECTS {
                break;
            }
            match object {
                crate::config::Object::Sphere {
                    center,
                    radius,
                    material,
                } => {
                    objects[i] = SSBOObjectData::new_sphere(*center, *radius, material);
                }
            }
        }

        objects
    }

    fn as_config(&self) -> SSBOConfigData {
        SSBOConfigData {
            camera_transform: self.camera.as_transform().to_cols_array_2d(),
            camera_fov: self.camera.fov,
            objects_count: self.objects.len() as u32,
            samples_count: self.samples_count,
            jitter_strength: self.jitter_strength,
            temporal_accumulation: self.temporal_accumulation,
        }
    }
}
