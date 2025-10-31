use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
#[allow(dead_code)]
pub struct TracerConfigInner {}

#[derive(Debug)]
#[allow(dead_code)]
pub struct TracerConfig(Rc<RefCell<TracerConfigInner>>);

impl Default for TracerConfig {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(TracerConfigInner {})))
    }
}

impl Clone for TracerConfig {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
