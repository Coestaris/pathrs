use anyhow::Context;
use ash::{vk, Entry};
use log::warn;
use std::ffi::{c_char, CStr};
use crate::vk::instance::InstanceCompatibilities;

unsafe extern "system" fn debug_callback(
    _message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    let message = CStr::from_ptr((*p_callback_data).p_message);
    eprintln!("VULKAN DEBUG: {:?}", message);
    vk::FALSE
}

pub struct DebugMessenger {
    handle: vk::DebugUtilsMessengerEXT,
    destroyed: bool,
}

impl DebugMessenger {
    pub(super) unsafe fn get_required_instance_extensions(
        available: &Vec<String>,
        compatibilities: &mut InstanceCompatibilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        let mut required = vec![];
        if available.contains(&"VK_EXT_debug_utils".to_string()) {
            const VK_EXT_DEBUG_UTILS: &CStr =
                unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_EXT_debug_utils\0") };
            required.push(VK_EXT_DEBUG_UTILS.as_ptr());
            compatibilities.debug_utils_ext = true;
        }

        Ok(required)
    }

    pub(super) unsafe fn get_required_instance_layers(
        available: &Vec<String>,
        compatibilities: &mut InstanceCompatibilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        let mut required = vec![];
        if available.contains(&"VK_LAYER_KHRONOS_validation".to_string()) {
            const VK_LAYER_KHRONOS_VALIDATION: &CStr =
                unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0") };
            required.push(VK_LAYER_KHRONOS_VALIDATION.as_ptr());
            compatibilities.validation_layer = true;
        }

        Ok(required)
    }

    pub unsafe fn available(compatibilities: &InstanceCompatibilities) -> bool {
        compatibilities.debug_utils_ext && compatibilities.validation_layer
    }

    pub unsafe fn destroy(&mut self, entry: &Entry, instance: &ash::Instance) {
        if !self.destroyed {
            let debug_utils_loader = ash::ext::debug_utils::Instance::new(entry, instance);
            debug_utils_loader.destroy_debug_utils_messenger(self.handle, None);
            self.destroyed = true;
        } else {
            warn!("Debug messanger already destroyed");
        }
    }

    pub unsafe fn new(entry: &Entry, instance: &ash::Instance) -> anyhow::Result<Self> {
        let debug_utils_loader = ash::ext::debug_utils::Instance::new(&entry, &instance);
        let create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(debug_callback));

        let debug_messenger = debug_utils_loader
            .create_debug_utils_messenger(&create_info, None)
            .context("Failed to create debug utils messenger")?;

        Ok(Self {
            handle: debug_messenger,
            destroyed: false,
        })
    }
}

impl Drop for DebugMessenger {
    fn drop(&mut self) {
        if !self.destroyed {
            warn!("Leaked debug messanger");
        }
    }
}
