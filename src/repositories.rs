use std::{
    collections::{HashMap, HashSet, hash_map},
    convert::From,
    hash::Hash,
    path::PathBuf,
};

use crate::storage::{
    RepositoryName, RepositoryStore,
    repository_storage::{DiffRangeStore, FileDiffStore, NoteStore, ReviewName, ReviewStore},
};

macro_rules! create_id {
    ($name:ident) => {
        #[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
        pub struct $name(usize);

        impl $name {
            pub fn as_i32(&self) -> i32 {
                self.0 as i32
            }
            pub fn as_usize(&self) -> usize {
                self.0 as usize
            }
            pub fn increment(&mut self) {
                self.0 += 1;
            }
            pub fn is_next_id_valid(&self) -> bool {
                self.0 < usize::MAX
            }
        }
        impl From<usize> for $name {
            fn from(value: usize) -> Self {
                $name(value)
            }
        }
        impl From<i32> for $name {
            fn from(value: i32) -> Self {
                $name(value as usize)
            }
        }
    };
}

create_id!(RepositoryId);
create_id!(ReviewId);
create_id!(NoteId);
create_id!(FileDiffId);

#[derive(Default, Clone)]
pub struct Notes {
    id_note_map: HashMap<NoteId, NoteStore>,
    last_note_id: NoteId,
}

impl Notes {
    fn new(note_stores: Vec<NoteStore>) -> Self {
        let mut notes = Self::default();
        note_stores.into_iter().for_each(|store| {
            let id = notes.allocate_note_id();
            notes.id_note_map.insert(id, store);
        });
        notes
    }

    pub fn stores(&self) -> Vec<&NoteStore> {
        self.id_note_map.values().collect::<Vec<_>>()
    }
    pub fn iter(&self) -> hash_map::Iter<'_, NoteId, NoteStore> {
        self.id_note_map.iter()
    }
    pub fn has(&self, id: &NoteId) -> bool {
        self.id_note_map.contains_key(id)
    }

    fn allocate_note_id(&mut self) -> NoteId {
        if !self.last_note_id.is_next_id_valid() {
            eprintln!("Too many note ids allocated");
            std::process::abort();
        }
        self.last_note_id.increment();
        self.last_note_id.clone()
    }
    pub fn get_mut(&mut self, id: &NoteId) -> Option<&mut NoteStore> {
        self.id_note_map.get_mut(id)
    }
    pub fn delete_note(&mut self, id: &NoteId) -> bool {
        let result = self.id_note_map.remove(id);
        result.is_some()
    }
    pub fn add_note(&mut self, text: String, context: String) -> NoteId {
        let store = NoteStore { text, context, is_done: false };
        let id = self.allocate_note_id();
        self.id_note_map.insert(id.clone(), store);
        id
    }
}

#[derive(Default, Clone)]
pub struct FileDiffs {
    id_store_map: HashMap<FileDiffId, FileDiffStore>,
    file_id_map: HashMap<String, FileDiffId>,
    last_file_diff_id: FileDiffId,
}

impl FileDiffs {
    pub fn new(file_diff_list: Vec<FileDiffStore>) -> Self {
        let mut file_diffs = FileDiffs::default();
        file_diff_list.into_iter().for_each(|store| {
            let id = file_diffs.allocate_file_diff_id();
            let path_string = store.file_path.to_string_lossy().as_ref().to_string();
            file_diffs.id_store_map.insert(id.clone(), store);
            file_diffs.file_id_map.insert(path_string, id);
        });
        file_diffs
    }

