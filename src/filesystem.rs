use std::{convert::TryInto, path::PathBuf, sync::RwLock};

use crate::{lines_columns_indexes::LineStarts, SourceId, Span};

pub struct Source {
    pub path: PathBuf,
    pub content: String,
    pub(crate) line_starts: LineStarts,
}

#[cfg(feature = "global-source-filesystem")]
pub struct GlobalStore;

// pub struct FileSystemStore;

#[derive(Default)]
pub struct MapFileStore(Vec<Source>);

#[cfg(feature = "global-source-filesystem")]
static SOURCE_IDS_TO_FILES: RwLock<MapFileStore> = RwLock::new(MapFileStore(Vec::new()));

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

    fn get_source<T, F: for<'a> FnOnce(&'a Source) -> T>(&self, source_id: SourceId, f: F) -> T;

    fn get_file_path_and_content(&self, source_id: SourceId) -> (PathBuf, String) {
        self.get_source(source_id, |Source { path, content, .. }| {
            (path.to_owned(), content.to_owned())
        })
    }

    fn get_file_path(&self, source_id: SourceId) -> PathBuf {
        self.get_source(source_id, |source| source.path.to_owned())
    }

    fn get_file_content(&self, source_id: SourceId) -> String {
        self.get_source(source_id, |source| source.content.to_owned())
    }

    fn get_file_whole_span(&self, source_id: SourceId) -> Span {
        self.get_source(source_id, |source| Span {
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
        self.get_source(source_id, |s| s.content.get(indexer).map(|v| v.to_owned()))
    }

    #[cfg(feature = "codespan-reporting")]
    fn into_code_span_store(&self) -> CodeSpanStore<Self> {
        CodeSpanStore(self)
    }
}

#[cfg(feature = "global-source-filesystem")]
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

    fn get_source<T, F: for<'a> FnOnce(&'a Source) -> T>(&self, source_id: SourceId, f: F) -> T {
        SOURCE_IDS_TO_FILES.read().unwrap().get_source(source_id, f)
    }
}

impl FileSystem for MapFileStore {
    fn new_source_id_with_line_starts(
        &mut self,
        path: PathBuf,
        content: String,
    ) -> (SourceId, LineStarts) {
        let line_starts = LineStarts::new(&content);
        let source = Source {
            path,
            content,
            line_starts: line_starts.clone(),
        };
        self.0.push(source);
        // Import that this is after. SourceId(0) is SourceId::NULL
        (SourceId(self.0.len().try_into().unwrap()), line_starts)
    }

    fn get_source<T, F: for<'a> FnOnce(&'a Source) -> T>(&self, source_id: SourceId, f: F) -> T {
        f(&self.0[source_id.0 as usize - 1])
    }
}

#[cfg(feature = "codespan-reporting")]
pub struct CodeSpanStore<'a, T: FileSystem>(&'a T);

#[cfg(feature = "codespan-reporting")]
impl<'a, T: FileSystem> codespan_reporting::files::Files<'a> for CodeSpanStore<'a, T> {
    type FileId = SourceId;
    type Name = String;
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
        Ok(self.0.get_source(id, |source| {
            source
                .line_starts
                .0
                .binary_search(&byte_index)
                .unwrap_or_else(|next_line| next_line - 1)
        }))
    }

    // Implementation copied from codespan codebase
    fn line_range(
        &'a self,
        id: Self::FileId,
        line_index: usize,
    ) -> Result<std::ops::Range<usize>, codespan_reporting::files::Error> {
        self.0.get_source(id, |source| {
            /// Copied from codespan-reporting
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
