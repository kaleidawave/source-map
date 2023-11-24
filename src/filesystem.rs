use std::{
    collections::HashMap,
    convert::TryInto,
    path::{Path, PathBuf},
};

use crate::{lines_columns_indexes::LineStarts, SourceId, SpanWithSource};

pub struct Source {
    pub path: PathBuf,
    pub content: String,
    pub(crate) line_starts: LineStarts,
}

#[cfg(feature = "global-source-filesystem")]
pub mod global_store {
    use super::*;

    pub struct GlobalStore;

    #[cfg(feature = "global-source-filesystem")]
    static SOURCE_IDS_TO_FILES: std::sync::RwLock<MapFileStore<NoPathMap>> =
        std::sync::RwLock::new(MapFileStore {
            sources: Vec::new(),
            mappings: NoPathMap,
        });

    impl FileSystem for GlobalStore {
        fn new_source_id_with_line_starts(
            &mut self,
            path: PathBuf,
            content: String,
        ) -> (SourceId, LineStarts) {
            SOURCE_IDS_TO_FILES
                .write()
                .unwrap()
                .new_source_id_with_line_starts(path, content)
        }

        fn get_source_by_id<T, F: for<'a> FnOnce(&'a Source) -> T>(
            &self,
            source_id: SourceId,
            f: F,
        ) -> T {
            SOURCE_IDS_TO_FILES
                .read()
                .unwrap()
                .get_source_by_id(source_id, f)
        }
    }
}

#[derive(Default)]
pub struct MapFileStore<T> {
    sources: Vec<Source>,
    mappings: T,
}

pub trait FileSystem: Sized {
    /// Generate a new [SourceId]
    fn new_source_id(&mut self, path: PathBuf, content: String) -> SourceId {
        self.new_source_id_with_line_starts(path, content).0
    }

    fn new_source_id_with_line_starts(
        &mut self,
        path: PathBuf,
        content: String,
    ) -> (SourceId, LineStarts);

    fn get_source_by_id<T, F: for<'a> FnOnce(&'a Source) -> T>(
        &self,
        source_id: SourceId,
        f: F,
    ) -> T;

    fn get_file_path_and_content(&self, source_id: SourceId) -> (PathBuf, String) {
        self.get_source_by_id(source_id, |Source { path, content, .. }| {
            (path.to_owned(), content.to_owned())
        })
    }

    fn get_file_path(&self, source_id: SourceId) -> PathBuf {
        self.get_source_by_id(source_id, |source| source.path.to_owned())
    }

    fn get_file_content(&self, source_id: SourceId) -> String {
        self.get_source_by_id(source_id, |source| source.content.to_owned())
    }

    fn get_file_whole_span(&self, source_id: SourceId) -> SpanWithSource {
        self.get_source_by_id(source_id, |source| SpanWithSource {
            start: 0,
            end: source
                .content
                .len()
                .try_into()
                .expect("File too large to convert into Span"),
            source: source_id,
        })
    }

    /// Note that this does clone the result
    fn get_file_slice<I: std::slice::SliceIndex<str>>(
        &self,
        source_id: SourceId,
        indexer: I,
    ) -> Option<<I::Output as ToOwned>::Owned>
    where
        I::Output: Sized + ToOwned,
    {
        self.get_source_by_id(source_id, |s| s.content.get(indexer).map(|v| v.to_owned()))
    }

    #[cfg(feature = "codespan-reporting")]
    fn into_code_span_store(&self) -> CodeSpanStore<Self> {
        CodeSpanStore(self)
    }
}

impl<M: PathMap> FileSystem for MapFileStore<M> {
    fn new_source_id_with_line_starts(
        &mut self,
        path: PathBuf,
        content: String,
    ) -> (SourceId, LineStarts) {
        let line_starts = LineStarts::new(&content);
        let source = Source {
            path: path.clone(),
            content,
            line_starts: line_starts.clone(),
        };
        self.sources.push(source);
        let source_id = SourceId(self.sources.len().try_into().unwrap());
        self.mappings.set_path(path, source_id);

        // Import that this is after. SourceId(0) is SourceId::NULL
        (source_id, line_starts)
    }

    fn get_source_by_id<T, F: for<'a> FnOnce(&'a Source) -> T>(
        &self,
        source_id: SourceId,
        f: F,
    ) -> T {
        f(&self.sources[source_id.0 as usize - 1])
    }
}

#[derive(Default)]
pub struct NoPathMap;

