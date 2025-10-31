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
    #[clap(
        short = 'd',
        long,
        help = "Run in headless mode. No window will be created"
    )]
    headless: bool,
    #[clap(short = 'c', long, help = "Path to the config file in JSON format")]
    config: Option<String>,
}

fn main() -> anyhow::Result<()> {
    setup_logging(LevelFilter::Debug, None, true);

    let args = Arguments::parse();
    info!("Starting application with args: {:?}", args);

    let config = if args.config.is_some() {
        let config_path = args.config.as_ref().unwrap();
        info!("Loading config from file: {}", config_path);
        serde_json::from_str(&std::fs::read_to_string(config_path)?)?
    } else {
        info!("No config file provided, using default config");
        TracerConfig::default()
    };

    let viewport = UVec2::new(args.width, args.height);
    if args.headless {
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
        let mut app = App::new(config, viewport, get_build_info().clone());
        event_loop.run_app(&mut app)?;
    }

    Ok(())
}
