use crate::back::TracerSlot;
use crate::common::capabilities::DeviceCapabilities;
use crate::common::queue::QueueFamily;
use crate::front::headless::TracerHeadlessOutput;
use crate::front::{Front, QueueFamilyIndices};
use crate::tracer::Bundle;
use ash::{vk, Device, Entry, Instance};
use log::info;
use std::ffi::{c_char, c_void};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HeadlessQueueFamilyIndices {}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HeadlessQueues {}

impl QueueFamilyIndices for HeadlessQueueFamilyIndices {
    type Queues = HeadlessQueues;

    fn as_families(&self) -> Vec<QueueFamily> {
        vec![]
    }

    unsafe fn into_queues(self, _device: &Device) -> anyhow::Result<Self::Queues> {
        Ok(HeadlessQueues {})
    }
}

#[allow(dead_code)]
pub struct TracerHeadlessFront {
    callback: Box<dyn FnMut(TracerHeadlessOutput) + Send>,
}

impl TracerHeadlessFront {
    pub(crate) fn new<F>(callback: F) -> Self
    where
        F: FnMut(TracerHeadlessOutput) + Send + 'static,
    {
        Self {
            callback: Box::new(callback),
        }
    }
}

impl TracerHeadlessOutput {
    pub fn from_rgba8888(width: u32, height: u32, rgba8888: Vec<u8>) -> Self {
        Self {
            width,
            height,
            rgb888: rgba8888
                .chunks(4)
                .flat_map(|pixel| vec![pixel[0], pixel[1], pixel[2]])
                .collect(),
        }
    }
}

impl Front for TracerHeadlessFront {
    type FrontQueueFamilyIndices = HeadlessQueueFamilyIndices;

    unsafe fn get_required_device_extensions(
        &self,
        available: &Vec<String>,
        capabilities: &mut DeviceCapabilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        let mut required = vec![];
        if available.contains(&ash::ext::host_image_copy::NAME.to_str()?.to_string()) {
            capabilities.host_image_copy = true;
            info!("Image copy extension required");
            required.push(ash::ext::host_image_copy::NAME.as_ptr());
        }

        Ok(required)
    }

    unsafe fn find_queue_families(
        &self,
        _entry: &Entry,
        _instance: &Instance,
        _physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<HeadlessQueueFamilyIndices> {
        Ok(HeadlessQueueFamilyIndices {})
    }

    unsafe fn patch_create_device_info(
        &self,
        _entry: &Entry,
        _instance: &Instance,
        _physical_device: vk::PhysicalDevice,
        device_capabilities: &DeviceCapabilities,
        create_info: vk::DeviceCreateInfo,
        on_patched: &mut impl FnMut(vk::DeviceCreateInfo) -> anyhow::Result<Device>,
    ) -> anyhow::Result<Device> {
        if device_capabilities.host_image_copy {
            let mut physical_device_host_image_copy_features =
                vk::PhysicalDeviceHostImageCopyFeaturesEXT::default().host_image_copy(true);
            let create_info = create_info.push_next(&mut physical_device_host_image_copy_features);
            on_patched(create_info)
        } else {
            on_patched(create_info)
        }
    }

    unsafe fn present(
        &mut self,
        bundle: Bundle,
        _w: Option<&winit::window::Window>,
        slot: TracerSlot,
    ) -> anyhow::Result<()> {
        info!("Presenting frame");

        let memory = vec![0u8; slot.image.byte_size];
        if bundle.device_capabilities.host_image_copy {
            let factory = ash::ext::host_image_copy::Device::new(&bundle.instance, &bundle.device);
            let regions = vk::ImageToMemoryCopyEXT::default()
                .host_pointer(memory.as_ptr() as *mut c_void)
                .image_subresource(
                    vk::ImageSubresourceLayers::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .mip_level(0)
                        .base_array_layer(0)
                        .layer_count(1),
                )
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D {
                    width: slot.image.dimensions.x,
                    height: slot.image.dimensions.y,
                    depth: 1,
                });
            let copy_image_to_memory_info = vk::CopyImageToMemoryInfoEXT::default()
                .regions(std::slice::from_ref(&regions))
                .src_image(slot.image.image)
                .src_image_layout(slot.image.layout);
            factory.copy_image_to_memory(&copy_image_to_memory_info)?;
        } else {
            unimplemented!("Not yet implemented without host image copy extension")
        }

        let data = match slot.image.format {
            vk::Format::R8G8B8A8_UNORM => TracerHeadlessOutput::from_rgba8888(
                slot.image.dimensions.x,
                slot.image.dimensions.y,
                memory,
            ),
            _ => panic!("Unsupported image format"),
        };

        (self.callback)(data);

        Ok(())
    }
}
