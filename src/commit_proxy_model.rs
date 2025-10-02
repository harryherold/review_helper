use chrono::{DateTime, FixedOffset};
use slint::{FilterModel, Model, ModelExt, ModelRc, SharedString, SortModel, StandardListViewItem, VecModel};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;
use std::str::FromStr;

use crate::git_utils::Commit;

type CommitModel = Rc<VecModel<slint::ModelRc<StandardListViewItem>>>;
type CommitFilterModel = Rc<FilterModel<ModelRc<ModelRc<StandardListViewItem>>, Box<dyn Fn(&ModelRc<StandardListViewItem>) -> bool>>>;
type CommitSortModel = Rc<SortModel<CommitFilterModel, Box<dyn Fn(&ModelRc<StandardListViewItem>, &ModelRc<StandardListViewItem>) -> Ordering>>>;

pub struct CommitProxyModel {
    commit_model: CommitModel,
    filter_model: CommitFilterModel,
    filter_text: Rc<RefCell<SharedString>>,
    sort_model: CommitSortModel,
}

impl CommitProxyModel {
    fn get_sort_callback(
        sort_index: usize,
        is_sort_ascending: bool,
    ) -> Box<dyn Fn(&ModelRc<StandardListViewItem>, &ModelRc<StandardListViewItem>) -> Ordering> {
        Box::new(move |lhs: &ModelRc<StandardListViewItem>, rhs: &ModelRc<StandardListViewItem>| -> Ordering {
            let compare_string_columns = || -> Ordering {
                if is_sort_ascending {
                    lhs.row_data(sort_index).unwrap().text.cmp(&rhs.row_data(sort_index).unwrap().text)
                } else {
                    rhs.row_data(sort_index).unwrap().text.cmp(&lhs.row_data(sort_index).unwrap().text)
                }
            };
            let compare_date_columns = || -> Ordering {
                let lhs_date: DateTime<FixedOffset> = DateTime::from_str(&lhs.row_data(sort_index).unwrap().text).unwrap();
                let rhs_date: DateTime<FixedOffset> = DateTime::from_str(&rhs.row_data(sort_index).unwrap().text).unwrap();
                if is_sort_ascending {
                    lhs_date.cmp(&rhs_date)
                } else {
                    rhs_date.cmp(&lhs_date)
                }
            };
            if sort_index == 3 {
                compare_date_columns()
            } else {
                compare_string_columns()
            }
        })
    }

    pub fn new() -> Self {
        let filter_text = Rc::new(RefCell::new(SharedString::new()));
        let clone_filter_text = filter_text.clone();
        let commit_model = Rc::new(VecModel::<ModelRc<StandardListViewItem>>::default());

        let fm: CommitFilterModel = Rc::new(FilterModel::new(
            commit_model.clone().into(),
            Box::new(move |row| {
                let filter_text = filter_text.clone();
                let pattern = filter_text.borrow();
                let message = row.row_data(1).unwrap();
                if pattern.is_empty() {
                    return true;
                } else {
                    message.text.to_lowercase().contains(&pattern.as_str().to_lowercase())
                }
            }),
        ));

        CommitProxyModel {
            commit_model,
            filter_model: fm.clone(),
            filter_text: clone_filter_text,
            sort_model: Rc::new(fm.sort_by(CommitProxyModel::get_sort_callback(3, false))),
        }
    }

    pub fn sort_by(&mut self, sort_index: usize, is_sort_ascending: bool) {
        self.sort_model = Rc::new(
            self.filter_model
                .clone()
                .sort_by(CommitProxyModel::get_sort_callback(sort_index, is_sort_ascending)),
        );
    }

    pub fn set_filter_text(&mut self, text: SharedString) {
        *self.filter_text.borrow_mut() = text;
        self.filter_model.reset();
    }

    pub fn sort_model(&self) -> ModelRc<ModelRc<StandardListViewItem>> {
        self.filter_model.clone().into()
    }

    pub fn set_commits(&mut self, commits: Vec<Commit>) {
        self.commit_model.clear();
        for commit in commits {
            let items = Rc::new(VecModel::<StandardListViewItem>::default());
            items.push(slint::SharedString::from(commit.hash).into());
            items.push(slint::SharedString::from(commit.message).into());
            items.push(slint::SharedString::from(commit.author).into());
            items.push(slint::SharedString::from(commit.date).into());

            self.commit_model.push(items.into());
        }
    }
}

impl Default for CommitProxyModel {
    fn default() -> Self {
        let model: ModelRc<ModelRc<StandardListViewItem>> = Rc::new(VecModel::<ModelRc<StandardListViewItem>>::default()).into();
        let fm: CommitFilterModel = Rc::new(model.filter(Box::new(|_| true)));
        CommitProxyModel {
            commit_model: Rc::new(VecModel::<ModelRc<StandardListViewItem>>::default()),
            filter_model: fm.clone(),
            filter_text: Rc::new(RefCell::new(SharedString::new())),
            sort_model: Rc::new(fm.sort_by(CommitProxyModel::get_sort_callback(3, false))),
        }
    }
}
