use std::{cell::RefCell, rc::Rc};

use review_todo::{AppWindow, Review};
use slint::ComponentHandle;

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    let review = Rc::new(RefCell::new(Review::new()));

    ui.on_review_todo_added({
        let review = review.clone();
        move |text| review.borrow().add_todo(text)
    });
    ui.on_review_todo_text_changed({
        let review = review.clone();
        move |todo_index, text| review.borrow().set_todo_text(todo_index as usize, text)
    });
    ui.on_review_todo_is_fixed_toggled({
        let review = review.clone();
        move |todo_index| review.borrow().toogle_is_fixed(todo_index as usize)
    });
    ui.on_review_todos_save_requested({
        let review = review.clone();
        let ui_weak = ui.as_weak();
        move || {
            if let Some(path) = review.borrow_mut().save_todos() {
                let ui = ui_weak.unwrap();
                ui.set_current_file(path);
            }
        }
    });
    ui.on_review_todos_open_requested({
        let review = review.clone();
        let ui_weak = ui.as_weak();
        move || {
            let ui = ui_weak.unwrap();
            if let Some(path) = review.borrow_mut().open_todos() {
                ui.set_current_file(path);
            }
        }
    });
    ui.on_review_open_repo_requested({
        let review = review.clone();
        let ui_weak = ui.as_weak();
        move || {
            let ui = ui_weak.unwrap();
            if let Some(path) = review.borrow_mut().open_repo() {
                ui.set_current_repo(path);
            }
        }
    });
    ui.on_review_diff_requested({
        let review = review.clone();
        move |start_commit, end_commit| review.borrow_mut().diff_repo(start_commit, end_commit)
    });
    ui.on_review_open_diff_requested({
        let review = review.clone();
        move |index| review.borrow().diff_file(index)
    });
    ui.set_review_todo_item_model(review.borrow().todo_model().into());
    ui.set_review_file_item_model(review.borrow().file_diff_model().into());
    ui.run()
}
