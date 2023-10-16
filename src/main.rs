use std::fs::{File, read_to_string};
use std::str::FromStr;
use std::{rc::Rc, path::PathBuf};
use std::io::{Write, Error};

use slint::{Model, VecModel};

use native_dialog::{FileDialog, MessageDialog, MessageType};

slint::include_modules!();

fn save_vec_model(vec_model: &VecModel<ReviewTodoItem>, path: PathBuf) -> Result<(), Error> {
    let mut file = File::create(path)?;
    for item in vec_model.iter() {
        let task = todo_txt::Task {
            subject: item.text.to_string(),
            finished: item.isFixed,
            ..Default::default()
        };
        write!(file, "{}\n", task.to_string())?;
    }
    Ok(())
}

// TODO create own error enum
fn open_vec_model(vec_model: &VecModel<ReviewTodoItem>, path: PathBuf) -> Result<(), Error> {
    for line in read_to_string(path)?.lines() {
        let task_result = todo_txt::Task::from_str(line);
        if let Ok(task) = task_result {
            vec_model.push(ReviewTodoItem { 
                isFixed: task.finished, text: task.subject.into()
            });
        }
    }
    Ok(())
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    let ui_weak = ui.as_weak();

    let todo_model = Rc::new(slint::VecModel::<ReviewTodoItem>::default());

    ui.on_review_todo_added({
        let todo_model = todo_model.clone();
        move | text |{
            todo_model.push(ReviewTodoItem{ isFixed: false, text: text });
        }
    });
    ui.on_review_todo_text_changed({
        let todo_model = todo_model.clone();
        move | index, text | {
            let data = todo_model.row_data_tracked(index as usize);
            if let Some(item) = data {
                if item.text != text {
                    todo_model.set_row_data(index as usize, ReviewTodoItem{
                        isFixed: item.isFixed,
                        text: text,
                    });
                }
            }
        }
    });
    ui.on_review_todo_is_fixed_toggled({
        let todo_model = todo_model.clone();
        move | index | {
            let data = todo_model.row_data_tracked(index as usize);
            if let Some(item) = data {
                todo_model.set_row_data(index as usize, ReviewTodoItem{
                    isFixed: !item.isFixed,
                    text: item.text,
                });
            }
        }
    });
    ui.on_review_todos_save_requested({
        let todo_model = todo_model.clone();
        move || {
            let save_option = FileDialog::new()
            .set_location("~")
            .add_filter("Text File (*.txt)", &["txt"])
            .show_save_single_file()
            .unwrap();
            // TODO use ui.get_current_file()
            if let Some(save_path) = save_option {
                let result = save_vec_model(&todo_model, save_path);
                if let Err(_) = result {
                    let _r = MessageDialog::new()
                        .set_type(MessageType::Error)
                        .set_title("Abort")
                        .set_text("Could save comments!")
                        .show_alert();
                }
            };
        }
    });
    ui.on_review_todos_open_requested({
        let todo_model = todo_model.clone();
        move || {
            let open_option = FileDialog::new()
                .set_location("~")
                .add_filter("Text File (*.txt)", &["txt"])
                .show_open_single_file()
                .unwrap();
            if let Some(open_path) = open_option {
                let ui = ui_weak.unwrap();
                ui.set_current_file(open_path.to_str().unwrap().into());
                open_vec_model(&todo_model, open_path);
            }
        }
    });
    
    ui.set_review_todo_item_model(todo_model.into());

    ui.run()
}
