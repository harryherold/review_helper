use std::{ffi::OsStr, path::Path};

use slint::SharedString;

use syntect::highlighting::FontStyle;
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

fn escape_for_styled_text(text: &str) -> String {
    // 1. escape hmtl special symbols
    let html_escaped = html_escape::encode_text(text);

    // 2. escape markdown special symbols
    html_escaped
        .replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('`', "\\`")
        .replace('[', "\\[")
        .replace(']', "\\]")
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
        let max_chars_per_line = unformatted_lines.iter().map(|line| line.line.len()).max().unwrap_or_default();
        unformatted_lines
            .iter()
            .map(|diff| {
                let html_line = if let Some(hightlight_lines) = hightlight_lines_opt.as_mut() {
                    match hightlight_lines.highlight_line(&diff.line, &self.config.syntax_set) {
                        Ok(regions) => {
                            let mut result = String::with_capacity(diff.line.len() * 2);

                            for (style, text) in regions {
                                if text.is_empty() {
                                    continue;
                                }

                                let color = color_to_hex(style.foreground);
                                let escaped = escape_for_styled_text(text);

                                result.push_str(&format!("<font color=\"{}\">", color));

                                // Optional: Bold / Italic / Underline mitnehmen
                                let bold = style.font_style.contains(FontStyle::BOLD);
                                let italic = style.font_style.contains(FontStyle::ITALIC);
                                let underline = style.font_style.contains(FontStyle::UNDERLINE);

                                if bold {
                                    result.push_str("**");
                                }
                                if italic {
                                    result.push('*');
                                }
                                if underline {
                                    result.push_str("<u>");
                                }

                                result.push_str(&escaped);

                                if underline {
                                    result.push_str("</u>");
                                }
                                if italic {
                                    result.push('*');
                                }
                                if bold {
                                    result.push_str("**");
                                }

                                result.push_str("</font>");
                            }
                            result
                        }
                        Err(_) => escape_for_styled_text(&diff.line),
                    }
                } else {
                    escape_for_styled_text(&diff.line)
                };

                let styled_line = if html_line.is_empty() {
                    slint::StyledText::from_plain_text(" ")
                } else {
                    slint::StyledText::from_markdown(&html_line).unwrap_or_else(|_| slint::StyledText::from_plain_text(&diff.line))
                };

                ui::SlintDiffLine {
                    new_line_no: diff.new_line_no,
                    old_line_no: diff.old_line_no,
                    source_line: SharedString::from(&diff.line),
                    status: SlintLineStatus::from(&diff.status),
                    styled_line,
                    max_chars_count: max_chars_per_line as i32,
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>()
    }
}
