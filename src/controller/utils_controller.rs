use crate::ui;
use slint::{ComponentHandle, Model, SharedString};
use std::path::PathBuf;

use chrono::{DateTime, Local};

use regex::Regex;

fn is_vaild_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let re = Regex::new(r"^[A-Za-z][A-Za-z0-9]*$").unwrap();
    re.is_match(name)
}

pub fn setup_utils(app_window: &ui::AppWindow) {
    app_window.global::<ui::SlintStringUtils>().on_filename({
        |path| {
            if let Some(file_name) = PathBuf::from(path.to_string()).file_name() {
                file_name.to_str().expect("Could not parse os string!").to_string().into()
            } else {
                "".into()
            }
        }
    });
    app_window
        .global::<ui::SlintStringUtils>()
        .on_is_valid_name(|name| is_vaild_name(name.as_str()));
    app_window.global::<ui::SlintModelUtils>().on_index_of_string({
        |model, value| match model.iter().position(|v| value == v) {
            None => -1,
            Some(i) => i as i32,
        }
    });
    app_window.global::<ui::SlintStringUtils>().on_format_datetime({
        |date_time_string| -> SharedString {
            let date_time: DateTime<Local> = date_time_string.to_string().parse().expect("Could not parse date time string!");
            SharedString::from(format!("{}", date_time.format("%d/%m/%Y %H:%M:%S")))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_names_check() {
        assert!(is_vaild_name("Foo"));
        assert!(is_vaild_name("Bar"));
        assert!(is_vaild_name("Foo123"));
        assert!(is_vaild_name("Foo12Bar"));

        assert!(false == is_vaild_name("12Foo"));
        assert!(false == is_vaild_name("Bar*Blubb"));
        assert!(false == is_vaild_name("Foo123__"));
        assert!(false == is_vaild_name("_Foo12Bar"));
    }
}
