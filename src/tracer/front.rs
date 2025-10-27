use winit::window::Window;

#[derive(Debug)]
pub struct TracerSurface {}

impl TracerSurface {
    pub fn new_from_winit(_: &Window) -> anyhow::Result<Self> {
        Ok(Self {})
    }
}

#[derive(Debug)]
pub struct WindowedTracerFront {
    surface: TracerSurface,
}

#[derive(Debug)]
pub(super) enum TracerFront {
    Headless,
    Windowed(WindowedTracerFront),
}

impl TracerFront {
    pub fn new_headless() -> Self {
        Self::Headless
    }

    pub fn new_windowed(surface: TracerSurface) -> Self {
        Self::Windowed(WindowedTracerFront { surface })
    }

    pub fn resize(&mut self, _size: glam::UVec2) -> anyhow::Result<()> {
        // Resize logic for the front-end
        Ok(())
    }

    pub fn present(&mut self) -> anyhow::Result<()> {
        // Presentation logic for the front-end
        Ok(())
    }
}
