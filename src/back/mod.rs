pub mod pipeline;

use crate::common::compatibilities::{DeviceCompatibilities, InstanceCompatibilities};
use crate::common::queue::QueueFamily;
use crate::front::QueueFamilyIndices;
use ash::{vk, Device};
use std::ffi::c_char;

impl QueueFamilyIndices for BackQueueFamilyIndices {
    type Queues = BackQueues;

    fn as_families(&self) -> Vec<QueueFamily> {
        let mut indices = vec![];
        indices.push(QueueFamily {
            index: self.graphics_family,
            priorities: vec![1.0],
        });
        indices.push(QueueFamily {
            index: self.compute_family,
            priorities: vec![1.0],
        });

        indices
    }

    unsafe fn into_queues(self, device: &Device) -> anyhow::Result<BackQueues> {
        let graphics_queue = device.get_device_queue(self.graphics_family, 0);
        let compute_queue = device.get_device_queue(self.compute_family, 0);

        Ok(BackQueues {
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
pub struct BackQueues {
    pub graphics_queue: vk::Queue,
    pub compute_queue: vk::Queue,
}

pub struct Back {}

impl Back {
    pub unsafe fn get_required_instance_extensions(
        _available: &Vec<String>,
        _compatibilities: &mut InstanceCompatibilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![])
    }

    pub unsafe fn get_required_instance_layers(
        _available: &Vec<String>,
        _compatibilities: &mut InstanceCompatibilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![])
    }

    pub unsafe fn get_required_device_extensions(
        _available: &Vec<String>,
        _compatibilities: &mut DeviceCompatibilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![ash::ext::buffer_device_address::NAME.as_ptr()])
    }

    pub unsafe fn is_device_suitable(
        _entry: &ash::Entry,
        _instance: &ash::Instance,
        _physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<bool> {
        Ok(true)
    }

    pub unsafe fn patch_create_device_info(
        _entry: &ash::Entry,
        _instance: &ash::Instance,
        _physical_device: vk::PhysicalDevice,
        create_info: vk::DeviceCreateInfo,
        on_patched: &mut impl FnMut(vk::DeviceCreateInfo) -> anyhow::Result<ash::Device>,
    ) -> anyhow::Result<ash::Device> {
        let mut device_address_info =
            vk::PhysicalDeviceBufferDeviceAddressFeatures::default().buffer_device_address(true);
        let create_info = create_info.push_next(&mut device_address_info);
        on_patched(create_info)
    }

    pub unsafe fn find_queue_families(
        _entry: &ash::Entry,
        instance: &ash::Instance,
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

                if queue_family.queue_flags.contains(vk::QueueFlags::COMPUTE) {
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
}
