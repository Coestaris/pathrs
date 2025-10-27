use crate::tracer::back::{InstanceCompatibilities, TracerBack};
use anyhow::Context;
use ash::{vk, Entry};
use std::ffi::CStr;

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

impl TracerBack {
    pub unsafe fn supports_debug_messanger(compatibilities: &InstanceCompatibilities) -> bool {
        compatibilities.debug_utils_ext && compatibilities.validation_layer
    }

    pub unsafe fn destroy_debug_messanger(&self) {
        if let Some(debug_messenger) = self.debug_messenger {
            let debug_utils_loader =
                ash::ext::debug_utils::Instance::new(&self.entry, &self.instance);
            debug_utils_loader.destroy_debug_utils_messenger(debug_messenger, None);
        }
    }

    pub unsafe fn new_debug_messanger(
        entry: &Entry,
        instance: &ash::Instance,
    ) -> anyhow::Result<vk::DebugUtilsMessengerEXT> {
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

        Ok(debug_messenger)
    }
}
