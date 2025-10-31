use crate::app::App;
use crate::logging::setup_logging;
use crate::tracer::config::TracerConfig;
use crate::tracer::front::headless::TracerHeadlessFront;
use crate::tracer::front::windowed::TracerWindowedFront;
use crate::tracer::Tracer;
use clap::Parser;
use glam::UVec2;
use log::{info, LevelFilter};
use winit::event_loop::{ControlFlow, EventLoop};

mod app;
mod logging;
mod tracer;

build_info::build_info!(pub fn get_build_info);

#[derive(Parser, Debug)]
struct Arguments {
    #[clap(long, default_value_t = 1280)]
    width: u32,
    #[clap(long, default_value_t = 720)]
    height: u32,
    #[clap(short = 'd', long)]
    headless: bool,
}

fn main() -> anyhow::Result<()> {
    setup_logging(LevelFilter::Debug, None, true);

    let args = Arguments::parse();
    info!("Starting application with args: {:?}", args);

    let viewport = UVec2::new(args.width, args.height);
    if args.headless {
        let config = TracerConfig::default();
        unsafe {
            let mut tracer = Tracer::<TracerHeadlessFront>::new_headless(
                config,
                viewport,
                get_build_info().clone(),
                |output| {
                    // TODO: Save to file or process output
                    info!(
                        "Received headless output: {}x{}, {} bytes",
                        output.width,
                        output.height,
                        output.rgb888.len()
                    );
                },
            )?;
            tracer.trace()?;
        }
    } else {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Wait);
        let mut app = App::new(viewport, get_build_info().clone());
        event_loop.run_app(&mut app)?;
    }

    Ok(())
}
