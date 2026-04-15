use std::collections::HashMap;

use slint::{Model, ModelNotify};

#[derive(Default)]
pub struct IdModel<T> {
    entity_map: std::cell::RefCell<HashMap<usize, T>>,
    index_id_map: std::cell::RefCell<Vec<usize>>,
    notify: ModelNotify,
}

impl<T: Clone + 'static> Model for IdModel<T> {
    type Data = T;

    fn row_count(&self) -> usize {
        self.index_id_map.borrow().len()
    }
    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        &self.notify
    }
    fn row_data(&self, row: usize) -> Option<Self::Data> {
        let index_id_map = self.index_id_map.borrow();
        let id = index_id_map.get(row)?;

        self.entity_map.borrow().get(id).cloned()
    }
    fn set_row_data(&self, row: usize, data: Self::Data) {
        let index_id_map = self.index_id_map.borrow();
        if let Some(id) = index_id_map.get(row) {
            self.entity_map.borrow_mut().insert(*id, data);
            self.notify.row_changed(row);
        }
    }
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}

impl<T: Clone> IdModel<T> {
    pub fn add(&self, id: usize, value: T) {
        {
            let mut entity_map = self.entity_map.borrow_mut();
            if entity_map.contains_key(&id) {
                return;
            }
            entity_map.insert(id, value);
        }
        let row = {
            let mut index_id_map = self.index_id_map.borrow_mut();
            index_id_map.push(id);
            index_id_map.len() - 1
        };
        self.notify.row_added(row, 1);
    }
    pub fn remove(&self, id: usize) {
        let mut entity_map = self.entity_map.borrow_mut();
        if !entity_map.contains_key(&id) {
            return;
        }

        let mut id_index_map = self.index_id_map.borrow_mut();
        if let Some(row) = id_index_map.iter().position(|&i| i == id) {
            id_index_map.remove(row);
            entity_map.remove(&id);
            self.notify.row_removed(row, 1);
        }
    }
    pub fn update(&self, id: usize, value: T) {
        let id_index_map = self.index_id_map.borrow();
        if let Some(index) = id_index_map.iter().position(|&i| i == id) {
            self.entity_map.borrow_mut().insert(id, value);
            self.notify.row_changed(index);
        }
    }
    pub fn get(&self, id: usize) -> Option<T> {
        self.entity_map.borrow().get(&id).cloned()
    }
    pub fn clear(&self) {
        self.entity_map.borrow_mut().clear();
        self.index_id_map.borrow_mut().clear();
        self.notify.reset();
    }
    pub fn id_to_index(&self, id: usize) -> Option<usize> {
        self.index_id_map.borrow().iter().position(|&i| i == id)
    }
    pub fn has(&self, id: usize) -> bool {
        self.entity_map.borrow().contains_key(&id)
    }
}

#[cfg(test)]
mod test {
    use std::{collections::BTreeMap, sync::atomic::AtomicUsize};

    use slint::Model;

    use super::IdModel;

    fn map_id() -> usize {
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    #[test]
    fn create_query_remove() {
        let model = IdModel::<String>::default();
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
