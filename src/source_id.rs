use std::{
    collections::HashMap,
    fmt,
    path::PathBuf,
    sync::{
        atomic::{AtomicU8, Ordering},
        RwLock,
    },
};

// #[cfg(feature="global-sources-map")]
lazy_static::lazy_static! {
    /// Maps source ids to paths and content
    static ref SOURCE_IDS_TO_FILES: RwLock<HashMap<SourceId, (PathBuf, String)>> =
    RwLock::new(HashMap::new());
}

// #[cfg(feature="global-sources-map")]
static SOURCE_ID_COUNTER: AtomicU8 = AtomicU8::new(1);

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
#[cfg_attr(feature = "span-serialize", derive(serde::Serialize))]
pub struct SourceId(u8);

impl fmt::Debug for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("SourceId({})", self.0))
    }
}

#[cfg(feature = "self-rust-tokenize")]
pub const SPAN_TOKEN_IDENT: &str = "CURRENT_SOURCE_ID";

#[cfg(feature = "self-rust-tokenize")]
impl self_rust_tokenize::SelfRustTokenize for SourceId {
    fn append_to_token_stream(
        &self,
        token_stream: &mut self_rust_tokenize::proc_macro2::TokenStream,
    ) {
        let current_source_id_reference = self_rust_tokenize::proc_macro2::Ident::new(
            SPAN_TOKEN_IDENT,
            self_rust_tokenize::proc_macro2::Span::call_site(),
        );
        self_rust_tokenize::TokenStreamExt::append(token_stream, current_source_id_reference);
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
    ///
    /// use [SourceId::get_file_slice] for a section of the source
    pub fn get_file(&self) -> Option<(PathBuf, String)> {
        SOURCE_IDS_TO_FILES.read().unwrap().get(self).cloned()
    }

    /// Note that this does clone the result
    pub fn get_file_slice<I: std::slice::SliceIndex<str>>(
        &self,
        slice: I,
    ) -> Option<(PathBuf, <I::Output as ToOwned>::Owned)>
    where
        I::Output: Sized + ToOwned,
    {
        if let Some((path_buf, string)) = SOURCE_IDS_TO_FILES.read().unwrap().get(self) {
            if let Some(slice) = string.get(slice) {
                Some((path_buf.clone(), slice.to_owned()))
            } else {
                None
            }
        } else {
            None
        }
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
