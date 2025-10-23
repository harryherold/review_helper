// independent lib

struct ReviewItemDesc {
    name: String,
    is_reviewed: bool,
}

struct NoteItemDesc {
    text: String,
    context: String,
    is_fixed: bool,
}

struct ReviewDesc {
    name: String,
    review_items: Vec<ReviewItemAdapter>,
    note_items: Vec<NoteItemDesc>,
}

trait ReviewStorage {
    fn store_review(&self, path: &PathBuf, review_desc: ReviewDesc) -> Result<()>;
    fn load_review(&self, path: &PathBuf) -> Result<ReviewDesc>;
}

// translation from/to ui types
use std::convert::From;

extern struct NoteItemUi;

impl From<NoteItemUi> for NoteItemDesc {
    fn from(item: NoteItemUi) -> Self {
        NoteItemDesc {
            text: item.text.into(),
            context: item.context.into(),
            is_fixed: item.is_fixed,
        }
    }
}

impl From<NoteItemDesc> for NoteItemUi {
    fn from(item: NoteItemDesc) -> Self {
        NoteItemUi {
            text: SharedString::from(item.text),
            context: SharedString::from(item.context),
            is_fixed: item.is_fixed,
        }
    }
}