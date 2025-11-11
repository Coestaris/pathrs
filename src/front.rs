use crate::vk::device::{DeviceCompatibilities, QueueFamily};
use crate::vk::instance::InstanceCompatibilities;
use ash::{vk, Device};
use std::ffi::c_char;
use std::fmt::Debug;

pub trait QueueFamilyIndices {
    type Queues: Debug;

    fn as_families(&self) -> Vec<QueueFamily>;
    unsafe fn into_queues(self, device: &Device) -> anyhow::Result<Self::Queues>;
}

pub trait Front {
    type FrontQueueFamilyIndices: QueueFamilyIndices + Debug;

    unsafe fn get_required_instance_extensions(
        _available: &Vec<String>,
        _compatibilities: &mut InstanceCompatibilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![])
    }

    unsafe fn get_required_instance_layers(
        _available: &Vec<String>,
        _compatibilities: &mut InstanceCompatibilities,
    ) -> anyhow::Result<Vec<*const c_char>> {
        Ok(vec![])
    }

    unsafe fn get_required_device_extensions(
        &self,
        _available: &Vec<String>,
        _compatibilities: &mut DeviceCompatibilities,
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

    unsafe fn set_device(
        &mut self,
        _entry: &ash::Entry,
        _instance: &ash::Instance,
        _device: &Device,
        _physical_device: vk::PhysicalDevice,
        _queues: <<Self as Front>::FrontQueueFamilyIndices as QueueFamilyIndices>::Queues,
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
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
