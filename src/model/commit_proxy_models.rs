use chrono::{DateTime, FixedOffset};
use slint::{FilterModel, MapModel, ModelRc, SharedString, SortModel};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;
use std::str::FromStr;

use crate::ui;

type CommitFilterModel = Rc<FilterModel<ModelRc<ui::SlintCommit>, Box<dyn Fn(&ui::SlintCommit) -> bool>>>;
type CommitSortModel = Rc<SortModel<CommitFilterModel, Box<dyn Fn(&ui::SlintCommit, &ui::SlintCommit) -> Ordering>>>;

struct SortCriteria {
    criterion: ui::SlintCommitSortCriterion,
    is_sort_ascending: bool,
}

pub struct CommitProxyModels {
    filter_model: CommitFilterModel,
    filter_text: Rc<RefCell<SharedString>>,
    filter_author: Rc<RefCell<SharedString>>,
    sort_criteria: Rc<RefCell<SortCriteria>>,
    sort_model: CommitSortModel,
}

impl CommitProxyModels {
    fn get_sort_callback(criteria: ui::SlintCommitSortCriterion, is_sort_ascending: bool) -> Box<dyn Fn(&ui::SlintCommit, &ui::SlintCommit) -> Ordering> {
        Box::new(move |lhs: &ui::SlintCommit, rhs: &ui::SlintCommit| -> Ordering {
            use ui::SlintCommitSortCriterion::*;

            let compare_text = |lhs_text: &str, rhs_text: &str| -> Ordering {
                if is_sort_ascending {
                    lhs_text.cmp(&rhs_text)
                } else {
                    rhs_text.cmp(&lhs_text)
                }
            };
            match criteria {
                Text => compare_text(lhs.message.as_str(), rhs.message.as_str()),
                Author => compare_text(&lhs.author.as_str(), rhs.author.as_str()),
                Date => {
                    let lhs_date: DateTime<FixedOffset> = DateTime::from_str(&lhs.date).unwrap();
                    let rhs_date: DateTime<FixedOffset> = DateTime::from_str(&rhs.date).unwrap();
                    if is_sort_ascending {
                        lhs_date.cmp(&rhs_date)
                    } else {
                        rhs_date.cmp(&lhs_date)
                    }
                }
            }
        })
    }

    pub fn new(source_model: ModelRc<ui::SlintCommit>) -> Self {
        let filter_text = Rc::new(RefCell::new(SharedString::new()));
        let clone_filter_text = filter_text.clone();

        let filter_author = Rc::new(RefCell::new(SharedString::new()));
        let clone_filter_author = filter_author.clone();

        let fm: CommitFilterModel = Rc::new(FilterModel::new(
            source_model.clone(),
            Box::new(move |commit| {
                let filter_author = filter_author.clone();
                let filter_text = filter_text.clone();

                let matches_text_filter = {
                    let text_pattern = filter_text.borrow();
                    if text_pattern.is_empty() {
                        true
                    } else {
                        commit.message.to_lowercase().contains(&text_pattern.as_str().to_lowercase())
                    }
                };
                if !matches_text_filter {
                    return false;
                }

                let author_pattern = filter_author.borrow();
                if author_pattern.is_empty() {
                    true
                } else {
                    commit.author.to_lowercase().contains(&author_pattern.as_str().to_lowercase())
                }
            }),
        ));

        let sort_criteria = Rc::new(RefCell::new(SortCriteria {
            is_sort_ascending: false,
            criterion: ui::SlintCommitSortCriterion::Date,
        }));
        let clone_sort_criteria = sort_criteria.clone();

        let sm: CommitSortModel = Rc::new(SortModel::new(
            fm.clone().into(),
            Box::new(move |lhs, rhs| {
                let criteria = clone_sort_criteria.borrow();
                let compare = Self::get_sort_callback(criteria.criterion, criteria.is_sort_ascending);
                compare(lhs, rhs)
            }),
        ));

        CommitProxyModels {
            filter_model: fm.clone(),
            filter_text: clone_filter_text,
            filter_author: clone_filter_author,
            sort_criteria: sort_criteria,
            sort_model: sm,
        }
    }

    pub fn set_sort_criteria(&self, criterion: ui::SlintCommitSortCriterion, is_sort_ascending: bool) {
        *self.sort_criteria.borrow_mut() = SortCriteria { criterion, is_sort_ascending };
        self.sort_model.reset();
    }

    pub fn set_filter_text(&self, text: SharedString, filter_type: ui::SlintCommitFilterType) {
        match filter_type {
            ui::SlintCommitFilterType::Message => {
                *self.filter_text.borrow_mut() = text;
            }
            ui::SlintCommitFilterType::Author => {
                *self.filter_author.borrow_mut() = text;
            }
        }
        self.filter_model.reset();
    }

    pub fn ui_model(&self) -> ModelRc<ui::SlintCommit> {
        self.sort_model.clone().into()
    }
}
