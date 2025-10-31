use crate::tracer::front::{Front, QueueFamilyIndices};
use anyhow::Context;
use ash::vk::{DeviceQueueCreateInfo, PhysicalDevice, PhysicalDeviceFeatures};
use ash::{vk, Device, Instance};
use log::{debug, warn};

pub struct DeviceCompatibilities {}

impl Default for DeviceCompatibilities {
    fn default() -> Self {
        Self {}
    }
}

#[derive(Debug)]
pub struct CommonQueueFamilyIndices {
    pub graphics_family: u32,
    pub compute_family: u32,
}

#[derive(Debug)]
pub struct CommonQueues {
    pub indices: CommonQueueFamilyIndices,
    pub graphics_queue: vk::Queue,
    pub compute_queue: vk::Queue,
}

impl QueueFamilyIndices for CommonQueueFamilyIndices {
    type Queues = CommonQueues;

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

    unsafe fn into_queues(self, device: &Device) -> anyhow::Result<CommonQueues> {
        let graphics_queue = device.get_device_queue(self.graphics_family, 0);
        let compute_queue = device.get_device_queue(self.compute_family, 0);

        Ok(CommonQueues {
            indices: self,
            graphics_queue,
            compute_queue,
        })
    }
}

#[derive(Debug)]
pub struct QueueFamily {
    pub index: u32,
    pub priorities: Vec<f32>,
}

