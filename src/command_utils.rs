use std::{path::Path, process::Command};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn run_command(command: &str, args: &[String], cwd: &Path) -> anyhow::Result<()> {
    let mut cmd = Command::new(command);
    cmd.current_dir(cwd).args(args);

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd.spawn().map_err(|e| anyhow::format_err!("Error running command: {}, e.g. {}", command, e))?;
    Ok(())
}
