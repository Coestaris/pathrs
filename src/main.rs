use crate::app::App;
use crate::logging::setup_logging;
use glam::UVec2;
use log::LevelFilter;
use winit::event_loop::{ControlFlow, EventLoop};

mod app;
mod logging;
mod tracer;

build_info::build_info!(pub fn get_build_info);

static VIEWPORT: UVec2 = UVec2::new(1280, 720);

fn main() -> anyhow::Result<()> {
    setup_logging(LevelFilter::Debug, None, true);

    // TODO: Headless mode

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(VIEWPORT, get_build_info().clone());
    event_loop.run_app(&mut app)?;

    Ok(())
}