    pub fn iter(&self) -> hash_map::Iter<'_, FileDiffId, FileDiffStore> {
        self.id_store_map.iter()
    }
    pub fn stores(&self) -> Vec<&FileDiffStore> {
        self.id_store_map.values().collect::<Vec<_>>()
    }
    pub fn get(&self, file_diff_id: &FileDiffId) -> Option<&FileDiffStore> {
        self.id_store_map.get(file_diff_id)
    }
    pub fn set_is_reviewed(&mut self, file_diff_id: &FileDiffId, is_reviewed: bool) {
        if let Some(file_diff) = self.id_store_map.get_mut(file_diff_id) {
            file_diff.is_reviewed = is_reviewed;
        }
    }
    pub fn update_file_diffs(&mut self, new_file_keys: HashSet<String>) {
        let old_file_keys = self.file_id_map.keys().cloned().collect::<HashSet<_>>();

        if old_file_keys == new_file_keys {
            return;
        }

        if new_file_keys.is_disjoint(&old_file_keys) {
            self.id_store_map.clear();
            self.file_id_map.clear();
            new_file_keys.into_iter().for_each(|file| self.add_new_file_diff(file));
        } else {
            let deleted_files = old_file_keys.difference(&new_file_keys).collect::<HashSet<_>>();
            deleted_files.into_iter().for_each(|deleted_file| self.remove_file_diff(deleted_file));

            let new_subset: HashSet<_> = new_file_keys.difference(&old_file_keys).collect();
            new_subset.into_iter().cloned().for_each(|file| self.add_new_file_diff(file));
        }
    }
    fn allocate_file_diff_id(&mut self) -> FileDiffId {
        if !self.last_file_diff_id.is_next_id_valid() {
            eprintln!("Too many file diff ids allocated");
            std::process::abort();
        }
        self.last_file_diff_id.increment();
        self.last_file_diff_id.clone()
    }
    fn add_new_file_diff(&mut self, file: String) {
        let id = self.allocate_file_diff_id();

        self.id_store_map.insert(
            id.clone(),
            FileDiffStore {
                file_path: PathBuf::from(&file),
                is_reviewed: false,
            },
        );

        self.file_id_map.insert(file, id);
    }
    fn remove_file_diff(&mut self, file: &String) {
        if let Some(id) = self.file_id_map.get(file) {
            self.id_store_map.remove(id);
            self.file_id_map.remove(file);
        }
    }
}

#[derive(Default, Clone)]
pub struct Review {
    name: ReviewName,
    diff_range: DiffRangeStore,
    pub notes: Notes,
    pub file_diffs: FileDiffs,
}

impl Review {
    pub fn new(store: ReviewStore, name: ReviewName) -> Self {
        let mut review = Review { name, ..Default::default() };
        review.diff_range = store.diff_range;

        review.file_diffs = FileDiffs::new(store.file_diff_list);

        review.notes = Notes::new(store.notes);

        review
    }

    pub fn name(&self) -> &ReviewName {
        &self.name
    }
    pub fn diff_range(&self) -> &DiffRangeStore {
        &self.diff_range
    }
    pub fn set_diff_range(&mut self, new_diff_range: DiffRangeStore) {
        self.diff_range = new_diff_range;
    }
}

#[derive(Default, Clone)]
pub struct Reviews {
    id_review_map: HashMap<ReviewId, Review>,
    id_review_name_map: HashMap<ReviewId, ReviewName>,
    review_name_set: HashSet<ReviewName>,
    last_review_id: ReviewId,
}

impl Reviews {
    fn new() -> Self {
        Self {
            id_review_map: HashMap::new(),
            last_review_id: ReviewId::from(0),
            id_review_name_map: HashMap::new(),
            review_name_set: HashSet::new(),
        }
    }

