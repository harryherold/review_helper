use std::{cell::RefCell, rc::Rc};

use slint::{FilterModel, Model, ModelExt, ModelNotify, ModelPeer, VecModel};

use crate::ui::DiffFileItem;

struct IdModelItem {
    value: String,
    id: u32,
}

struct IdModel {
    data: std::cell::RefCell<Vec<String>>,
    notify: ModelNotify,
}

impl Model for IdModel {
    type Data = String;

    fn row_count(&self) -> usize {
        self.data.borrow().len()
    }
    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        &self.notify
    }
    fn row_data(&self, row: usize) -> Option<Self::Data> {
        self.data.borrow().get(row).cloned()
    }
    fn set_row_data(&self, row: usize, data: Self::Data) {
        self.data.borrow_mut()[row] = data;
        self.notify.row_changed(row);
    }
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}

impl IdModel {
    fn new() -> IdModel {
        let m = IdModel {
            data: RefCell::new(Vec::new()),
            notify: ModelNotify::default(),
        };
        // m.model_tracker().attach_peer(peer);
        m
    }
    fn push(&mut self, value: String) {
        self.data.borrow_mut().push(value);
        self.notify.row_added(self.data.borrow().len() - 1, 1);
    }
}

struct Names {
    data: FilterModel<VecModel<String>, fn(&String) -> bool>,
}

fn filter_names(name: &String) -> bool {
    name.starts_with("a")
}

impl Names {
    fn filter_callback(name: &String) -> bool {
        name.contains("42")
    }
    fn new() -> Names {
        Names {
            data: VecModel::default().filter(Names::filter_callback),
        }
    }
    // fn data(&self) -> &VecModel<String> {
    //     &self.data
    // }
}

struct FooCallback<F> {
    handler: F,
}

impl<F> FooCallback<F>
where
    F: Fn(&str) -> bool + 'static,
{
    fn new(func: F) -> Self {
        FooCallback { handler: func }
    }
    fn call(&self) -> bool {
        (self.handler)("foo")
    }
}

struct Bar {
    callback: FooCallback<fn(&str) -> bool>,
}

impl Bar {
    fn new() -> Self {
        Bar {
            callback: FooCallback::new(Bar::my_call),
        }
    }
    fn exec(&self) -> bool {
        self.callback.call()
    }
    fn my_call(name: &str) -> bool {
        name.contains("a")
    }
}

// impl PartialOrd for DiffFileItem {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         self.text.partial_cmp(&other.text)
//     }
// }

// impl PartialEq for DiffFileItem {
//     fn eq(&self, other: &Self) -> bool {
//         self.text == other.text
//     }
// }

// impl Eq for DiffFileItem {}

// impl Ord for DiffFileItem {
//     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//         self.text.cmp(&other.text)
//     }
// }

#[cfg(test)]
mod test {
    use core::borrow;
    use std::{cell::RefCell, default, rc::Rc};

    use slint::{FilterModel, Model, ModelExt, SortModel, VecModel};

    use super::{Bar, FooCallback, Names};

    fn callback(name: &str) -> bool {
        name.contains("a")
    }

    #[test]
    fn test_difffile_proxy_models() {}

    #[test]
    fn create_rows() {
        let filter_name = Rc::new(RefCell::new("42".to_string()));

        let model = Rc::new(VecModel::<String>::default());
        let f = filter_name.clone();

        let filter_model = Rc::new(FilterModel::new(model.clone(), move |s| {
            let f = f.borrow();
            s.contains(f.as_str())
        }));
        let mut sort_model = Rc::new(SortModel::new_ascending(model.clone()));

        model.push("foo 42".to_string());
        model.push("bar 42".to_string());
        model.push("zoo 42 #".to_string());
        model.push("couch 42 #".to_string());

        model.push("alarm".to_string());

        for name in sort_model.iter() {
            println!("{}", name);
        }

        println!("");

        // assert_eq!(sort_model.row_data(0), Some("bar 42".to_string()));
        // assert_eq!(sort_model.row_data(1), Some("couch 42 #".to_string()));
        // assert_eq!(sort_model.row_data(2), Some("foo 42".to_string()));
        // assert_eq!(sort_model.row_data(3), Some("zoo 42 #".to_string()));

        {
            let mut f = filter_name.borrow_mut();
            *f = "42 #".to_string();
        }

        filter_model.reset();

        for name in sort_model.iter() {
            println!("{}", name);
        }

        sort_model.reset();

        // assert_eq!(sort_model.row_data(0), Some("couch 42 #".to_string()));
        // assert_eq!(sort_model.row_data(1), Some("zoo 42 #".to_string()));
    }
}
