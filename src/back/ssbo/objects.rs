use crate::back::ssbo::SSBO;
use crate::config::Material;
use glam::Vec3;

const OBJECT_TYPE_SPHERE: u32 = 1;

pub const MAX_OBJECTS: usize = 128;

#[derive(Default, Clone, Debug)]
#[repr(C)]
#[repr(align(16))]
#[derive(Copy)]
pub struct SSBOObjectData {
    pub object_type: [u32; 4],
    pub albedo: [f32; 4],
    pub metallic_roughness: [f32; 4],
    pub data2: [f32; 4],
    pub data3: [f32; 4],
}

impl SSBOObjectData {
    pub(crate) fn new_sphere(center: Vec3, radius: f32, material: &Material) -> Self {
        Self {
            object_type: [OBJECT_TYPE_SPHERE, 0, 0, 0],
            albedo: [material.albedo.x, material.albedo.y, material.albedo.z, 0.0],
            metallic_roughness: [material.metallic, material.roughness, 0.0, 0.0],
            data2: [center[0], center[1], center[2], 0.0],
            data3: [radius, 0.0, 0.0, 0.0],
        }
    }
}

pub type SSBOObjectsData = [SSBOObjectData; MAX_OBJECTS];
pub type SSBOObjects = SSBO<SSBOObjectsData>;
