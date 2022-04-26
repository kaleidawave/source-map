use std::{
    collections::HashMap,
    fmt,
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

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
#[cfg_attr(feature = "span-serialize", derive(serde::Serialize))]
pub struct SourceId(u8);

impl fmt::Debug for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("SourceId({})", self.0))
    }
}

impl SourceId {
    /// Returns a [SourceId] handle that references a file and its content
    pub fn new(path: PathBuf, content: String) -> Self {
        let source_id = Self(SOURCE_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
        SOURCE_IDS_TO_FILES
            .write()
            .unwrap()
            .insert(source_id, (path, content));
        source_id
    }

    /// For content which does not have a source file **use with caution**
    pub const NULL: Self = Self(0);

    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Note that this does clone the result
    pub fn get_file(&self) -> Option<(PathBuf, String)> {
        SOURCE_IDS_TO_FILES.read().unwrap().get(self).cloned()
    }

    /// Unwraps the count
    #[doc(hidden)]
    pub fn get_count(&self) -> u8 {
        self.0
    }

    /// Remove the filename and content mapping behind the handle
    /// **make sure that this is the only [SourceId]**
    pub fn drop_handle(self) {
        SOURCE_IDS_TO_FILES.write().unwrap().remove(&self);
    }
}