    pub fn review_name(&self, id: &ReviewId) -> Option<&ReviewName> {
        self.id_review_name_map.get(id)
    }
    pub fn has_review_name(&self, name: &ReviewName) -> bool {
        self.review_name_set.contains(name)
    }
    pub fn get(&self, id: &ReviewId) -> Option<&Review> {
        self.id_review_map.get(id)
    }
    pub fn register_review_name(&mut self, review_name: ReviewName) -> ReviewId {
        let id = self.allocate_review_id();
        self.id_review_name_map.insert(id.clone(), review_name.clone());
        self.review_name_set.insert(review_name);
        id
    }
    pub fn insert_review(&mut self, review_id: ReviewId, review: Review) {
        assert!(self.id_review_map.insert(review_id, review).is_none());
    }
    pub fn new_review(&mut self, name: ReviewName) -> ReviewId {
        let id = self.register_review_name(name.clone());
        self.insert_review(id.clone(), Review { name, ..Default::default() });
        id
    }
    pub fn delete_review(&mut self, review_id: &ReviewId) -> Option<ReviewName> {
        let review_name = self.id_review_name_map.remove(review_id)?;
        self.review_name_set.remove(&review_name);
        self.id_review_map.remove(review_id);
        Some(review_name)
    }
    pub fn get_mut(&mut self, id: &ReviewId) -> Option<&mut Review> {
        self.id_review_map.get_mut(id)
    }
    fn allocate_review_id(&mut self) -> ReviewId {
        if !self.last_review_id.is_next_id_valid() {
            eprintln!("Too many review ids allocated");
            std::process::abort();
        }
        self.last_review_id.increment();
        self.last_review_id.clone()
    }
}

#[derive(Default, Clone)]
pub struct Repository {
    pub reviews: Reviews,
    pub name: RepositoryName,
    store: RepositoryStore,
}

impl Repository {
    fn new(name: &RepositoryName, store: RepositoryStore) -> Self {
        Self {
            name: name.clone(),
            store,
            reviews: Reviews::new(),
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.store.path
    }
    pub fn store(&self) -> &RepositoryStore {
        &self.store
    }
    pub fn set_base_branch(&mut self, new_base_branch: String) {
        self.store.base_branch = new_base_branch;
    }
}

#[derive(Default)]
pub struct Repositories {
    id_repository_map: HashMap<RepositoryId, Repository>,
    repository_path_set: HashSet<PathBuf>,
    last_repository_id: RepositoryId,
}

impl Repositories {
    pub fn new(repository_stores: Vec<RepositoryStore>) -> Self {
        let mut repositories = Self {
            id_repository_map: HashMap::new(),
            repository_path_set: HashSet::new(),
            last_repository_id: RepositoryId::from(0),
        };
        repository_stores.iter().for_each(|item| {
            repositories.repository_path_set.insert(item.path.clone());

            let id = repositories.allocate_repository_id();
            repositories.id_repository_map.insert(id, Repository::new(&item.name, item.clone()));
        });
        repositories
    }

    pub fn iter(&self) -> hash_map::Iter<'_, RepositoryId, Repository> {
        self.id_repository_map.iter()
    }
    pub fn contains_repository_path(&self, path: &PathBuf) -> bool {
        self.repository_path_set.contains(path)
    }
    pub fn get(&self, id: &RepositoryId) -> Option<&Repository> {
        self.id_repository_map.get(id)
    }
    pub fn get_mut(&mut self, id: &RepositoryId) -> Option<&mut Repository> {
        self.id_repository_map.get_mut(id)
    }
    pub fn add_repository(&mut self, store: RepositoryStore) -> RepositoryId {
        self.repository_path_set.insert(store.path.clone());

        let repository_name = store.name.clone();

        let repository = Repository::new(&repository_name, store);

        let id = self.allocate_repository_id();
        self.id_repository_map.insert(id.clone(), repository);
        id
    }
    pub fn delete_repository(&mut self, repository_id: &RepositoryId) -> Option<RepositoryName> {
        let repository = self.id_repository_map.remove(repository_id)?;
        let path = repository.store.path;
        self.repository_path_set.remove(&path);
        Some(repository.name)
    }
    fn allocate_repository_id(&mut self) -> RepositoryId {
        if !self.last_repository_id.is_next_id_valid() {
            eprintln!("Too many repository ids allocated");
            std::process::abort();
        }
        self.last_repository_id.increment();
        self.last_repository_id.clone()
    }
}
