use std::{fs, path::PathBuf};

use serde_derive::{Deserialize, Serialize};

extern crate dirs;

const APP_CONFIG_FILENAME: &'static str = "app_config.toml";

#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    pub diff_tool: String
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            diff_tool: "meld".to_string()
        }
    }
}

fn config_dir_path() -> PathBuf {
    let mut path = dirs::data_local_dir().expect("Could not find OS specific dirs!");
    let app_name = std::env!("CARGO_CRATE_NAME");
    path.push(app_name);
    path
}

impl AppConfig {
    pub fn new() -> anyhow::Result<Self> {
        let mut path = config_dir_path();
        path.push(APP_CONFIG_FILENAME);

        if path.exists() && path.is_file() {
            let file_content = fs::read_to_string(&path).map_err(|e| anyhow::format_err!("Could not read app config: {}", e.to_string()))?;
            let config: AppConfig = toml::from_str(&file_content).map_err(|e| anyhow::format_err!("Could not convert file content to toml: {}", e.to_string()))?;
            Ok(config)
        }
        else {
            Ok(AppConfig::default())
        }
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let mut path = config_dir_path();
        if !path.exists() {
            fs::create_dir(&path).map_err(|e| anyhow::format_err!("Could not create app config dir: {}", e.to_string()))?;
        }
        path.push(APP_CONFIG_FILENAME);

        let contents = toml::to_string(self).expect("Could not convert AppConfig struct to toml string!");
        fs::write(path, contents).map_err(|e| anyhow::format_err!("Could not write app config file: {}", e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::app_config::config_dir_path;

    use super::AppConfig;

    // NOTE before runinng these tests backup your app config from system!

    #[test]
    fn test_new_config() {
        let path = config_dir_path();

        if path.exists() {
            eprintln!("App config data already exists on the system! -> {:?}", path.as_os_str());
            assert!(false);
        }
        {
            let app_config = AppConfig::new();
            assert!(app_config.is_ok());

            let mut app_config = app_config.unwrap();
            assert_eq!(app_config.diff_tool, "meld");

            app_config.diff_tool = "vscode".to_string();

            assert!(app_config.save().is_ok())
        }
        {
            let app_config = AppConfig::new();
            assert!(app_config.is_ok());
            let app_config = app_config.unwrap();
            assert_eq!(app_config.diff_tool, "vscode");
        }
        {
            let result = fs::remove_dir_all(path);
            assert!(result.is_ok());
        }
    }

}
