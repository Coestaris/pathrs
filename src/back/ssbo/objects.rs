use crate::back::ssbo::SSBO;
use glam::{Vec3, Vec4};

const OBJECT_TYPE_SPHERE: u32 = 1;

pub const MAX_OBJECTS: usize = 128;

#[derive(Default, Clone, Debug)]
#[repr(C)]
#[repr(align(16))]
#[derive(Copy)]
pub struct SSBOObjectData {
    pub object_type: [u32; 4],
    pub color: [f32; 4],
    pub data2: [f32; 4],
    pub data3: [f32; 4],
}

impl SSBOObjectData {
    pub(crate) fn new_sphere(center: Vec3, radius: f32, color: Vec4) -> Self {
        Self {
            object_type: [OBJECT_TYPE_SPHERE, 0, 0, 0],
            color: [color.x, color.y, color.z, color.w],
            data2: [center[0], center[1], center[2], 0.0],
            data3: [radius, 0.0, 0.0, 0.0],
        }
    }
}

pub type SSBOObjectsData = [SSBOObjectData; MAX_OBJECTS];
pub type SSBOObjects = SSBO<SSBOObjectsData>;
