mod back;
mod config;
pub mod front;
mod instance;
mod debug_messanger;

use crate::tracer::back::TracerBack;
use crate::tracer::config::TracerConfig;
use crate::tracer::front::{TracerFront, TracerSurface};
use anyhow::Context;
use build_info::BuildInfo;
use glam::UVec2;

pub struct Tracer {
    back: TracerBack,
    config: TracerConfig,
    viewport: UVec2,
}

impl Tracer {
    pub fn new_headless(
        viewport: UVec2,
        bi: BuildInfo,
        config: TracerConfig,
    ) -> anyhow::Result<Self> {
        unsafe {
            Ok(Self {
                back: TracerBack::new(viewport, bi, TracerFront::new_headless())?,
                config,
                viewport,
            })
        }
    }

    pub fn new_windowed(
        viewport: UVec2,
        bi: BuildInfo,
        config: TracerConfig,
        surface: TracerSurface,
    ) -> anyhow::Result<Self> {
        unsafe {
            Ok(Self {
                back: TracerBack::new(viewport, bi, TracerFront::new_windowed(surface))?,
                config,
                viewport,
            })
        }
    }

    pub fn resize(&mut self, size: UVec2) -> anyhow::Result<()> {
        self.viewport = size;
        self.back
            .resize(size)
            .context("Failed to resize tracer back")?;
        Ok(())
    }

    pub fn trace(&mut self) -> anyhow::Result<()> {
        self.back.trace().context("Failed to trace")?;
        Ok(())
    }
}
