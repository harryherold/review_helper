use std::{path::PathBuf, process::Command};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
pub fn run_command(command: &str, args: &Vec<String>, cwd: &PathBuf) -> anyhow::Result<()> {
    Command::new(command).current_dir(cwd).args(args).creation_flags(0x08000000).spawn().map_err(|e| { anyhow::format_err!("Error running command: {}, e.g. {}", command, e) })?;
    Ok(())
}

#[cfg(not(windows))]
pub fn run_command(command: &str, args: &Vec<String>, cwd: &PathBuf) -> Result<String, String> {
    Command::new(command).current_dir(cwd).args(args).spawn().map_err(|e| { anyhow::format_err!("Error running command: {}, e.g. {}", command, e) })?;
    Ok(())
}