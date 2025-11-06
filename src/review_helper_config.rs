use std::{fs, path::PathBuf, rc::Rc};

use slint::{SharedString, VecModel};

use serde_derive::{Deserialize, Serialize};

extern crate dirs;

const REVIEW_HELPER_CONFIG_FILENAME: &'static str = "review_helper_config.toml";

type DiffToolModel = Rc<VecModel<SharedString>>;

#[derive(Serialize, Deserialize)]
pub struct ReviewHelperConfig {
    pub diff_tool: String,
    pub editor: String,
    pub editor_args: Vec<String>,
    pub color_scheme: String,
    #[serde(skip)]
    path: PathBuf,
    #[serde(skip)]
    pub diff_tool_model: DiffToolModel,
}

impl Default for ReviewHelperConfig {
    fn default() -> Self {
        Self {
            diff_tool: "meld".to_string(),
            editor: "code".to_string(),
            editor_args: vec!["-n".to_string(), "{file}".to_string()],
            color_scheme: "Dark".to_string(),
            path: PathBuf::new(),
            diff_tool_model: Rc::new(VecModel::default()),
        }
    }
}

impl ReviewHelperConfig {
    pub fn new(mut path: PathBuf) -> anyhow::Result<Self> {
        let mut review_helper_config = ReviewHelperConfig::default();

        path.push(REVIEW_HELPER_CONFIG_FILENAME);

        review_helper_config.path = path.clone();

        if path.exists() && path.is_file() {
            let file_content = fs::read_to_string(&path).map_err(|e| anyhow::format_err!("Could not read app config: {}", e.to_string()))?;
            review_helper_config =
                toml::from_str(&file_content).map_err(|e| anyhow::format_err!("Could not convert file content to toml: {}", e.to_string()))?;
        }
        Ok(review_helper_config)
    }
    pub fn save(&self) -> anyhow::Result<()> {
        if self.path.as_os_str().is_empty() {
            return Err(anyhow::format_err!("path is not valid!"));
        }

        let parent_dir = self.path.parent().expect("path has no parent dir!");
        if !parent_dir.exists() {
            fs::create_dir(parent_dir).map_err(|e| anyhow::format_err!("Could not create app config dir: {}", e.to_string()))?;
        }

        let contents = toml::to_string(self).expect("Could not convert ReviewHelperConfig struct to toml string!");
        fs::write(&self.path, contents).map_err(|e| anyhow::format_err!("Could not write app config file: {}", e.to_string()))
    }
    pub fn set_diff_tools(&mut self, diff_tools: &Vec<String>) {
        self.diff_tool_model
            .set_vec(diff_tools.iter().map(|s| SharedString::from(s)).collect::<Vec<_>>());
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    use super::ReviewHelperConfig;

    struct TestContext {
        review_helper_config: ReviewHelperConfig,
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

        let review_helper_config = ReviewHelperConfig::new(path.clone());
        assert!(review_helper_config.is_ok());
        TestContext {
            review_helper_config: review_helper_config.unwrap(),
            path,
            is_clean_enabled,
        }
    }

    #[test]
    fn test_new_config() {
        {
            let mut ctx = setup(false);
            assert_eq!(ctx.review_helper_config.diff_tool, "meld");

            ctx.review_helper_config.diff_tool = "vscode".to_string();
            assert!(ctx.review_helper_config.save().is_ok());
        }
        {
            let ctx = setup(true);
            assert_eq!(ctx.review_helper_config.diff_tool, "vscode");
        }
    }
}
