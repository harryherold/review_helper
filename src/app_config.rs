use std::{fs, path::PathBuf};

use serde_derive::{Deserialize, Serialize};

extern crate dirs;

const APP_CONFIG_FILENAME: &'static str = "app_config.toml";

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    pub diff_tool: String,
}

pub struct AppConfig {
    config: Config,
    path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self { diff_tool: "meld".to_string() }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            config: Config::default(),
            path: PathBuf::new(),
        }
    }
}

pub fn config_dir_path() -> PathBuf {
    let mut path = dirs::data_local_dir().expect("Could not find OS specific dirs!");
    let app_name = std::env!("CARGO_CRATE_NAME");
    path.push(app_name);
    path
}

impl AppConfig {
    pub fn new(mut path: PathBuf) -> anyhow::Result<Self> {
        path.push(APP_CONFIG_FILENAME);

        if path.exists() && path.is_file() {
            let file_content = fs::read_to_string(&path).map_err(|e| anyhow::format_err!("Could not read app config: {}", e.to_string()))?;
            let config: Config = toml::from_str(&file_content).map_err(|e| anyhow::format_err!("Could not convert file content to toml: {}", e.to_string()))?;
            Ok(AppConfig { config, path })
        } else {
            Ok(AppConfig {
                config: Config::default(),
                path,
            })
        }
    }
    pub fn save(&self) -> anyhow::Result<()> {
        if self.path.as_os_str().is_empty() {
            return Err(anyhow::format_err!("path is not valid!"));
        }

        let parent_dir = self.path.parent().expect("path has no parent dir!");
        if !parent_dir.exists() {
            fs::create_dir(parent_dir).map_err(|e| anyhow::format_err!("Could not create app config dir: {}", e.to_string()))?;
        }

        let contents = toml::to_string(&self.config).expect("Could not convert AppConfig struct to toml string!");
        fs::write(&self.path, contents).map_err(|e| anyhow::format_err!("Could not write app config file: {}", e.to_string()))
    }
    pub fn diff_tool(&self) -> &str {
        &self.config.diff_tool
    }
    pub fn set_diff_tool(&mut self, new_diff_tool: String) {
        self.config.diff_tool = new_diff_tool;
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    use super::AppConfig;

    struct TestContext {
        app_config: AppConfig,
        path: PathBuf,
        is_clean_enabled: bool,
    }

    impl Drop for TestContext {
        fn drop(&mut self) {
            if self.is_clean_enabled {
                println!("remove");
                let result = fs::remove_dir_all(&self.path);
                assert!(result.is_ok());
            }
        }
    }

    fn setup(is_clean_enabled: bool) -> TestContext {
        let path = test_dir_path();
        let app_config = AppConfig::new(path.clone());
        assert!(app_config.is_ok());
        TestContext {
            app_config: app_config.unwrap(),
            path,
            is_clean_enabled,
        }
    }

    fn test_dir_path() -> PathBuf {
        let mut path = env::temp_dir();
        let app_name = std::env!("CARGO_CRATE_NAME");
        path.push(app_name);
        path
    }

    #[test]
    fn test_new_config() {
        {
            let mut ctx = setup(false);
            assert_eq!(ctx.app_config.diff_tool(), "meld");

            ctx.app_config.set_diff_tool("vscode".to_string());
            assert!(ctx.app_config.save().is_ok());
        }
        {
            let ctx = setup(true);
            assert_eq!(ctx.app_config.diff_tool(), "vscode");
        }
    }
}
