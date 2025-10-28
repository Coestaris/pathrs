use crate::tracer::device::{DeviceCompatibilities, QueueFamily};
use crate::tracer::instance::InstanceCompatibilities;
use ash::vk;
use std::ffi::c_char;

pub mod headless;
pub mod windowed;

pub trait Front {
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

    unsafe fn get_required_device_layers(
        &self, _available: &Vec<String>,
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
        &mut self,
        _entry: &ash::Entry,
        _instance: &ash::Instance,
        _physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<Vec<QueueFamily>> {
        Ok(vec![])
    }

    unsafe fn destroy(&mut self, _entry: &ash::Entry, _instance: &ash::Instance) {}

    unsafe fn resize(&mut self, _size: glam::UVec2) -> anyhow::Result<()> {
        Ok(())
    }

    unsafe fn present(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
