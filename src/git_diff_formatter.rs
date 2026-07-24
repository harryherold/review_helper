use std::{ffi::OsStr, path::Path};

use slint::SharedString;

use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet};

use crate::git_repo::{GitDiffLine, LineType};
use crate::ui::{self, SlintLineStatus};

struct HighLighterConfig {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme: String,
}

impl HighLighterConfig {
    fn new(theme: &str) -> Self {
        HighLighterConfig {
            syntax_set: SyntaxSet::load_defaults_nonewlines(),
            theme_set: ThemeSet::load_defaults(),
            theme: theme.to_owned(),
        }
    }
}

impl From<&LineType> for ui::SlintLineStatus {
    fn from(value: &LineType) -> Self {
        match value {
            LineType::Added => ui::SlintLineStatus::Added,
            LineType::Removed => ui::SlintLineStatus::Removed,
            LineType::Unchanged => ui::SlintLineStatus::Unchanged,
        }
    }
}

fn extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension().and_then(OsStr::to_str)
}

fn color_to_hex(c: syntect::highlighting::Color) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

pub struct GitDiffFormatter {
    config: HighLighterConfig,
}

impl GitDiffFormatter {
    pub fn new(theme: &str) -> Self {
        GitDiffFormatter {
            config: HighLighterConfig::new(theme),
        }
    }
    pub fn format_lines(&self, unformatted_lines: Vec<GitDiffLine>, file: &str) -> Vec<ui::SlintDiffLine> {
        let extension = extension_from_filename(file).unwrap_or_default();
        let syntax_opt = self.config.syntax_set.find_syntax_by_extension(extension);
        let mut hightlight_lines_opt = syntax_opt.map(|syntax| HighlightLines::new(syntax, &self.config.theme_set.themes[&self.config.theme]));

        let x = unformatted_lines
            .iter()
            .map(|diff| {
                let html_line = if let Some(hightlight_lines) = hightlight_lines_opt.as_mut() {
                    match hightlight_lines.highlight_line(&diff.line, &self.config.syntax_set) {
                        Ok(regions) => regions
                            .into_iter()
                            .map(|(style, text)| {
                                let escaped_text = html_escape::encode_text(text);
                                format!(r#"<font color="{}">{}</font>"#, color_to_hex(style.foreground), escaped_text,)
                            })
                            .collect::<Vec<_>>()
                            .join(""),
                        Err(_) => diff.line.clone(),
                    }
                } else {
                    diff.line.clone()
                };

                ui::SlintDiffLine {
                    new_line_no: diff.new_line_no,
                    old_line_no: diff.old_line_no,
                    source_line: SharedString::from(&diff.line),
                    status: SlintLineStatus::from(&diff.status),
                    styled_line: slint::StyledText::from_markdown(&html_line).unwrap_or_default(),
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();

        x.iter().for_each(|u| {
            println!("{}", &u.styled_line);
        });
        x
    }
}
