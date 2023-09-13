use super::SourceId;
use crate::{encodings::*, FileSystem};
use std::{convert::TryInto, fmt, ops::Range};

/// A start and end. Also contains trace of original source
#[derive(PartialEq, Eq, Clone, Hash)]
#[cfg_attr(feature = "span-serialize", derive(serde::Serialize))]
#[cfg_attr(
    feature = "self-rust-tokenize",
    derive(self_rust_tokenize::SelfRustTokenize)
)]
pub struct BaseSpan<T> {
    pub start: u32,
    pub end: u32,
    pub source: T,
}

pub type SourceLessSpan = BaseSpan<()>;
pub type Span = BaseSpan<SourceId>;

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "{}..{}#{}",
            self.start, self.end, self.source.0
        ))
    }
}

impl fmt::Debug for SourceLessSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}..{}", self.start, self.end,))
    }
}

impl SourceLessSpan {
    /// Returns whether the end of `self` is the start of `other`
    pub fn is_adjacent_to(&self, other: &Self) -> bool {
        self.end == other.start
    }

    /// Returns a new [`SourceLessSpan`] which starts at the start of `self` a ends at the end of `other`
    pub fn union(&self, other: &Self) -> SourceLessSpan {
        SourceLessSpan {
            start: self.start,
            end: other.end,
            source: (),
        }
    }
}

impl Span {
    pub fn get_start(&self) -> Position {
        Position(self.start, self.source)
    }

    pub fn get_end(&self) -> Position {
        Position(self.end, self.source)
    }

    pub fn into_line_column_span<T: StringEncoding>(
        self,
        fs: &impl FileSystem,
    ) -> LineColumnSpan<T> {
        fs.get_source_by_id(self.source, |source| {
            let line_start = source
                .line_starts
                .get_index_of_line_pos_is_on(self.start as usize);
            let line_start_byte = source.line_starts.0[line_start];
            let column_start =
                T::get_encoded_length(&source.content[line_start_byte..(self.start as usize)]);

            let line_end = source
                .line_starts
                .get_index_of_line_pos_is_on(self.end as usize);
            let line_end_byte = source.line_starts.0[line_end];
            let column_end =
                T::get_encoded_length(&source.content[line_end_byte..(self.end as usize)]);

            LineColumnSpan {
                line_start: line_start as u32,
                column_start: column_start as u32,
                line_end: line_end as u32,
                column_end: column_end as u32,
                encoding: T::new(),
                source: self.source,
            }
        })
    }

    /// TODO explain use cases
    pub const NULL_SPAN: Span = Span {
        start: 0,
        end: 0,
        source: SourceId::NULL,
    };

    /// TODO explain use cases
    pub fn is_null(&self) -> bool {
        self.source == SourceId::NULL
    }
}

// TODO why are two implementations needed
impl<T> From<BaseSpan<T>> for Range<u32> {
    fn from(span: BaseSpan<T>) -> Range<u32> {
        Range {
            start: span.start,
            end: span.end,
        }
    }
}

impl<T> From<BaseSpan<T>> for Range<usize> {
    fn from(span: BaseSpan<T>) -> Range<usize> {
        Range {
            start: span.start.try_into().unwrap(),
            end: span.end.try_into().unwrap(),
        }
    }
}

/// A scalar/singular byte wise position. **Zero based**
#[derive(PartialEq, Eq, Clone)]
pub struct Position(pub u32, pub SourceId);

impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.1.is_null() {
            f.write_fmt(format_args!("{}", self.0,))
        } else {
            f.write_fmt(format_args!("{}#{}", self.0, self.1 .0))
        }
    }
}

impl Position {
    pub fn into_line_column_position<T: StringEncoding>(
        self,
        fs: &impl FileSystem,
    ) -> LineColumnPosition<T> {
        fs.get_source_by_id(self.1, |source| {
            let line = source
                .line_starts
                .get_index_of_line_pos_is_on(self.0 as usize);
            let line_byte = source.line_starts.0[line];
            let column =
                T::get_encoded_length(&source.content[line_byte..(self.0 as usize)]) as u32;
            LineColumnPosition {
                line: line as u32,
                column,
                encoding: T::new(),
                source: self.1,
            }
        })
    }
}

/// **Zero based**
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LineColumnPosition<T: StringEncoding> {
    pub line: u32,
    pub column: u32,
    pub source: SourceId,
    encoding: T,
}

impl<T: StringEncoding> LineColumnPosition<T> {
    pub fn into_scalar_position(self, fs: &impl FileSystem) -> Position {
        fs.get_source_by_id(self.source, |source| {
            let line_byte = source.line_starts.0[self.line as usize];
            let column_length =
                T::encoded_length_to_byte_count(&source.content[line_byte..], self.column as usize);
            Position((line_byte + column_length).try_into().unwrap(), self.source)
        })
    }
}

