use crate::logging::setup_logging;
use log::LevelFilter;

mod logging;

build_info::build_info!(pub fn get_build_info);

fn main() {
    setup_logging(LevelFilter::Debug, None, true);
}
