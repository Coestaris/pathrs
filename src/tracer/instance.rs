use crate::tracer::debug_messanger::DebugMessenger;
use crate::tracer::front::Front;
use crate::tracer::Tracer;
use anyhow::Context;
use ash::{vk, Entry};
use build_info::BuildInfo;
use log::{debug, warn};
use std::ffi::{c_char, CStr, CString};

pub struct InstanceCompatibilities {
    pub debug_utils_ext: bool,
    pub validation_layer: bool,
}

impl Default for InstanceCompatibilities {
    fn default() -> Self {
        Self {
            debug_utils_ext: false,
            validation_layer: false,
        }
    }
}

impl<F: Front> Tracer<F> {
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
        compatibilities: &mut InstanceCompatibilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        let extensions = Self::get_instance_extensions(entry)?;
        debug!("Available instance extensions: {:?}", extensions);

        let mut required = vec![];
        required.extend(DebugMessenger::get_required_instance_extensions(
            &extensions,
            compatibilities,
        )?);
        required.extend(F::get_required_instance_extensions(
            &extensions,
            compatibilities,
        )?);
        Ok(required)
    }

    unsafe fn get_required_instance_layers(
        entry: &Entry,
        compatibilities: &mut InstanceCompatibilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        let layers = Self::get_instance_layers(entry)?;
        debug!("Available instance layers: {:?}", layers);

        let mut required = vec![];
        required.extend(DebugMessenger::get_required_instance_layers(
            &layers,
            compatibilities,
        )?);
        required.extend(F::get_required_instance_layers(&layers, compatibilities)?);
        Ok(required)
    }

    pub unsafe fn new_instance(
        entry: &Entry,
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
        let api_version = vk::make_api_version(0, 1, 3, 0);

        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(app_version)
            .engine_name(&engine_name)
            .engine_version(engine_version)
            .api_version(api_version);

        let mut compatibilities = InstanceCompatibilities::default();
        let instance_extensions =
            Self::get_required_instance_extensions(entry, &mut compatibilities)?;
        let instance_layers = Self::get_required_instance_layers(entry, &mut compatibilities)?;
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
