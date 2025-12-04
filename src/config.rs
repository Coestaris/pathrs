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
pub enum Object {
    Sphere {
        center: Vec3,
        radius: f32,
        color: Vec3,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TracerConfigInner {
    pub camera: Camera,
    pub objects: Vec<Object>,
    pub samples_count: u32,
    pub jitter_strength: f32,
    pub temporal_accumulation: f32,
    pub updated: bool,
}

impl Default for TracerConfigInner {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            objects: vec![
                Object::Sphere {
                    center: Vec3::new(0.0, 0.0, -1.0),
                    radius: 0.5,
                    color: Vec3::new(1.0, 0.0, 0.0),
                },
                Object::Sphere {
                    center: Vec3::new(0.0, -100.5, -1.0),
                    radius: 100.0,
                    color: Vec3::new(0.3, 0.8, 0.2),
                },
            ],
            samples_count: 4,
            jitter_strength: 0.8,
            temporal_accumulation: 0.9,
            updated: true,
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