#[derive(Default)]
pub struct WithPathMap(HashMap<PathBuf, SourceId>);

impl PathMap for NoPathMap {
    fn set_path(&mut self, _path: PathBuf, _source: SourceId) {}
}

impl PathMap for WithPathMap {
    fn set_path(&mut self, path: PathBuf, source: SourceId) {
        self.0.insert(path, source);
    }
}

pub trait PathMap {
    fn set_path(&mut self, path: PathBuf, source: SourceId);
}

impl<T: PathMap> MapFileStore<T> {
    pub fn update_file(&mut self, id: SourceId, content: String) {
        let item = &mut self.sources[id.0 as usize - 1];
        item.line_starts = LineStarts::new(&content);
        item.content = content;
    }

    /// Returns the OLD and NEW length of the file's content
    pub fn append_to_file(&mut self, id: SourceId, content: &str) -> (usize, usize) {
        let existing = &mut self.sources[id.0 as usize - 1];
        let old_length = existing.content.len();
        existing.line_starts.append(old_length, content);
        existing.content.push_str(content);
        (old_length, existing.content.len())
    }
}

impl MapFileStore<WithPathMap> {
    /// Updates an **existing** entry
    ///
    /// TODO partial updates
    pub fn update_file_at_path(&mut self, path: &Path, content: String) {
        self.update_file(self.mappings.0[path], content);
    }

    /// Returns a possible [SourceId] for a path
    pub fn get_source_at_path(&self, path: &Path) -> Option<SourceId> {
        self.mappings.0.get(path).copied()
    }

    /// Either a rename or move. **Must already exist**
    pub fn change_file_path(&mut self, from: &Path, to: PathBuf) {
        let id = self.mappings.0[from];
        self.sources[id.0 as usize - 1].path = to;
        self.mappings.0.remove(from);
        self.mappings.0.insert(from.to_path_buf(), id);
    }

    pub fn create_or_update_file_at_path(&mut self, path: &Path, content: String) {
        if let Some(existing_id) = self.mappings.0.get(path) {
            self.update_file(*existing_id, content);
        } else {
            self.new_source_id(path.to_path_buf(), content);
        }
    }
}

#[cfg(feature = "codespan-reporting")]
pub struct CodeSpanStore<'a, T: FileSystem>(&'a T);

#[cfg(feature = "codespan-reporting")]
impl<'a, T: FileSystem> codespan_reporting::files::Files<'a> for CodeSpanStore<'a, T> {
    type FileId = SourceId;
    type Name = String;
    // TODO should just be &str
    type Source = String;

    fn name(&'a self, id: Self::FileId) -> Result<Self::Name, codespan_reporting::files::Error> {
        Ok(self.0.get_file_path(id).display().to_string())
    }

    fn source(
        &'a self,
        id: Self::FileId,
    ) -> Result<Self::Source, codespan_reporting::files::Error> {
        Ok(self.0.get_file_content(id))
    }

    // Implementation copied from codespan codebase
    fn line_index(
        &'a self,
        id: Self::FileId,
        byte_index: usize,
    ) -> Result<usize, codespan_reporting::files::Error> {
        Ok(self.0.get_source_by_id(id, |source| {
            source
                .line_starts
                .0
                .binary_search(&byte_index)
                .unwrap_or_else(|next_line| next_line - 1)
        }))
    }

    fn line_range(
        &'a self,
        id: Self::FileId,
        line_index: usize,
    ) -> Result<std::ops::Range<usize>, codespan_reporting::files::Error> {
        // Implementation copied from codespan codebase
        self.0.get_source_by_id(id, |source| {
            // Copied from codespan-reporting
            fn line_start(
                line_starts: &[usize],
                line_index: usize,
                source_len: usize,
            ) -> Result<usize, codespan_reporting::files::Error> {
                use std::cmp::Ordering;

                match line_index.cmp(&line_starts.len()) {
                    Ordering::Less => Ok(line_starts
                        .get(line_index)
                        .cloned()
                        .expect("failed despite previous check")),
                    Ordering::Equal => Ok(source_len),
                    Ordering::Greater => Err(codespan_reporting::files::Error::LineTooLarge {
                        given: line_index,
                        max: line_starts.len() - 1,
                    }),
                }
            }

            line_start(&source.line_starts.0, line_index, source.content.len()).and_then(
                |prev_line_start| {
                    line_start(&source.line_starts.0, line_index + 1, source.content.len())
                        .map(|next_line_start| prev_line_start..next_line_start)
                },
            )
        })
    }
}
