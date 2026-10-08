use slint::{Model, ModelNotify, ModelTracker};
use std::cell::RefCell;

pub struct SliceProxyModel<M: Model> {
    source: M,
    start: RefCell<usize>,
    count: RefCell<usize>,
    notify: ModelNotify,
}

impl<M: Model> SliceProxyModel<M> {
    pub fn new(source: M) -> Self {
        Self {
            source,
            start: RefCell::new(0),
            count: RefCell::new(0),
            notify: ModelNotify::default(),
        }
    }

    pub fn set_slice(&self, start: usize, count: usize) {
        let max_len = self.source.row_count();
        let valid_start = start.min(max_len);
        let valid_count = count.min(max_len - valid_start);

        let mut current_start = self.start.borrow_mut();
        let mut current_count = self.count.borrow_mut();

        if *current_start != valid_start || *current_count != valid_count {
            *current_start = valid_start;
            *current_count = valid_count;
            self.notify.reset();
        }
    }

    pub fn source(&self) -> &M {
        &self.source
    }
}

impl<M: Model + 'static> Model for SliceProxyModel<M> {
    type Data = M::Data;

    fn row_count(&self) -> usize {
        *self.count.borrow()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        let start = *self.start.borrow();
        let count = *self.count.borrow();

        if row < count { self.source.row_data(start + row) } else { None }
    }

    fn set_row_data(&self, row: usize, data: Self::Data) {
        let start = *self.start.borrow();
        let count = *self.count.borrow();
        if row < count {
            self.source.set_row_data(start + row, data);
            self.notify.row_changed(row);
        }
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &self.notify
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}
