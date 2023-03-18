use std::{
    convert::TryInto,
    path::{Path, PathBuf},
    sync::RwLock,
};

use crate::{SourceId, Span};

#[cfg(feature = "global-source-filesystem")]
pub struct GlobalStore;

// pub struct FileSystemStore;

#[derive(Default)]
pub struct MapFileStore(Vec<(PathBuf, String)>);

#[cfg(feature = "global-source-filesystem")]
static SOURCE_IDS_TO_FILES: RwLock<MapFileStore> = RwLock::new(MapFileStore(Vec::new()));

pub trait FileSystem: Sized {
    /// Generate the
    fn new_source_id(&mut self, path: PathBuf, content: String) -> SourceId;

    /// Note that this does clone the result
    ///
    /// use [SourceId::get_file_slice] for a section of the source
    fn get_file<T, F: for<'a> FnOnce(&'a Path, &'a str) -> T>(
        &self,
        source_id: SourceId,
        f: F,
    ) -> T;

    fn get_file_path_and_content(&self, source_id: SourceId) -> (PathBuf, String) {
        self.get_file(source_id, |p, c| (p.to_owned(), c.to_owned()))
    }

    fn get_file_path(&self, source_id: SourceId) -> PathBuf {
        self.get_file(source_id, |p, _| p.to_owned())
    }

    fn get_file_content(&self, source_id: SourceId) -> String {
        self.get_file(source_id, |_, c| c.to_owned())
    }

    fn get_file_whole_span(&self, source_id: SourceId) -> Span {
        self.get_file(source_id, |_, c| Span {
            start: 0,
            end: c
                .len()
                .try_into()
                .expect("File too large to convert to span"),
            source_id,
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
        self.get_file(source_id, |_, f| f.get(indexer).map(|v| v.to_owned()))
    }

    #[cfg(feature = "codespan-reporting")]
    fn into_code_span_store(&self) -> CodeSpanStore<Self> {
        CodeSpanStore(self)
    }
}

#[cfg(feature = "global-source-filesystem")]
impl FileSystem for GlobalStore {
    fn new_source_id(&mut self, path: PathBuf, content: String) -> SourceId {
        SOURCE_IDS_TO_FILES
            .write()
            .unwrap()
            .new_source_id(path, content)
    }

    fn get_file<T, F: for<'a> FnOnce(&'a Path, &'a str) -> T>(
        &self,
        source_id: SourceId,
        f: F,
    ) -> T {
        SOURCE_IDS_TO_FILES.read().unwrap().get_file(source_id, f)
    }
}

impl FileSystem for MapFileStore {
    fn new_source_id(&mut self, path: PathBuf, content: String) -> SourceId {
        self.0.push((path, content));
        // Import that this is after. SourceId(0) is SourceId::NULL
        SourceId(self.0.len().try_into().unwrap())
    }

    fn get_file<T, F: for<'a> FnOnce(&'a Path, &'a str) -> T>(
        &self,
        source_id: SourceId,
        f: F,
    ) -> T {
        let (path, content) = &self.0[source_id.0 as usize - 1];
        f(path, content)
    }
}

#[cfg(feature = "codespan-reporting")]
pub struct CodeSpanStore<'a, T: FileSystem>(&'a T);

#[cfg(feature = "codespan-reporting")]
/// Copied from codespan-reporting
fn line_starts<'source>(source: &'source str) -> impl 'source + Iterator<Item = usize> {
    std::iter::once(0).chain(source.match_indices('\n').map(|(i, _)| i + 1))
}

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
        self.0.get_file(id, |_, source| {
            // TODO cache
            let collect = line_starts(&source).collect::<Vec<_>>();
            Ok(collect
                .binary_search(&byte_index)
                .unwrap_or_else(|next_line| next_line - 1))
        })
    }

    // Implementation copied from codespan codebase
    fn line_range(
        &'a self,
        id: Self::FileId,
        line_index: usize,
    ) -> Result<std::ops::Range<usize>, codespan_reporting::files::Error> {
        self.0.get_file(id, |_, source| {
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

            // TODO cache
            let collect = line_starts(&source).collect::<Vec<_>>();

            line_start(&collect, line_index, source.len()).and_then(|prev_line_start| {
                line_start(&collect, line_index + 1, source.len())
                    .map(|next_line_start| prev_line_start..next_line_start)
            })
        })
    }
}
