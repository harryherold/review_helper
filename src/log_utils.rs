use simplelog::*;

use std::fs::File;

pub fn init_logger() -> anyhow::Result<()> {
    let config = ConfigBuilder::new().set_location_level(LevelFilter::Info).build();

    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Info, config.clone(), TerminalMode::Mixed, ColorChoice::Auto),
        WriteLogger::new(LevelFilter::Info, config, File::create("review_helper.log").expect("Could not create log file")),
    ])?;

    std::panic::set_hook(Box::new(|panic_info| {
        log::error!("Internal panic occurred: {}", panic_info);
        let _ = native_dialog::MessageDialog::new()
            .set_title("Internal Error")
            .set_text(&format!("An internal error occurred. Please check the log file for details.\n\n{}", panic_info))
            .set_type(native_dialog::MessageType::Error)
            .show_alert();
        std::process::exit(1);
    }));

    Ok(())
}

#[macro_export]
macro_rules! check_result {
    ($result:expr, $msg:literal) => {
        if let Err(e) = $result {
            ::log::error!("{}: {}", $msg, &e.to_string());
        }
    };
}

#[macro_export]
macro_rules! unwrap_or_return {
    ($result:expr, $msg:literal) => {
        match $result {
            Some(r) => r,
            None => {
                ::log::error!("{}", $msg);
                return;
            }
        }
    };
    ($result:expr, $msg:literal, $error_return:expr) => {
        match $result {
            Some(r) => r,
            None => {
                ::log::error!("{}", $msg);
                return $error_return;
            }
        }
    };
}
