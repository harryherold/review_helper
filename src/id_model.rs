use std::{cell::RefCell, collections::BTreeMap, sync::atomic::AtomicUsize};

use slint::{Model, ModelNotify};

fn map_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as usize
}

struct IdModelItem {
    value: String,
    id: u32,
}

struct IdModel<T> {
    entity_map: std::cell::RefCell<BTreeMap<usize, T>>,
    notify: ModelNotify,
}

impl<T: Clone + 'static> Model for IdModel<T> {
    type Data = T;

    fn row_count(&self) -> usize {
        self.entity_map.borrow().len()
    }
    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        &self.notify
    }
    fn row_data(&self, row: usize) -> Option<Self::Data> {
        match self.entity_map.borrow().keys().nth(row) {
            None => None,
            Some(key) => self.entity_map.borrow().get(key).map_or(None, |s| Some(s.to_owned())),
        }
    }
    fn set_row_data(&self, row: usize, data: Self::Data) {
        if let Some(key) = self.entity_map.borrow().keys().nth(row) {
            if let Some(entry) = self.entity_map.borrow_mut().get_mut(key) {
                *entry = data;
                self.notify.row_changed(row);
            }
        }
    }
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}

impl<T: Clone> IdModel<T> {
    fn new() -> IdModel<T> {
        IdModel {
            entity_map: RefCell::new(BTreeMap::new()),
            notify: ModelNotify::default(),
        }
    }
    fn add(&mut self, id: usize, value: T) {
        self.entity_map.borrow_mut().insert(id, value);

        if let Some(index) = self.entity_map.borrow().keys().position(|&k| k == id) {
            self.notify.row_added(index, 1);
        }
    }
    fn remove(&mut self, id: usize) {
        let opt_index = self.entity_map.borrow().keys().position(|&k| k == id);
        if let Some(index) = opt_index {
            self.entity_map.borrow_mut().remove(&id);
            self.notify.row_removed(index, 1);
        }
    }
    fn get(&self, id: usize) -> Option<T> {
        self.entity_map.borrow().get(&id).cloned()
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use slint::Model;

    use super::{map_id, IdModel};

    #[test]
    fn create_query_remove() {
        let mut model = IdModel::<String>::new();
        let foo_id = map_id();
        let bar_id = map_id();
        model.add(foo_id, "foo".to_string());
        model.add(bar_id, "bar".to_string());
        model.remove(foo_id);
        assert_eq!(model.row_count(), 1);
        assert_eq!(model.row_data(0), Some("bar".to_string()));
        let baz_id = map_id();
        model.add(baz_id, "baz".to_string());
        assert_eq!(model.get(baz_id), Some("baz".to_string()));
    }

    #[test]
    fn test_btree_map() {
        let mut map = BTreeMap::<usize, String>::new();
        let get_position = |m: &BTreeMap<usize, String>, id: usize| m.keys().position(|&k| k == id).unwrap();

        map.insert(1, "A".to_string());
        assert_eq!(get_position(&map, 1), 0);
        map.insert(2, "C".to_string());
        assert_eq!(get_position(&map, 2), 1);
        map.insert(3, "B".to_string());
        assert_eq!(get_position(&map, 3), 2);

        map.remove(&3);
        map.insert(3, "C".to_string());
        assert_eq!(get_position(&map, 3), 2);
        assert_eq!(map.get(&3), Some("C".to_string()).as_ref());
    }
}
