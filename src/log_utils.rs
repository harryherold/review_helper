use simplelog::*;

use std::fs::File;

pub fn init_logger() {
    let config = ConfigBuilder::new().set_location_level(LevelFilter::Info).build();

    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Info, config.clone(), TerminalMode::Mixed, ColorChoice::Auto),
        WriteLogger::new(LevelFilter::Info, config, File::create("review_helper.log").expect("Could not create log file")),
    ])
    .unwrap();
}
