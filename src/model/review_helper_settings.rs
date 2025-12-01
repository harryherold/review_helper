use std::{fs, path::PathBuf};

use serde_derive::{Deserialize, Serialize};

extern crate dirs;

const REVIEW_HELPER_CONFIG_FILENAME: &'static str = "review_helper_settings.toml";

#[derive(Serialize, Deserialize)]
pub struct ReviewHelperSettings {
    pub diff_tool: String,
    pub editor: String,
    pub editor_args: Vec<String>,
    pub color_scheme: String,
    #[serde(skip)]
    path: PathBuf,
}

impl Default for ReviewHelperSettings {
    fn default() -> Self {
        Self {
            diff_tool: "meld".to_string(),
            editor: "code".to_string(),
            editor_args: vec!["-n".to_string(), "{file}".to_string()],
            color_scheme: "Dark".to_string(),
            path: PathBuf::new(),
        }
    }
}

impl ReviewHelperSettings {
    pub fn new(mut path: PathBuf) -> anyhow::Result<Self> {
        let mut review_helper_settings = ReviewHelperSettings::default();

        path.push(REVIEW_HELPER_CONFIG_FILENAME);

        review_helper_settings.path = path.clone();

        if path.exists() && path.is_file() {
            let file_content = fs::read_to_string(&path).map_err(|e| anyhow::format_err!("Could not read app config: {}", e.to_string()))?;
            review_helper_settings =
                toml::from_str(&file_content).map_err(|e| anyhow::format_err!("Could not convert file content to toml: {}", e.to_string()))?;
        }
        review_helper_settings.path = path.clone();
        Ok(review_helper_settings)
    }
    pub fn save(&self) -> anyhow::Result<()> {
        if self.path.as_os_str().is_empty() {
            return Err(anyhow::format_err!("path is not valid!"));
        }

        let parent_dir = self.path.parent().expect("path has no parent dir!");
        if !parent_dir.exists() {
            fs::create_dir(parent_dir).map_err(|e| anyhow::format_err!("Could not create app config dir: {}", e.to_string()))?;
        }

        let contents = toml::to_string(self).expect("Could not convert ReviewHelperSettings struct to toml string!");
        fs::write(&self.path, contents).map_err(|e| anyhow::format_err!("Could not write app config file: {}", e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    use super::ReviewHelperSettings;

    struct TestContext {
        review_helper_settings: ReviewHelperSettings,
        path: PathBuf,
        is_clean_enabled: bool,
    }

    impl Drop for TestContext {
        fn drop(&mut self) {
            if self.is_clean_enabled {
                let _ = fs::remove_dir_all(self.path.clone());
            }
        }
    }

    fn setup(is_clean_enabled: bool) -> TestContext {
        let mut path = env::temp_dir();
        path.push(std::env!("CARGO_CRATE_NAME"));

        if !path.exists() {
            let result = fs::create_dir(&path);
            assert!(result.is_ok());
        }

        let review_helper_settings = ReviewHelperSettings::new(path.clone());
        assert!(review_helper_settings.is_ok());
        TestContext {
            review_helper_settings: review_helper_settings.unwrap(),
            path,
            is_clean_enabled,
        }
    }

    #[test]
    fn test_new_config() {
        {
            let mut ctx = setup(false);
            assert_eq!(ctx.review_helper_settings.diff_tool, "meld");

            ctx.review_helper_settings.diff_tool = "vscode".to_string();
            assert!(ctx.review_helper_settings.save().is_ok());
        }
        {
            let ctx = setup(true);
            assert_eq!(ctx.review_helper_settings.diff_tool, "vscode");
        }
    }
}
