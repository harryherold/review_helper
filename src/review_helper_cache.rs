use std::{
    collections::{HashMap, HashSet},
    convert::From,
    hash::Hash,
    path::PathBuf,
    rc::Rc,
};

use slint::{SharedString, VecModel};

use crate::{
    model::IdModel,
    storage::{
        RepositoryName, RepositoryStore,
        repository_storage::{DiffRangeStore, FileDiffStore, NoteStore, ReviewName, ReviewStore},
    },
    ui,
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
    diff_range: DiffRangeStore,
    pub notes: HashMap<NoteId, NoteStore>,
    pub file_diffs: HashMap<FileDiffId, FileDiffStore>,
    last_note_id: NoteId,
    last_file_diff_id: FileDiffId,
}

impl Review {
    pub fn new(store: ReviewStore) -> Self {
        let mut review = Review::default();
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
    review_names: HashMap<ReviewId, ReviewName>,
}

impl Repository {
    pub fn new(name: &RepositoryName, store: RepositoryStore) -> Self {
        Self {
            name: name.clone(),
            store,
            reviews: HashMap::new(),
            last_review_id: ReviewId::from(0),
            review_names: HashMap::new(),
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
        self.review_names.insert(id.clone(), review_name);
        id
    }
    pub fn insert_review(&mut self, review_id: ReviewId, review: Review) {
        assert!(self.reviews.insert(review_id, review).is_none());
    }
    pub fn get_review_name(&self, id: &ReviewId) -> Option<&ReviewName> {
        self.review_names.get(id)
    }
}

#[derive(Default)]
pub struct ReviewHelperCache {
    pub repositories: HashMap<RepositoryName, Repository>,
    repository_paths: HashSet<PathBuf>,
}

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

impl ReviewHelperCache {
    pub fn set_repositories(&mut self, repository_stores: &Vec<RepositoryStore>) {
        self.repositories.clear();
        self.repository_paths.clear();

        repository_stores.iter().for_each(|item| {
            self.repository_paths.insert(item.path.clone());
            self.repositories.insert(item.name.clone(), Repository::new(&item.name, item.clone()));
        });
    }
    pub fn add_repository(&mut self, store: RepositoryStore) {
        self.repository_paths.insert(store.path.clone());

        let repository_name = store.name.clone();

        let repository = Repository::new(&repository_name, store);

        self.repositories.insert(repository_name, repository);
    }
    pub fn contains_repository_path(&self, path: &PathBuf) -> bool {
        self.repository_paths.contains(path)
    }
    pub fn get_mut_repository(&mut self, name: &RepositoryName) -> Option<&mut Repository> {
        self.repositories.get_mut(name)
    }
}
