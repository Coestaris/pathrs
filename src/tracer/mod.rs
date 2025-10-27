mod back;
mod config;
pub mod front;

use crate::tracer::back::TracerBack;
use crate::tracer::config::TracerConfig;
use crate::tracer::front::{TracerFront, TracerSurface};
use anyhow::Context;
use glam::UVec2;

#[derive(Debug)]
pub struct Tracer {
    front: TracerFront,
    back: TracerBack,
    config: TracerConfig,
    viewport: UVec2,
}

impl Tracer {
    pub fn new_headless(viewport: UVec2, config: TracerConfig) -> anyhow::Result<Self> {
        Ok(Self {
            front: TracerFront::new_headless(),
            back: TracerBack::new(viewport)?,
            config,
            viewport,
        })
    }

    pub fn new_windowed(
        viewport: UVec2,
        config: TracerConfig,
        surface: TracerSurface,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            front: TracerFront::new_windowed(surface),
            back: TracerBack::new(viewport)?,
            config,
            viewport,
        })
    }

    pub fn resize(&mut self, size: UVec2) -> anyhow::Result<()> {
        self.viewport = size;
        self.back
            .resize(size)
            .context("Failed to resize tracer back")?;
        self.front
            .resize(size)
            .with_context(|| format!("Failed to resize tracer front to {:?}", size))?;
        Ok(())
    }

    pub fn trace(&mut self) -> anyhow::Result<()> {
        // 1. Trace
        self.back.trace().context("Failed to trace")?;

        // 2. Present
        self.front
            .present()
            .context("Failed to present tracer front")?;

        Ok(())
    }
}
