use crate::get_build_info;
use build_info::{BuildInfo, VersionControl};
use log::{info, Level, LevelFilter};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Instant, SystemTime};
use tracing::span::{Attributes, Record};
use tracing::{Event, Id, Metadata, Subscriber};

fn format_system_time(system_time: SystemTime) -> Option<String> {
    let datetime: chrono::DateTime<chrono::Utc> = system_time.into();
    Some(datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string())
}

fn print_build_info(bi: &BuildInfo) {
    info!(r"$$$$$$$\   $$$$$$\ $$$$$$$$\ $$\   $$\ $$$$$$$\   $$$$$$\");
    info!(r"$$  __$$\ $$  __$$\\__$$  __|$$ |  $$ |$$  __$$\ $$  __$$\");
    info!(r"$$ |  $$ |$$ /  $$ |  $$ |   $$ |  $$ |$$ |  $$ |$$ /  \__|");
    info!(r"$$$$$$$  |$$$$$$$$ |  $$ |   $$$$$$$$ |$$$$$$$  |\$$$$$$\");
    info!(r"$$  ____/ $$  __$$ |  $$ |   $$  __$$ |$$  __$$<  \____$$\");
    info!(r"$$ |      $$ |  $$ |  $$ |   $$ |  $$ |$$ |  $$ |$$\   $$ |");
    info!(r"$$ |      $$ |  $$ |  $$ |   $$ |  $$ |$$ |  $$ |\$$$$$$  |");
    info!(r"\__|      \__|  \__|  \__|   \__|  \__|\__|  \__| \______/");

    info!(
        "Current time: {}",
        format_system_time(SystemTime::now()).unwrap()
    );
    info!("Build Information:");
    info!("  Version: {}", bi.crate_info.version);
    info!("  Features: {:?}", bi.crate_info.enabled_features);
    info!("  Timestamp: {}", bi.timestamp);
    info!("  Profile: {}", bi.profile);
    info!("  Optimizations: {}", bi.optimization_level);
    info!("  Target: {}", bi.target);
    info!("  Compiler: {}", bi.compiler);
    if let Some(VersionControl::Git(git)) = &bi.version_control {
        info!("  VCS (Git) Information:");
        info!("    Commit: {} ({})", git.commit_id, git.commit_timestamp);
        info!("    Is dirty: {}", git.dirty);
        info!("    Refs: {:?}, {:?}", git.branch, git.tags);
    }
}

// Store the start time of the application
// Used for logging elapsed time
static START_TIME: OnceLock<Instant> = OnceLock::new();

fn format_inner<'a, F, const COLORED: bool>(
    message: &'a fmt::Arguments<'a>,
    record: &'a log::Record<'a>,
    callback: F,
) where
    F: FnOnce(fmt::Arguments),
{
    let red: &'static str = if COLORED { "\x1B[31m" } else { "" }; // Red
    let yellow: &'static str = if COLORED { "\x1B[33m" } else { "" }; // Yellow
    let green: &'static str = if COLORED { "\x1B[32m" } else { "" }; // Green
    let blue: &'static str = if COLORED { "\x1B[34m" } else { "" }; // Blue
    let magenta: &'static str = if COLORED { "\x1B[35m" } else { "" }; // Magenta
    let cyan: &'static str = if COLORED { "\x1B[36m" } else { "" }; // Cyan
    let white: &'static str = if COLORED { "\x1B[37m" } else { "" }; // White
    let reset: &'static str = if COLORED { "\x1B[0m" } else { "" }; // Reset

    let elapsed = START_TIME
        .get()
        .map(|start| start.elapsed())
        .unwrap_or_default();

    // Keep only the file name, not the full path since that can be very long
    // and filename is really additional info anyway
    let file = Path::new(record.file().unwrap_or("unknown"));
    let base = file.file_name().unwrap_or_default().to_string_lossy();
    let location = format!("{}:{}", base, record.line().unwrap_or(0));

    callback(format_args!(
        "[{cyan}{:^10.3}{reset}][{magenta}{:^25}{reset}][{yellow}{:^10}{reset}][{}{:>5}{reset}]: {}",
        elapsed.as_secs_f32(),
        location,
        std::thread::current().name().unwrap_or("main"),
        match record.level() {
            Level::Error => red,
            Level::Warn => yellow,
            Level::Info => green,
            Level::Debug => blue,
            Level::Trace => white,
        },
        record.level(),
        message,
    ))
}

fn format<'a, F>(message: &'a fmt::Arguments<'a>, record: &'a log::Record<'a>, callback: F)
where
    F: FnOnce(fmt::Arguments),
{
    format_inner::<F, false>(message, record, callback);
}

fn format_colored<'a, F>(message: &'a fmt::Arguments<'a>, record: &'a log::Record<'a>, callback: F)
where
    F: FnOnce(fmt::Arguments),
{
    format_inner::<F, true>(message, record, callback);
}

struct TracerSubscriber;

impl Subscriber for TracerSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        // Disable tracing for now since it's too verbose and not really useful
        false
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        unreachable!()
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {
        unreachable!()
    }

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {
        unreachable!()
    }

    fn event(&self, _event: &Event<'_>) {
        unreachable!()
    }

    fn enter(&self, _span: &Id) {
        unreachable!()
    }

    fn exit(&self, _span: &Id) {
        unreachable!()
    }
}

pub fn setup_logging(level: LevelFilter, file_logging: Option<PathBuf>, colored: bool) {
    START_TIME.set(Instant::now()).ok();

    tracing::subscriber::set_global_default(TracerSubscriber).ok();

    let mut dispatch = fern::Dispatch::new().level(level).chain(std::io::stdout());

    if colored {
        dispatch = dispatch.format(|cb, args, r| format_colored(args, r, |fmt| cb.finish(fmt)));
    } else {
        dispatch = dispatch.format(|cb, args, r| format(args, r, |fmt| cb.finish(fmt)));
    }

    if let Some(path) = file_logging {
        dispatch = dispatch.chain(fern::log_file(path).unwrap());
    }

    dispatch.apply().unwrap();

    let build_info = get_build_info();
    print_build_info(&build_info);
}
