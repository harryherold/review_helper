// use slint::{FilterModel, Model, SortModel};
use std::rc::Rc;

use slint::Model;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let todo_model = Rc::new(slint::VecModel::<ReviewTodoItem>::from(vec![
        ReviewTodoItem { isFixed: true, text: "Implement the .slint file".into() },
        ReviewTodoItem { isFixed: false, text: "Do the Rust part".into() },
    ]));

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

    ui.set_review_todo_item_model(todo_model.into());

    ui.run()
}
