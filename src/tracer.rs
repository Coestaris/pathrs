use crate::assets::AssetManager;
use crate::back::pipeline::TracerPipeline;
use crate::back::{Back, BackQueues};
use crate::common::capabilities::{DeviceCapabilities, InstanceCapabilities};
use crate::common::queue::QueueFamily;
use crate::config::TracerConfig;
use crate::fps::FPSResult;
use crate::front::{Front, QueueFamilyIndices};
use anyhow::Context;
use ash::vk::{DeviceQueueCreateInfo, PhysicalDevice, PhysicalDeviceFeatures};
use ash::{vk, Entry, Instance};
use build_info::BuildInfo;
use glam::UVec2;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use log::{debug, info, warn};
use std::ffi::{c_char, CStr, CString};
use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone)]
pub struct TracerProfile {
    pub fps: FPSResult,
    pub render_time: f32,
}

pub struct DebugMessenger {
    handle: vk::DebugUtilsMessengerEXT,
    destroyed: bool,
}

#[allow(dead_code)]
impl DebugMessenger {
    unsafe extern "system" fn debug_callback(
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        message_types: vk::DebugUtilsMessageTypeFlagsEXT,
        p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
        _p_user_data: *mut std::ffi::c_void,
    ) -> vk::Bool32 {
        let message = CStr::from_ptr((*p_callback_data).p_message);
        let level = if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
            log::Level::Error
        } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING) {
            log::Level::Warn
        } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE) {
            log::Level::Debug
        } else {
            log::Level::Info
        };

        let mtype = if message_types.contains(vk::DebugUtilsMessageTypeFlagsEXT::GENERAL) {
            "GENERAL"
        } else if message_types.contains(vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION) {
            "VALIDATION"
        } else if message_types.contains(vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE) {
            "PERFORMANCE"
        } else {
            "UNKNOWN"
        };

        match level {
            log::Level::Error => warn!("[vulkan] {}: {}", mtype, message.to_string_lossy()),
            log::Level::Warn => info!("[vulkan] {}: {}", mtype, message.to_string_lossy()),
            log::Level::Debug => debug!("[vulkan] {}: {}", mtype, message.to_string_lossy()),
            log::Level::Info => info!("[vulkan] {}: {}", mtype, message.to_string_lossy()),
            _ => unreachable!(),
        }

        vk::FALSE
    }

    pub(super) unsafe fn get_required_instance_extensions(
        available: &Vec<String>,
        capabilities: &mut InstanceCapabilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        let mut required = vec![];
        if available.contains(&"VK_EXT_debug_utils".to_string()) {
            const VK_EXT_DEBUG_UTILS: &CStr =
                unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_EXT_debug_utils\0") };
            required.push(VK_EXT_DEBUG_UTILS.as_ptr());
            capabilities.debug_utils_ext = true;
        }

        Ok(required)
    }

    pub(super) unsafe fn get_required_instance_layers(
        available: &Vec<String>,
        capabilities: &mut InstanceCapabilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        let mut required = vec![];
        if available.contains(&"VK_LAYER_KHRONOS_validation".to_string()) {
            const VK_LAYER_KHRONOS_VALIDATION: &CStr =
                unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0") };
            required.push(VK_LAYER_KHRONOS_VALIDATION.as_ptr());
            capabilities.validation_layer = true;
        }

        Ok(required)
    }

    pub unsafe fn available(capabilities: &InstanceCapabilities) -> bool {
        capabilities.debug_utils_ext && capabilities.validation_layer
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
            .pfn_user_callback(Some(Self::debug_callback));

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

pub struct Tracer<F: Front> {
    viewport: UVec2,

    pub front: Option<F>,
    pub back: Option<Back>,

    pub entry: Entry,
    pub instance: Instance,
    pub debug_messenger: Option<DebugMessenger>,
    pub physical_device: PhysicalDevice,
    pub logical_device: ash::Device,
    pub allocator: Option<Arc<Mutex<Allocator>>>,
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

    unsafe fn get_required_device_extensions(
        _entry: &ash::Entry,
        _instance: &ash::Instance,
        available: &Vec<String>,
        front: &F,
        capabilities: &mut DeviceCapabilities,
    ) -> anyhow::Result<Vec<*const std::ffi::c_char>> {
        let mut required = vec![];
        required.extend(front.get_required_device_extensions(available, capabilities)?);
        required.extend(Back::get_required_device_extensions(
            available,
            capabilities,
        )?);
        Ok(required)
    }

    unsafe fn get_required_instance_extensions(
        entry: &Entry,
        capabilities: &mut InstanceCapabilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        let extensions = Self::get_instance_extensions(entry)?;
        debug!("Available instance extensions: {:?}", extensions);

        let mut required = vec![];
        required.extend(DebugMessenger::get_required_instance_extensions(
            &extensions,
            capabilities,
        )?);
        required.extend(F::get_required_instance_extensions(
            &extensions,
            capabilities,
        )?);
        required.extend(Back::get_required_instance_extensions(
            &extensions,
            capabilities,
        )?);
        Ok(required)
    }

    unsafe fn get_required_instance_layers(
        entry: &Entry,
        capabilities: &mut InstanceCapabilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        let layers = Self::get_instance_layers(entry)?;
        debug!("Available instance layers: {:?}", layers);

        let mut required = vec![];
        #[cfg(debug_assertions)]
        required.extend(DebugMessenger::get_required_instance_layers(
            &layers,
            capabilities,
        )?);
        required.extend(F::get_required_instance_layers(&layers, capabilities)?);
        required.extend(Back::get_required_instance_layers(
            &layers,
            capabilities,
        )?);
        Ok(required)
    }

    unsafe fn new_entry() -> anyhow::Result<Entry> {
        Ok(Entry::load()?)
    }

    pub unsafe fn new_instance(
        entry: &Entry,
        bi: BuildInfo,
    ) -> anyhow::Result<(ash::Instance, InstanceCapabilities)> {
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

        let mut capabilities = InstanceCapabilities::default();
        let instance_extensions =
            Self::get_required_instance_extensions(entry, &mut capabilities)?;
        let instance_layers = Self::get_required_instance_layers(entry, &mut capabilities)?;
        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&instance_extensions)
            .enabled_layer_names(&instance_layers);

        Ok((
            entry
                .create_instance(&create_info, None)
                .context("Failed to create Vulkan instance")?,
            capabilities,
        ))
    }

    unsafe fn is_device_suitable(
        entry: &ash::Entry,
        instance: &ash::Instance,
        front: &F,
        capabilities: &mut DeviceCapabilities,
        device: vk::PhysicalDevice,
    ) -> bool {
        let extensions = Self::get_device_extensions(instance, device).unwrap_or(vec![]);
        let required_extensions = Self::get_required_device_extensions(
            entry,
            instance,
            &extensions,
            front,
            capabilities,
        )
        .unwrap_or(vec![]);
        let extensions_ok = is_subset(&extensions, &required_extensions).unwrap_or(false);

        let front_ok = front
            .is_device_suitable(entry, instance, device)
            .unwrap_or(false);
        let back_ok = Back::is_device_suitable(entry, instance, device).unwrap_or(false);

        let properties = instance.get_physical_device_properties(device);
        debug!("Device: {:?}", properties);

        debug!(
            "extensions_ok: {}, front_ok: {}, back_ok: {}",
            extensions_ok, front_ok, back_ok
        );

        extensions_ok && front_ok && back_ok
    }

    unsafe fn find_suitable_device(
        entry: &ash::Entry,
        instance: &ash::Instance,
        front: &F,
    ) -> anyhow::Result<vk::PhysicalDevice> {
        let devices = instance
            .enumerate_physical_devices()
            .context("Failed to enumerate physical devices")?;

        for device in devices {
            let mut capabilities = DeviceCapabilities::default();

            // TODO: Implement some kind of scoring system for compatibility
            if Self::is_device_suitable(entry, instance, front, &mut capabilities, device) {
                return Ok(device);
            }
        }

        Err(anyhow::anyhow!("No suitable physical device found"))
    }

    unsafe fn new_allocator(
        instance: Instance,
        device: ash::Device,
        physical_device: PhysicalDevice,
    ) -> anyhow::Result<Arc<Mutex<Allocator>>> {
        Ok(Arc::new(Mutex::new(Allocator::new(
            &AllocatorCreateDesc {
                instance,
                device,
                physical_device,
                debug_settings: Default::default(),
                buffer_device_address: true,
                allocation_sizes: Default::default(),
            },
        )?)))
    }

    pub unsafe fn new_device(
        entry: &ash::Entry,
        instance: &Instance,
        front: &mut F,
    ) -> anyhow::Result<(
        Arc<Mutex<Allocator>>,
        BackQueues,
        <<F as Front>::FrontQueueFamilyIndices as QueueFamilyIndices>::Queues,
        vk::PhysicalDevice,
        ash::Device,
    )> {
        let physical_device = Self::find_suitable_device(entry, instance, front)?;

        let mut capabilities = DeviceCapabilities::default();
        let extensions = Self::get_device_extensions(instance, physical_device)?;
        let extensions = Self::get_required_device_extensions(
            entry,
            instance,
            &extensions,
            front,
            &mut capabilities,
        )?;

        let back_queues = Back::find_queue_families(entry, instance, physical_device)?;
        debug!("Using back queue families: {:?}", back_queues);
        let font_queues = front.find_queue_families(entry, instance, physical_device)?;
        debug!("Using front queue families: {:?}", font_queues);

        let mut queue_family_infos = vec![];
        queue_family_infos.extend(back_queues.as_families());
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
            .queue_create_infos(&queue_create_infos)
            .enabled_features(&features)
            .flags(vk::DeviceCreateFlags::empty());

        let logical_device = front.patch_create_device_info(
            entry,
            instance,
            physical_device,
            device_create_info,
            &mut |device_create_info| {
                Back::patch_create_device_info(
                    entry,
                    instance,
                    physical_device,
                    device_create_info,
                    &mut |device_create_info| {
                        instance
                            .create_device(physical_device, &device_create_info, None)
                            .context("Failed to create logical device")
                    },
                )
            },
        )?;

        let back_queues = back_queues.into_queues(&logical_device)?;
        debug!("Acquired common queues: {:?}", back_queues);
        let front_queues = font_queues.into_queues(&logical_device)?;
        debug!("Acquired front queues: {:?}", front_queues);

        debug!("Creating allocator");
        let allocator =
            Self::new_allocator(instance.clone(), logical_device.clone(), physical_device)?;

        Ok((
            allocator,
            back_queues,
            front_queues,
            physical_device,
            logical_device,
        ))
    }

    pub(crate) unsafe fn new<D: Front>(
        config: TracerConfig,
        asset_manager: AssetManager,
        viewport: UVec2,
        bi: BuildInfo,
        constructor: impl FnOnce(&Entry, &Instance) -> anyhow::Result<D>,
    ) -> anyhow::Result<Tracer<D>> {
        info!("Creating Vulkan instance");
        let entry = Self::new_entry()?;

        info!("Created Vulkan entry");
        let (instance, _instance_capabilities) = Self::new_instance(&entry, bi)?;

        info!("Created Front");
        let mut front =
            constructor(&entry, &instance).context("Failed to create tracer front-end")?;

        #[cfg(debug_assertions)]
        let debug_messenger = if DebugMessenger::available(&_instance_capabilities) {
            info!("Setting up debug messanger");
            Some(
                DebugMessenger::new(&entry, &instance)
                    .context("Failed to create debug messanger")?,
            )
        } else {
            warn!("Debug messanger not supported on this system");
            None
        };
        #[cfg(not(debug_assertions))]
        let debug_messenger = None;

        info!("Creating logical device");
        let (allocator, back_queues, front_queues, physical_device, logical_device) =
            Tracer::<D>::new_device(&entry, &instance, &mut front)?;

        info!("Initializing back-end");
        let back = Back::new(
            allocator.clone(),
            asset_manager.clone(),
            viewport,
            &instance,
            physical_device,
            &logical_device,
            back_queues,
            config,
        )
        .context("Failed to create tracer pipeline")?;

        info!("Initializing front-end");
        front.init(
            &entry,
            &instance,
            &logical_device,
            physical_device,
            front_queues,
            allocator.clone(),
        )?;

        Ok(Tracer {
            viewport,
            front: Some(front),
            back: Some(back),
            entry,
            instance,
            debug_messenger,
            physical_device,
            logical_device,
            allocator: Some(allocator),
        })
    }

    pub unsafe fn trace(&mut self, w: Option<&winit::window::Window>) -> anyhow::Result<()> {
        let slot = self
            .back
            .as_mut()
            .unwrap()
            .present(&self.logical_device)
            .context("Failed to present tracer back-end")?;

        self.front
            .as_mut()
            .unwrap()
            .present(
                w,
                &self.entry,
                &self.instance,
                &self.logical_device,
                self.physical_device,
                slot,
            )
            .context("Failed to present tracer front")?;

        Ok(())
    }

    pub unsafe fn resize(&mut self, size: UVec2) -> anyhow::Result<()> {
        self.viewport = size;

        self.back
            .as_mut()
            .unwrap()
            .resize(&self.logical_device, size)
            .with_context(|| format!("Failed to resize tracer back-end to {:?}", size))?;

        self.front
            .as_mut()
            .unwrap()
            .resize(
                &self.entry,
                &self.instance,
                &self.logical_device,
                self.physical_device,
                size,
            )
            .with_context(|| format!("Failed to resize tracer front to {:?}", size))?;

        Ok(())
    }

    pub fn get_profile(&self) -> TracerProfile {
        self.back.as_ref().unwrap().get_profile()
    }
}

impl<F: Front> Drop for Tracer<F> {
    fn drop(&mut self) {
        unsafe {
            if let Some(mut back) = self.back.take() {
                debug!("Destroying back-end");
                back.destroy(&self.logical_device);
            }

            if let Some(mut front) = self.front.take() {
                debug!("Destroying front-end");
                front.destroy(&self.entry, &self.instance, &self.logical_device);
            }

            debug!("Destroying allocator");
            if let Some(allocator) = self.allocator.take() {
                let mutex = Arc::try_unwrap(allocator).unwrap();
                let allocator = mutex.into_inner().unwrap();
                drop(allocator);
            }

            debug!("Destroying logical device");
            self.logical_device.destroy_device(None);

            debug!("Destroying debug messanger");
            if let Some(mut debug_messenger) = self.debug_messenger.take() {
                debug_messenger.destroy(&self.entry, &self.instance);
            }

            debug!("Destroying Vulkan instance");
            self.instance.destroy_instance(None);
        }
    }
}
