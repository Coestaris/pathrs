use crate::tracer::front::Front;
use ash::vk::PhysicalDevice;
use ash::Instance;

pub struct TracerHeadlessFront {}

impl TracerHeadlessFront {
    pub(crate) fn new() -> Self {
        todo!()
    }
}

impl Front for TracerHeadlessFront {}