/// **Zero based**
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LineColumnSpan<T: StringEncoding> {
    pub line_start: u32,
    pub column_start: u32,
    pub line_end: u32,
    pub column_end: u32,
    pub source: SourceId,
    encoding: T,
}

impl<T: StringEncoding> LineColumnSpan<T> {
    pub fn into_scalar_span(self, fs: &impl FileSystem) -> Span {
        fs.get_source_by_id(self.source, |source| {
            let line_start_byte = source.line_starts.0[self.line_start as usize];
            let column_start_length = T::encoded_length_to_byte_count(
                &source.content[line_start_byte..],
                self.column_start as usize,
            );

            let line_end_byte = source.line_starts.0[self.line_end as usize];
            let column_end_length = T::encoded_length_to_byte_count(
                &source.content[line_end_byte..],
                self.column_start as usize,
            );

            Span {
                start: (line_start_byte + column_start_length).try_into().unwrap(),
                end: (line_end_byte + column_end_length).try_into().unwrap(),
                source: self.source,
            }
        })
    }
}

#[cfg(feature = "lsp-types-morphisms")]
impl Into<lsp_types::Position> for LineColumnPosition<Utf8> {
    fn into(self) -> lsp_types::Position {
        lsp_types::Position {
            line: self.line,
            character: self.column,
        }
    }
}

#[cfg(feature = "lsp-types-morphisms")]
impl Into<lsp_types::Range> for LineColumnSpan<Utf8> {
    fn into(self) -> lsp_types::Range {
        lsp_types::Range {
            start: lsp_types::Position {
                line: self.line_start,
                character: self.column_start,
            },
            end: lsp_types::Position {
                line: self.line_end,
                character: self.column_end,
            },
        }
    }
}

#[cfg(feature = "lsp-types-morphisms")]
impl From<lsp_types::Position> for LineColumnPosition<Utf8> {
    fn from(lsp_position: lsp_types::Position) -> Self {
        LineColumnPosition {
            column: lsp_position.character,
            line: lsp_position.line,
            encoding: Utf8,
            source: SourceId::NULL,
        }
    }
}

#[cfg(feature = "lsp-types-morphisms")]
impl From<lsp_types::Range> for LineColumnSpan<Utf8> {
    fn from(lsp_range: lsp_types::Range) -> Self {
        LineColumnSpan {
            line_start: lsp_range.start.line,
            column_start: lsp_range.start.character,
            line_end: lsp_range.end.line,
            column_end: lsp_range.end.character,
            encoding: Utf8,
            source: SourceId::NULL,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{encodings::Utf8, MapFileStore, NoPathMap};

    use super::*;

    const SOURCE: &str = "Hello World
I am a paragraph over two lines
Another line";

    fn get_file_system_and_source() -> (MapFileStore<NoPathMap>, SourceId) {
        let mut fs = MapFileStore::default();
        let source = fs.new_source_id("".into(), SOURCE.into());
        (fs, source)
    }

    #[test]
    fn scalar_span_to_line_column() {
        let (fs, source) = get_file_system_and_source();

        let paragraph_span = Span {
            start: 19,
            end: 28,
            source,
        };

        assert_eq!(&SOURCE[Range::from(paragraph_span.clone())], "paragraph");
        assert_eq!(
            paragraph_span.into_line_column_span(&fs),
            LineColumnSpan {
                line_start: 1,
                column_start: 7,
                line_end: 1,
                column_end: 16,
                encoding: Utf8,
                source
            }
        );
    }

    #[test]
    fn scalar_position_to_line_column() {
        let (fs, source) = get_file_system_and_source();

        let l_of_line_position = Position(52, source);
        assert_eq!(&SOURCE[l_of_line_position.0.try_into().unwrap()..], "line");

        assert_eq!(
            l_of_line_position.into_line_column_position(&fs),
            LineColumnPosition {
                line: 2,
                column: 8,
                encoding: Utf8,
                source
            }
        );
    }

    #[test]
    fn line_column_position_to_position() {
        let (fs, source) = get_file_system_and_source();
        let start_of_another_position = LineColumnPosition {
            line: 2,
            column: 0,
            source,
            encoding: Utf8,
        };
        assert_eq!(
            start_of_another_position.into_scalar_position(&fs),
            Position(44, source)
        );
    }

    #[test]
    fn line_column_span_to_span() {
        let (fs, source) = get_file_system_and_source();
        let line_another_span = LineColumnSpan {
            line_start: 1,
            column_start: 26,
            line_end: 2,
            column_end: 12,
            source,
            encoding: Utf8,
        };

        let line_another_span = line_another_span.into_scalar_span(&fs);
        assert_eq!(
            &SOURCE[Range::from(line_another_span)],
            "lines\nAnother line"
        );
    }
}
