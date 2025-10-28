use crate::tracer::front::Front;
use crate::tracer::Tracer;
use anyhow::Context;
use ash::vk;
use log::debug;

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

    unsafe fn are_required_queues_supported(
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
    ) -> bool {
        // For simplicity, assume that if the device has any queue families, it is suitable.
        // In a real implementation, you would check for specific queue family capabilities.
        let queue_family_properties = instance.get_physical_device_queue_family_properties(device);
        !queue_family_properties.is_empty()
    }

    unsafe fn is_device_suitable(
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
    ) -> bool {
        let extensions_ok =
            Self::are_required_extensions_supported(entry, instance, device).unwrap_or(false);
        let layers_ok =
            Self::are_required_layers_supported(entry, instance, device).unwrap_or(false);
        let queues_ok = Self::are_required_queues_supported(instance, device);

        extensions_ok && layers_ok && queues_ok
    }

    pub unsafe fn pick_physical_device(
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> anyhow::Result<vk::PhysicalDevice> {
        let physical_devices = instance
            .enumerate_physical_devices()
            .context("Failed to enumerate physical devices")?;

        let device = physical_devices
            .into_iter()
            .find(|&device| Self::is_device_suitable(entry, instance, device))
            .ok_or_else(|| anyhow::anyhow!("No suitable device found"))?;

        Ok(device)
    }
}
