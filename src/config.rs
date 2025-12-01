use serde::{Deserialize, Serialize, Serializer};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TracerConfigInner {
    pub slider: f32,
}

impl Default for TracerConfigInner {
    fn default() -> Self {
        Self { slider: 0.0 }
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
