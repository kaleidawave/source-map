use std::{fmt, ops::Range, str::CharIndices};

use super::SourceId;

/// A start and end. Also contains trace of original source
#[derive(PartialEq, Eq, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub source_id: SourceId,
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.source_id.is_null() {
            f.write_fmt(format_args!(
                "{}..{}#{}",
                self.start,
                self.end,
                self.source_id.get_count()
            ))
        } else {
            f.write_fmt(format_args!("{}..{}", self.start, self.end,))
        }
    }
}

impl Span {
    /// Returns whether the end of `self` is the start of `other`
    pub fn is_adjacent_to(&self, other: &Self) -> bool {
        self.source_id == other.source_id && self.end == other.start
    }

    /// Returns a new [`Span`] which starts at the start of `self` a ends at the end of `other`
    pub fn union(&self, other: &Self) -> Span {
        Span {
            start: self.start,
            end: other.end,
            source_id: self.source_id,
        }
    }

    pub fn into_line_column_span(self, on_slice: &str) -> LineColumnSpan {
        let mut char_indices = on_slice.char_indices();
        let Span { start, end, .. } = self;
        let (line_start, column_start) =
            get_line_column_from_string_and_idx(start, &mut char_indices);
        let (line_end, column_end) = get_line_column_from_string_and_idx(end, &mut char_indices);
        LineColumnSpan {
            line_start,
            column_start,
            line_end: line_end + line_start,
            column_end: column_end + column_start + 1,
        }
    }
}

impl From<Span> for Range<usize> {
    fn from(span: Span) -> Range<usize> {
        Range {
            start: span.start,
            end: span.end,
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct Position(pub usize);

impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Position({})", &self.0))
    }
}

impl Position {
    pub fn into_line_column_position(self, on_slice: &str) -> LineColumnPosition {
        let (line, column) =
            get_line_column_from_string_and_idx(self.0, &mut on_slice.char_indices());
        LineColumnPosition { line, column }
    }
}

/// Returns `(line, column)`
fn get_line_column_from_string_and_idx(end: usize, char_indices: &mut CharIndices) -> (u32, u32) {
    let (mut line, mut column) = (0, 0);
    for (_, chr) in char_indices.take_while(|(idx, _)| *idx < end) {
        if chr == '\n' {
            line += 1;
            column = 0;
        } else {
            column += chr.len_utf8() as u32;
        }
    }
    (line, column)
}

/// Converts line and column to index in string
/// Isomorphism of [get_line_column_from_string_and_idx]
fn line_column_position_to_position(
    mut line: u32,
    column: u32,
    char_indices: &mut CharIndices,
    full_length: usize,
) -> usize {
    while line > 0 {
        // TODO unwrap
        if '\n' == char_indices.next().unwrap().1 {
            line -= 1;
        }
    }
    char_indices
        .next()
        .map(|(idx, _)| idx)
        .unwrap_or(full_length)
        + column as usize
}

/// TODO should these include [SourceId]?
/// Zero based
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LineColumnPosition {
    line: u32,
    column: u32,
}

impl LineColumnPosition {
    pub fn into_scalar_position(self, on_slice: &str) -> Position {
        Position(line_column_position_to_position(
            self.line,
            self.column,
            &mut on_slice.char_indices(),
            on_slice.len(),
        ))
    }
}

/// TODO should these include [SourceId]?
/// Zero based
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LineColumnSpan {
    line_start: u32,
    column_start: u32,
    line_end: u32,
    column_end: u32,
}

impl LineColumnSpan {
    pub fn into_scalar_span(self, on_slice: &str, source_id: SourceId) -> Span {
        let mut char_indices = on_slice.char_indices();
        let start = line_column_position_to_position(
            self.line_start,
            self.column_start,
            &mut char_indices,
            on_slice.len(),
        );
        let end = if self.line_start == self.line_end {
            start + self.column_end as usize - self.column_start as usize
        } else {
            line_column_position_to_position(
                self.line_end - self.line_start,
                self.column_end,
                &mut char_indices,
                on_slice.len(),
            )
        };
        Span {
            start,
            end,
            source_id,
        }
    }
}

#[cfg(feature = "lsp-types-morphisms")]
impl Into<lsp_types::Position> for LineColumnPosition {
    fn into(self) -> lsp_types::Position {
        lsp_types::Position {
            line: self.line,
            character: self.column,
        }
    }
}

#[cfg(feature = "lsp-types-morphisms")]
impl Into<lsp_types::Range> for LineColumnSpan {
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
impl From<lsp_types::Position> for LineColumnPosition {
    fn from(lsp_position: lsp_types::Position) -> Self {
        LineColumnPosition {
            column: lsp_position.character,
            line: lsp_position.line,
        }
    }
}

#[cfg(feature = "lsp-types-morphisms")]
impl From<lsp_types::Range> for LineColumnSpan {
    fn from(lsp_range: lsp_types::Range) -> Self {
        LineColumnSpan {
            line_start: lsp_range.start.line,
            column_start: lsp_range.start.character,
            line_end: lsp_range.end.line,
            column_end: lsp_range.end.character,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "Hello World
I am a paragraph over two lines
Another line";

    #[test]
    fn scalar_span_to_line_column() {
        let paragraph_span = Span {
            start: 19,
            end: 28,
            source_id: SourceId::new_null(),
        };
        assert_eq!(&SOURCE[Range::from(paragraph_span.clone())], "paragraph");

        assert_eq!(
            paragraph_span.into_line_column_span(SOURCE),
            LineColumnSpan {
                line_start: 1,
                column_start: 7,
                line_end: 1,
                column_end: 16,
            }
        );
    }

    #[test]
    fn scalar_position_to_line_column() {
        let l_of_line_position = Position(52);
        assert_eq!(&SOURCE[l_of_line_position.0..], "line");

        assert_eq!(
            l_of_line_position.into_line_column_position(SOURCE),
            LineColumnPosition { line: 2, column: 8 }
        );
    }

    #[test]
    fn line_column_position_to_position() {
        let start_of_another_position = LineColumnPosition { line: 2, column: 0 };
        assert_eq!(
            start_of_another_position.into_scalar_position(SOURCE),
            Position(44)
        );
    }

    #[test]
    fn line_column_span_to_span() {
        let line_another_span = LineColumnSpan {
            line_start: 1,
            column_start: 26,
            line_end: 2,
            column_end: 12,
        };
        let line_another_span = line_another_span.into_scalar_span(SOURCE, SourceId::new_null());
        assert_eq!(
            &SOURCE[Range::from(line_another_span)],
            "lines\nAnother line"
        );
    }
}
