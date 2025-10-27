use glam::UVec2;

#[derive(Debug)]
pub(super) struct TracerBack {
    viewport: UVec2,
}

impl TracerBack {
    pub fn new(viewport: UVec2) -> anyhow::Result<Self> {
        Ok(Self { viewport })
    }

    pub fn trace(&mut self) -> anyhow::Result<()> {
        // Tracing logic for the back-end
        Ok(())
    }

    pub fn resize(&mut self, size: UVec2) -> anyhow::Result<()> {
        self.viewport = size;
        Ok(())
    }
}
