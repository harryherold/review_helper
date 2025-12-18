use std::{
    collections::{HashMap, HashSet},
    convert::From,
    hash::Hash,
    path::PathBuf,
    rc::Rc,
};

use slint::SharedString;

use crate::{
    model::IdModel,
    storage::{
        RepositoryName, RepositoryStore,
        repository_storage::{DiffRangeStore, FileDiffStore, NoteStore, ReviewName, ReviewStore},
    },
    ui,
};

impl From<(usize, &RepositoryStore)> for ui::SlintRepository {
    fn from((id, value): (usize, &RepositoryStore)) -> Self {
        ui::SlintRepository {
            id: id as i32,
            first_commit: SharedString::from(&value.first_commit),
            name: String::from(&value.name).into(),
            path: value.path.as_os_str().to_str().unwrap_or_default().into(),
            base_branch: SharedString::from(&value.base_branch),
            review_model: Rc::new(IdModel::default()).into(),
        }
    }
}

impl From<&ui::SlintRepository> for RepositoryStore {
    fn from(value: &ui::SlintRepository) -> Self {
        RepositoryStore {
            first_commit: String::from(value.first_commit.as_str()),
            name: RepositoryName::from(value.name.as_str()),
            path: PathBuf::from(value.path.as_str()),
            base_branch: String::from(value.base_branch.as_str()),
        }
    }
}

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

#[derive(Debug, Clone)]
pub enum ReviewHelperError {
    GitCommandFailed(String),
    NoGitDirectory(String),
}

#[derive(Default, Clone)]
pub struct Review {
    pub diff_range: DiffRangeStore,
    pub notes: HashMap<NoteId, NoteStore>,
    pub file_diffs: HashMap<FileDiffId, FileDiffStore>,
    last_note_id: NoteId,
    last_file_diff_id: FileDiffId,
    pub name: ReviewName,
}

impl Review {
    pub fn new(store: ReviewStore, name: ReviewName) -> Self {
        let mut review = Review { name, ..Default::default() };
        review.diff_range = store.diff_range;
        store.notes.into_iter().for_each(|store| {
            let id = review.allocate_note_id();
            review.notes.insert(id, store);
        });
        store.file_diff_list.into_iter().for_each(|store| {
            let id = review.allocate_file_diff_id();
            review.file_diffs.insert(id, store);
        });

        review
    }
    fn allocate_note_id(&mut self) -> NoteId {
        if !self.last_note_id.is_next_id_valid() {
            eprintln!("Too many note ids allocated");
            std::process::abort();
        }
        self.last_note_id.increment();
        self.last_note_id.clone()
    }
    fn allocate_file_diff_id(&mut self) -> FileDiffId {
        if !self.last_file_diff_id.is_next_id_valid() {
            eprintln!("Too many file diff ids allocated");
            std::process::abort();
        }
        self.last_file_diff_id.increment();
        self.last_file_diff_id.clone()
    }
}

#[derive(Default, Clone)]
pub struct Repository {
    pub name: RepositoryName,
    pub store: RepositoryStore,
    reviews: HashMap<ReviewId, Review>,
    last_review_id: ReviewId,
    // TODO refactor names to *_map names
    review_names: HashMap<ReviewId, ReviewName>,
    review_name_set: HashSet<ReviewName>,
}

impl Repository {
    pub fn new(name: &RepositoryName, store: RepositoryStore) -> Self {
        Self {
            name: name.clone(),
            store,
            reviews: HashMap::new(),
            last_review_id: ReviewId::from(0),
            review_names: HashMap::new(),
            review_name_set: HashSet::new(),
        }
    }
    fn allocate_review_id(&mut self) -> ReviewId {
        if !self.last_review_id.is_next_id_valid() {
            eprintln!("Too many review ids allocated");
            std::process::abort();
        }
        self.last_review_id.increment();
        self.last_review_id.clone()
    }
    pub fn register_review_name(&mut self, review_name: ReviewName) -> ReviewId {
        let id = self.allocate_review_id();
        self.review_names.insert(id.clone(), review_name.clone());
        self.review_name_set.insert(review_name);
        id
    }
    pub fn insert_review(&mut self, review_id: ReviewId, review: Review) {
        assert!(self.reviews.insert(review_id, review).is_none());
    }
    pub fn new_review(&mut self, name: ReviewName) -> ReviewId {
        let id = self.register_review_name(name.clone());
        self.insert_review(id.clone(), Review { name, ..Default::default() });
        id
    }
    pub fn get_review_name(&self, id: &ReviewId) -> Option<&ReviewName> {
        self.review_names.get(id)
    }
    pub fn has_review_name(&self, name: &ReviewName) -> bool {
        self.review_name_set.contains(name)
    }
    pub fn get_mut_review(&mut self, id: &ReviewId) -> Option<&mut Review> {
        self.reviews.get_mut(id)
    }
}

#[derive(Default)]
pub struct ReviewHelperCache {
    pub repositories: HashMap<RepositoryId, Repository>,
    repository_paths: HashSet<PathBuf>,
    last_repository_id: RepositoryId,
}

impl ReviewHelperCache {
    pub fn set_repositories(&mut self, repository_stores: &Vec<RepositoryStore>) {
        self.repositories.clear();

        self.repository_paths.clear();

        repository_stores.iter().for_each(|item| {
            self.repository_paths.insert(item.path.clone());

            let id = self.allocate_repository_id();
            self.repositories.insert(id, Repository::new(&item.name, item.clone()));
        });
    }
    pub fn add_repository(&mut self, store: RepositoryStore) -> RepositoryId {
        self.repository_paths.insert(store.path.clone());

        let repository_name = store.name.clone();

        let repository = Repository::new(&repository_name, store);

        let id = self.allocate_repository_id();
        self.repositories.insert(id.clone(), repository);
        id
    }
    pub fn contains_repository_path(&self, path: &PathBuf) -> bool {
        self.repository_paths.contains(path)
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
