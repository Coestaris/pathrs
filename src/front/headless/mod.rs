use crate::config::TracerConfig;
use crate::front::headless::front::TracerHeadlessFront;
use crate::tracer::Tracer;
use build_info::BuildInfo;
use glam::UVec2;

mod front;

pub struct TracerHeadlessOutput {
    pub width: u32,
    pub height: u32,
    pub rgb888: Vec<u8>,
}

pub unsafe fn headless_tracer<C>(
    config: TracerConfig,
    viewport: UVec2,
    bi: BuildInfo,
    callback: C,
) -> anyhow::Result<Tracer<TracerHeadlessFront>>
where
    C: FnMut(TracerHeadlessOutput) + Send + 'static,
{
    Tracer::<TracerHeadlessFront>::new(config, viewport, bi, |_, _| {
        Ok(TracerHeadlessFront::new(callback))
    })
}
