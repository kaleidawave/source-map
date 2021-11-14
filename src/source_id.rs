use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicU8, Ordering},
        RwLock,
    },
};

use lazy_static::lazy_static;

lazy_static! {
    /// Maps source ids to paths and content
    static ref SOURCE_IDS_TO_FILES: RwLock<HashMap<SourceId, (PathBuf, String)>> =
        RwLock::new(HashMap::new());
}

static SOURCE_ID_COUNTER: AtomicU8 = AtomicU8::new(1);

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct SourceId(pub u8);

impl SourceId {
    pub fn new(path: PathBuf, content: String) -> Self {
        let source_id = Self(SOURCE_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
        SOURCE_IDS_TO_FILES
            .write()
            .unwrap()
            .insert(source_id, (path, content));
        source_id
    }

    /// **ONLY FOR TESTING METHODS**
    pub const fn null() -> Self {
        Self(0)
    }

    /// Note that this does clone the result
    pub fn get_file(&self) -> Option<(PathBuf, String)> {
        SOURCE_IDS_TO_FILES.read().unwrap().get(self).cloned()
    }
}
