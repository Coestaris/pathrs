use crate::tracer::back::{InstanceCompatibilities, TracerBack};
use crate::tracer::front::TracerFront;
use anyhow::Context;
use ash::{vk, Entry};
use build_info::BuildInfo;
use log::{debug, warn};
use std::ffi::{c_char, CStr, CString};


impl TracerBack {
    unsafe fn get_instance_extensions(entry: &Entry) -> anyhow::Result<Vec<String>> {
        let extension_properties = entry
            .enumerate_instance_extension_properties(None)
            .context("Failed to enumerate instance extension properties")?;
        let extensions = extension_properties
            .iter()
            .map(|ext| {
                let ext_name = CStr::from_ptr(ext.extension_name.as_ptr());
                ext_name.to_string_lossy().into_owned()
            })
            .collect();
        Ok(extensions)
    }

    unsafe fn get_instance_layers(entry: &Entry) -> anyhow::Result<Vec<String>> {
        let layer_properties = entry
            .enumerate_instance_layer_properties()
            .context("Failed to enumerate instance layer properties")?;
        let layers = layer_properties
            .iter()
            .map(|layer| {
                let layer_name = CStr::from_ptr(layer.layer_name.as_ptr());
                layer_name.to_string_lossy().into_owned()
            })
            .collect();
        Ok(layers)
    }

    unsafe fn get_required_instance_extensions(
        entry: &Entry,
        front: &TracerFront,
        compatibilities: &mut InstanceCompatibilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        let extensions = Self::get_instance_extensions(entry)?;
        debug!("Available instance extensions: {:?}", extensions);

        let mut required = vec![];
        if extensions.contains(&"VK_EXT_debug_utils".to_string()) {
            const VK_EXT_DEBUG_UTILS: &CStr =
                unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_EXT_debug_utils\0") };
            required.push(VK_EXT_DEBUG_UTILS.as_ptr());
            compatibilities.debug_utils_ext = true;
        }

        required.extend(front.get_required_instance_extensions()?);
        Ok(required)
    }

    unsafe fn get_required_instance_layers(
        entry: &Entry,
        front: &TracerFront,
        compatibilities: &mut InstanceCompatibilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        let layers = Self::get_instance_layers(entry)?;
        debug!("Available instance layers: {:?}", layers);

        let mut required = vec![];
        if layers.contains(&"VK_LAYER_KHRONOS_validation".to_string()) {
            const VK_LAYER_KHRONOS_VALIDATION: &CStr =
                unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0") };
            required.push(VK_LAYER_KHRONOS_VALIDATION.as_ptr());
            compatibilities.validation_layer = true;
        }

        required.extend(front.get_required_instance_layers()?);
        Ok(required)
    }

    pub unsafe fn new_instance(
        entry: &Entry,
        front: &TracerFront,
        bi: BuildInfo,
    ) -> anyhow::Result<(ash::Instance, InstanceCompatibilities)> {
        let app_name = CString::new(bi.crate_info.name)?;
        let app_version = bi.crate_info.version;
        let app_version = vk::make_api_version(
            0,
            app_version.major as u32,
            app_version.minor as u32,
            app_version.patch as u32,
        );
        let engine_name = CString::new("NoEngine")?;
        let engine_version = vk::make_api_version(0, 1, 0, 0);
        let api_version = vk::make_api_version(0, 1, 0, 0);

        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(app_version)
            .engine_name(&engine_name)
            .engine_version(engine_version)
            .api_version(api_version);

        let mut compatibilities = InstanceCompatibilities::default();
        let instance_extensions =
            Self::get_required_instance_extensions(entry, front, &mut compatibilities)?;
        let instance_layers =
            Self::get_required_instance_layers(entry, front, &mut compatibilities)?;
        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&instance_extensions)
            .enabled_layer_names(&instance_layers);

        Ok((
            entry
                .create_instance(&create_info, None)
                .context("Failed to create Vulkan instance")?,
            compatibilities,
        ))
    }
}
