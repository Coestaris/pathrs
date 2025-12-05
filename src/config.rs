use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize, Serializer};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct Camera {
    pub position: Vec3,
    pub direction: Vec3,
    pub fov: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            direction: Vec3::new(0.0, 0.0, -1.0),
            fov: std::f32::consts::FRAC_PI_2,
        }
    }
}

impl Camera {
    pub fn as_transform(&self) -> Mat4 {
        let forward = self.direction.normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = -right.cross(forward);

        Mat4::from_cols(
            right.extend(0.0),
            up.extend(0.0),
            (-forward).extend(0.0),
            self.position.extend(1.0),
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Material {
    pub albedo: Vec3,
    pub emission_color: Vec3,
    pub emission_strength: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Object {
    Sphere {
        center: Vec3,
        radius: f32,
        material: Material,
    },
}

impl Object {
    pub fn as_material_mut(&mut self) -> &mut Material {
        match self {
            Object::Sphere { material, .. } => material,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TracerConfigInner {
    pub camera: Camera,
    pub objects: Vec<Object>,
    pub samples_count: u32,
    pub jitter_strength: f32,
    pub max_bounces: u32,
    pub sky_color_top: Vec3,
    pub sky_color_bottom: Vec3,
    pub ground_color: Vec3,
    
    pub updated: bool,
    pub objects_updated: bool,
}

#[allow(dead_code)]
fn scene_simple() -> Vec<Object> {
    vec![
        Object::Sphere {
            center: Vec3::new(-8.0, 4.5, -9.0),
            radius: 3.0,
            material: Material {
                albedo: Vec3::new(0.0, 0.0, 0.0),
                emission_color: Vec3::new(1.0, 1.0, 1.0),
                emission_strength: 3.00,
            },
        },
        Object::Sphere {
            center: Vec3::new(16.0, 4.5, -9.0),
            radius: 3.0,
            material: Material {
                albedo: Vec3::new(0.0, 0.0, 0.0),
                emission_color: Vec3::new(1.0, 1.0, 1.0),
                emission_strength: 3.00,
            },
        },
        Object::Sphere {
            center: Vec3::new(0.0, -100.5, -1.0),
            radius: 100.0,
            material: Material {
                albedo: Vec3::new(0.2, 0.4, 0.4),
                emission_color: Vec3::new(0.0, 0.0, 0.0),
                emission_strength: 0.00,
            },
        },
        Object::Sphere {
            center: Vec3::new(0.0, 0.0, -1.2),
            radius: 0.5,
            material: Material {
                albedo: Vec3::new(0.1, 0.2, 0.5),
                emission_color: Vec3::new(0.0, 0.0, 0.0),
                emission_strength: 0.00,
            },
        },
        Object::Sphere {
            center: Vec3::new(-1.0, 0.0, -1.0),
            radius: 0.5,
            material: Material {
                albedo: Vec3::new(0.8, 0.8, 0.8),
                emission_color: Vec3::new(0.0, 0.0, 0.0),
                emission_strength: 0.00,
            },
        },
        Object::Sphere {
            center: Vec3::new(1.0, 0.0, -1.0),
            radius: 0.5,
            material: Material {
                albedo: Vec3::new(0.8, 0.6, 0.2),
                emission_color: Vec3::new(0.0, 0.0, 0.0),
                emission_strength: 0.00,
            },
        },
    ]
}

#[allow(dead_code)]
fn scene_array() -> Vec<Object> {
    let mut objects = Vec::new();
    const ALBEDO: Vec3 = Vec3::new(0.1, 0.2, 0.5);
    const RADIUS: f32 = 0.5;
    const OFFSET: f32 = RADIUS * 2.3;
    const SIDE: usize = 8;

    for x in 0..SIDE {
        for y in 0..SIDE {
            objects.push(Object::Sphere {
                center: Vec3::new(
                    x as f32 * OFFSET - OFFSET / 2.0,
                    y as f32 * OFFSET - OFFSET / 2.0,
                    0.0,
                ),
                radius: RADIUS,
                material: Material {
                    albedo: ALBEDO,
                    emission_color: Vec3::new(0.0, 0.0, 0.0),
                    emission_strength: 0.00,
                },
            })
        }
    }

    objects
}

impl Default for TracerConfigInner {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            objects: scene_simple(),
            // objects: scene_array(),
            samples_count: 1,
            jitter_strength: 0.8,
            max_bounces: 5,
            sky_color_top: Vec3::new(1.0, 1.0, 1.0),
            sky_color_bottom: Vec3::new(0.5, 0.7, 1.0),
            ground_color: Vec3::new(0.8, 0.8, 0.0),
            updated: true,
            objects_updated: true,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct TracerConfig(pub Rc<RefCell<TracerConfigInner>>);

impl Default for TracerConfig {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(TracerConfigInner::default())))
    }
}

impl Serialize for TracerConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.borrow().serialize(serializer)
    }
}

impl<'a> Deserialize<'a> for TracerConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        let inner = TracerConfigInner::deserialize(deserializer)?;
        Ok(TracerConfig(Rc::new(RefCell::new(inner))))
    }
}

impl Clone for TracerConfig {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl TracerConfig {}
