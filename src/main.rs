use crate::assets::AssetManager;
use crate::config::TracerConfig;
use crate::front::headless::headless_tracer;
use crate::front::windowed::TracerApp;
use crate::logging::setup_logging;
use clap::builder::PossibleValuesParser;
use clap::Parser;
use glam::UVec2;
use log::{info, LevelFilter};
use winit::event_loop::{ControlFlow, EventLoop};

mod assets;
mod back;
mod common;
mod config;
mod fps;
mod front;
mod logging;
mod tracer;

build_info::build_info!(pub fn get_build_info);

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Arguments {
    #[clap(short = 'l', long, help = "Set the log level", value_parser = PossibleValuesParser::new(["error", "warn", "info", "debug", "trace"]))]
    log_level: Option<String>,
    #[clap(long, help = "Disable color output")]
    no_color: bool,

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
    let args = Arguments::parse();

    let log_level = match args.log_level.as_deref() {
        Some("error") => LevelFilter::Error,
        Some("warn") => LevelFilter::Warn,
        Some("info") => LevelFilter::Info,
        Some("debug") => LevelFilter::Debug,
        Some("trace") => LevelFilter::Trace,
        _ => LevelFilter::Debug,
    };
    setup_logging(log_level, None, !args.no_color);

    info!("Starting application with args: {:?}", args);

    let config = if args.config.is_some() {
        let config_path = args.config.as_ref().unwrap();
        info!("Loading config from file: {}", config_path);
        serde_json::from_str(&std::fs::read_to_string(config_path)?)?
    } else {
        info!("No config file provided, using default config");
        TracerConfig::default()
    };

    let asset_manager = AssetManager::new_from_pwd(&std::env::current_dir()?)?;

    let viewport = UVec2::new(args.width, args.height);
    if args.headless {
        unsafe {
            let mut tracer = headless_tracer(
                config,
                asset_manager,
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
            tracer.trace(None)?;
        }
    } else {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Wait);
        let mut app = TracerApp::new(config, asset_manager, viewport, get_build_info().clone());
        event_loop.run_app(&mut app)?;
    }

    Ok(())
}
