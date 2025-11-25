use std::time::Instant;

const FPS_CALCULATE_INTERVAL: u128 = 500; // in milliseconds

#[derive(Debug, Clone, Copy)]
pub enum FPSResult {
    Updated(f32),
    Cached(f32),
}

impl Default for FPSResult {
    fn default() -> Self {
        Self::Cached(0.0)
    }
}

impl FPSResult {
    pub fn fps(self) -> f32 {
        match self {
            Self::Updated(fps) => fps,
            Self::Cached(fps) => fps,
        }
    }
}

pub struct Fps {
    prev_calculate: Instant,
    accumulated: u32,
    fps: f32,
}

impl Fps {
    pub(crate) fn new() -> Self {
        Self {
            prev_calculate: Instant::now(),
            accumulated: 0,
            fps: 0.0,
        }
    }

    pub(crate) fn update(&mut self) -> FPSResult {
        let now = Instant::now();
        let elapsed = now.duration_since(self.prev_calculate).as_millis();
        self.accumulated += 1;
        if elapsed > FPS_CALCULATE_INTERVAL {
            let fps = (self.accumulated as f32) * 1000.0 / (elapsed as f32);
            self.accumulated = 0;
            self.prev_calculate = now;
            self.fps = fps;
            FPSResult::Updated(fps)
        } else {
            FPSResult::Cached(self.fps)
        }
    }
}
