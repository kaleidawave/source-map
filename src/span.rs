use std::ops::Range;

use super::SourceId;

/// A start and end. Also contains trace of original source
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub source_id: SourceId,
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

    pub fn into_range(self) -> Range<usize> {
        self.into()
    }
}

impl Into<Range<usize>> for Span {
    fn into(self) -> Range<usize> {
        Range {
            start: self.start,
            end: self.end,
        }
    }
}
