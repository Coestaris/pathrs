use crate::tracer::front::Front;
use crate::tracer::Tracer;
use anyhow::Context;
use ash::vk;
use log::debug;

#[derive(Debug)]
pub struct QueueFamilyIndices {
    pub graphics_family: u32,
    pub compute_family: u32,
}

impl QueueFamilyIndices {
    fn as_vec(&self) -> Vec<QueueFamily> {
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
}

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

#[derive(Debug)]
pub struct QueueFamily {
    pub index: u32,
    pub priorities: Vec<f32>,
}

impl<F: Front> Tracer<F> {
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

    unsafe fn are_required_extensions_supported(
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
    ) -> anyhow::Result<bool> {
        let available_extensions = Self::get_device_extensions(instance, device)?;
        debug!("Available device extensions: {:?}", available_extensions);

        let mut required = vec![];
        required.extend(F::get_required_device_extensions()?);

        // Check if all required extensions are available
        for req in required {
            let req_str = std::ffi::CStr::from_ptr(req).to_string_lossy();
            if !available_extensions.contains(&req_str.into_owned()) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    unsafe fn are_required_layers_supported(
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
    ) -> anyhow::Result<bool> {
        let available_layers = Self::get_device_layers(instance, device)?;
        debug!("Available device layers: {:?}", available_layers);

        let mut required = vec![];
        required.extend(F::get_required_device_layers()?);

        // Check if all required layers are available
        for req in required {
            let req_str = std::ffi::CStr::from_ptr(req).to_string_lossy();
            if !available_layers.contains(&req_str.into_owned()) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    unsafe fn find_queue_families(
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
    ) -> anyhow::Result<QueueFamilyIndices> {
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

        Ok(QueueFamilyIndices {
            graphics_family: graphics_queue_index
                .ok_or_else(|| anyhow::anyhow!("No graphics queue family found"))?,
            compute_family: compute_queue_index
                .ok_or_else(|| anyhow::anyhow!("No compute queue family found"))?,
        })
    }

    unsafe fn is_device_suitable(
        entry: &ash::Entry,
        instance: &ash::Instance,
        front: &F,
        device: vk::PhysicalDevice,
    ) -> bool {
        let extensions_ok =
            Self::are_required_extensions_supported(entry, instance, device).unwrap_or(false);
        let layers_ok =
            Self::are_required_layers_supported(entry, instance, device).unwrap_or(false);
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

    pub unsafe fn new_logical_device(
        entry: &ash::Entry,
        instance: &ash::Instance,
        front: &mut F,
    ) -> anyhow::Result<vk::PhysicalDevice> {
        let physical_devices = instance
            .enumerate_physical_devices()
            .context("Failed to enumerate physical devices")?;

        let physical_device = physical_devices
            .into_iter()
            .find(|&device| Self::is_device_suitable(entry, instance, front, device))
            .ok_or_else(|| anyhow::anyhow!("No suitable device found"))?;

        let common_queues = Self::find_queue_families(instance, physical_device)?;
        debug!("Common queue families: {:?}", common_queues);

        let font_queues = front.find_queue_families(entry, instance, physical_device)?;

        let mut queue_family_infos = vec![];
        queue_family_infos.extend(common_queues.as_vec());
        queue_family_infos.extend(font_queues);
        merge_queues(&mut queue_family_infos);
        debug!("Using queue families: {:?}", queue_family_infos);

        todo!()
    }
}