impl QueueFamily {
    fn merge_queues(a: &mut Vec<QueueFamily>) {
        // If the queue families are the same, merge them
        let mut i = 0;
        while i < a.len() {
            let mut j = i + 1;
            while j < a.len() {
                if a[i].index == a[j].index {
                    // TODO: Merge priorities
                    a.remove(j);
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }
}

pub struct LogicalDevice {
    compatibilities: DeviceCompatibilities,
    pub(crate) device: Device,
    queues: CommonQueues,
    destroyed: bool,
}

unsafe fn is_subset(
    available: &Vec<String>,
    required: &Vec<*const std::ffi::c_char>,
) -> anyhow::Result<bool> {
    for req in required {
        let req_str = std::ffi::CStr::from_ptr(*req).to_string_lossy();
        if !available.contains(&req_str.into_owned()) {
            return Ok(false);
        }
    }

    Ok(true)
}

impl LogicalDevice {
    unsafe fn get_device_extensions(
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
    ) -> anyhow::Result<Vec<String>> {
        let extension_properties = instance
            .enumerate_device_extension_properties(device)
            .context("Failed to enumerate device extension properties")?;
        let extensions = extension_properties
            .iter()
            .map(|ext| {
                let ext_name = unsafe { std::ffi::CStr::from_ptr(ext.extension_name.as_ptr()) };
                ext_name.to_string_lossy().into_owned()
            })
            .collect();
        Ok(extensions)
    }

    unsafe fn get_device_layers(
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
    ) -> anyhow::Result<Vec<String>> {
        let layer_properties = instance
            .enumerate_device_layer_properties(device)
            .context("Failed to enumerate device layer properties")?;
        let layers = layer_properties
            .iter()
            .map(|layer| {
                let layer_name = unsafe { std::ffi::CStr::from_ptr(layer.layer_name.as_ptr()) };
                layer_name.to_string_lossy().into_owned()
            })
            .collect();
        Ok(layers)
    }

    unsafe fn get_required_device_extensions<F: Front>(
        _entry: &ash::Entry,
        _instance: &ash::Instance,
        available: &Vec<String>,
        front: &F,
        compatibilities: &mut DeviceCompatibilities,
    ) -> anyhow::Result<Vec<*const std::ffi::c_char>> {
        let mut required = vec![];
        required.extend(front.get_required_device_extensions(available, compatibilities)?);
        Ok(required)
    }

    unsafe fn get_required_device_layers<F: Front>(
        _entry: &ash::Entry,
        _instance: &ash::Instance,
        available: &Vec<String>,
        front: &F,
        compatibilities: &mut DeviceCompatibilities,
    ) -> anyhow::Result<Vec<*const std::ffi::c_char>> {
        let mut required = vec![];
        required.extend(front.get_required_device_layers(available, compatibilities)?);
        Ok(required)
    }

    unsafe fn find_queue_families(
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
    ) -> anyhow::Result<CommonQueueFamilyIndices> {
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

        Ok(CommonQueueFamilyIndices {
            graphics_family: graphics_queue_index
                .ok_or_else(|| anyhow::anyhow!("No graphics queue family found"))?,
            compute_family: compute_queue_index
                .ok_or_else(|| anyhow::anyhow!("No compute queue family found"))?,
        })
    }

    unsafe fn is_device_suitable<F: Front>(
        entry: &ash::Entry,
        instance: &ash::Instance,
        front: &F,
        compatibilities: &mut DeviceCompatibilities,
        device: vk::PhysicalDevice,
    ) -> bool {
        let extensions = Self::get_device_extensions(instance, device).unwrap_or(vec![]);
        let required_extensions = Self::get_required_device_extensions(
            entry,
            instance,
            &extensions,
            front,
            compatibilities,
        )
        .unwrap_or(vec![]);
        let extensions_ok = is_subset(&extensions, &required_extensions).unwrap_or(false);

        let layers = Self::get_device_layers(instance, device).unwrap_or(vec![]);
        let required_layers =
            Self::get_required_device_layers(entry, instance, &layers, front, compatibilities)
                .unwrap_or(vec![]);
        let layers_ok = is_subset(&layers, &required_layers).unwrap_or(false);

        let front_ok = front
            .is_device_suitable(entry, instance, device)
            .unwrap_or(false);
        let queues_ok = Self::find_queue_families(instance, device).is_ok();

        let properties = instance.get_physical_device_properties(device);
        debug!("Device: {:?}", properties);
        debug!(
            "extensions_ok: {}, layers_ok: {}, front_ok: {}, queues_ok: {}",
            extensions_ok, layers_ok, front_ok, queues_ok
        );

        extensions_ok && layers_ok && front_ok && queues_ok
    }

    unsafe fn find_suitable_device<F: Front>(
        entry: &ash::Entry,
        instance: &ash::Instance,
        front: &F,
    ) -> anyhow::Result<vk::PhysicalDevice> {
        let devices = instance
            .enumerate_physical_devices()
            .context("Failed to enumerate physical devices")?;

        for device in devices {
            let mut compatibilities = DeviceCompatibilities::default();

            // TODO: Implement some kind of scoring system for compatibility
            if Self::is_device_suitable(entry, instance, front, &mut compatibilities, device) {
                return Ok(device);
            }
        }

        Err(anyhow::anyhow!("No suitable physical device found"))
    }

    pub unsafe fn destroy(&mut self) {
        if !self.destroyed {
            self.device.destroy_device(None);
            self.destroyed = true;
        } else {
            warn!("Logical device already destroyed");
        }
    }

    pub unsafe fn new<F: Front>(
        entry: &ash::Entry,
        instance: &Instance,
        front: &mut F,
    ) -> anyhow::Result<(PhysicalDevice, Self)> {
        let physical_device = Self::find_suitable_device(entry, instance, front)?;

        let mut compatibilities = DeviceCompatibilities::default();
        let extensions = Self::get_device_extensions(instance, physical_device)?;
        let extensions = Self::get_required_device_extensions(
            entry,
            instance,
            &extensions,
            front,
            &mut compatibilities,
        )?;
        let layers = Self::get_device_layers(instance, physical_device)?;
        let layers = Self::get_required_device_layers(
            entry,
            instance,
            &layers,
            front,
            &mut compatibilities,
        )?;

        let common_queues = Self::find_queue_families(instance, physical_device)?;
        debug!("Using common queue families: {:?}", common_queues);
        let font_queues = front.find_queue_families(entry, instance, physical_device)?;
        debug!("Using front queue families: {:?}", font_queues);

        let mut queue_family_infos = vec![];
        queue_family_infos.extend(common_queues.as_families());
        queue_family_infos.extend(font_queues.as_families());
        QueueFamily::merge_queues(&mut queue_family_infos);
        debug!("Using queue families: {:?}", queue_family_infos);

        let queue_create_infos = queue_family_infos
            .iter()
            .map(|qfi| {
                DeviceQueueCreateInfo::default()
                    .queue_family_index(qfi.index)
                    .queue_priorities(&qfi.priorities)
                    .flags(vk::DeviceQueueCreateFlags::empty())
            })
            .collect::<Vec<_>>();
        let features = PhysicalDeviceFeatures::default();

        let device_create_info = vk::DeviceCreateInfo::default()
            .enabled_extension_names(&extensions)
            .enabled_layer_names(&layers)
            .queue_create_infos(&queue_create_infos)
            .enabled_features(&features)
            .flags(vk::DeviceCreateFlags::empty());
        let logical_device = instance
            .create_device(physical_device, &device_create_info, None)
            .context("Failed to create logical device")?;

        let common_queues = common_queues.into_queues(&logical_device)?;
        debug!("Acquired common queues: {:?}", common_queues);
        let font_queues = font_queues.into_queues(&logical_device)?;
        debug!("Acquired front queues: {:?}", font_queues);

        front.set_device(
            entry,
            instance,
            &logical_device,
            physical_device,
            font_queues,
        )?;

        Ok((
            physical_device,
            Self {
                compatibilities,
                device: logical_device,
                queues: common_queues,
                destroyed: false,
            },
        ))
    }
}

impl Drop for LogicalDevice {
    fn drop(&mut self) {
        if !self.destroyed {
            warn!("Leaked logical device");
        }
    }
}
