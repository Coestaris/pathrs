// Vulkan makes heavy use of complex types and many function arguments,
// so we disable some clippy warnings globally for this project.
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

use crate::assets::AssetManager;
use crate::config::TracerConfig;
use crate::front::headless::headless_tracer;
use crate::front::windowed::TracerApp;
use crate::logging::setup_logging;
use clap::builder::PossibleValuesParser;
use clap::Parser;
use glam::UVec2;
use image::{ImageBuffer, Rgb};
use log::{info, warn, LevelFilter};
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
    #[clap(
        short = 'l',
        long,
        help = "Set the log level",
        value_parser = PossibleValuesParser::new(["error", "warn", "info", "debug", "trace"])
    )]
    log_level: Option<String>,

    #[clap(
        long,
        help = "Disable color output"
    )]
    no_color: bool,

    #[clap(
        short = 'x',
        long,
        default_value_t = 1280,
        help = "Width of the default viewport in pixels"
    )]
    width: u32,

    #[clap(
        short = 'y',
        long,
        default_value_t = 720,
        help = "Height of the default viewport in pixels"
    )]
    height: u32,

    #[clap(
        short = 'd',
        long,
        help = "If set, run the tracer in headless mode, outputting the specified path as a PNG image"
    )]
    headless: Option<String>,

    #[clap(
        short = 'c',
        long,
        help = "Path to the config file in JSON format"
    )]
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
    if let Some(path) = args.headless {
        let path = std::path::PathBuf::from(path);
        if path.extension() != Some(std::ffi::OsStr::new("png")) {
            warn!("Headless output path does not have a .png extension, the output image will still be saved as a PNG file");
        }

        unsafe {
            let mut tracer = headless_tracer(
                config,
                asset_manager,
                viewport,
                get_build_info().clone(),
                move |output| {
                    info!(
                        "Received headless output: {}x{}, {} bytes",
                        output.width,
                        output.height,
                        output.rgb888.len()
                    );

                    let image: ImageBuffer<Rgb<u8>, _> =
                        ImageBuffer::from_raw(output.width, output.height, output.rgb888).unwrap();
                    image.save(&path).unwrap();
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
