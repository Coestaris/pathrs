use crate::back::TracerSlot;
use crate::common::capabilities::{DeviceCapabilities, InstanceCapabilities};
use crate::common::queue::QueueFamily;
use ash::{vk, Device};
use gpu_allocator::vulkan::Allocator;
use std::ffi::c_char;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

pub mod headless;
pub mod windowed;

pub trait QueueFamilyIndices {
    type Queues: Debug;

    fn as_families(&self) -> Vec<QueueFamily>;
    unsafe fn into_queues(self, device: &Device) -> anyhow::Result<Self::Queues>;
}

pub trait Front {
    type FrontQueueFamilyIndices: QueueFamilyIndices + Debug;

    unsafe fn get_required_instance_extensions(
        _available: &Vec<String>,
        _capabilities: &mut InstanceCapabilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![])
    }

    unsafe fn get_required_instance_layers(
        _available: &Vec<String>,
        _capabilities: &mut InstanceCapabilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![])
    }

    unsafe fn get_required_device_extensions(
        &self,
        _available: &Vec<String>,
        _capabilities: &mut DeviceCapabilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![])
    }

    unsafe fn is_device_suitable(
        &self,
        _entry: &ash::Entry,
        _instance: &ash::Instance,
        _physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<bool> {
        Ok(true)
    }

    unsafe fn find_queue_families(
        &self,
        _entry: &ash::Entry,
        _instance: &ash::Instance,
        _physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<Self::FrontQueueFamilyIndices>;

    unsafe fn patch_create_device_info(
        &self,
        _entry: &ash::Entry,
        _instance: &ash::Instance,
        _physical_device: vk::PhysicalDevice,
        create_info: vk::DeviceCreateInfo,
        on_patched: &mut impl FnMut(vk::DeviceCreateInfo) -> anyhow::Result<ash::Device>,
    ) -> anyhow::Result<ash::Device> {
        on_patched(create_info)
    }

    unsafe fn init(
        &mut self,
        _entry: &ash::Entry,
        _instance: &ash::Instance,
        _device: &Device,
        _physical_device: vk::PhysicalDevice,
        _queues: <<Self as Front>::FrontQueueFamilyIndices as QueueFamilyIndices>::Queues,
        _allocator: Arc<Mutex<Allocator>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    unsafe fn destroy(&mut self, _entry: &ash::Entry, _instance: &ash::Instance, _device: &Device) {
    }

    unsafe fn resize(
        &mut self,
        _entry: &ash::Entry,
        _instance: &ash::Instance,
        _device: &Device,
        _physical_device: vk::PhysicalDevice,
        _size: glam::UVec2,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    unsafe fn present(
        &mut self,
        _w: Option<&winit::window::Window>, // ???
        _entry: &ash::Entry,
        _instance: &ash::Instance,
        _device: &Device,
        _physical_device: vk::PhysicalDevice,
        _tracer_slot: TracerSlot,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
