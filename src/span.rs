use std::cmp::Ordering;

use super::SourceId;

/// A start and end line and column. Also contains trace of original source
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Span {
    pub line_start: usize,
    pub column_start: usize,
    pub line_end: usize,
    pub column_end: usize,
    pub source_id: SourceId,
}

impl Span {
    /// Returns whether the end of `self` is the start of `other`
    pub fn is_adjacent_to(&self, other: &Self) -> bool {
        self.source_id == other.source_id
            && self.line_end == other.line_start
            && self.column_end == other.column_start
    }

    /// Returns a new [`Span`] which starts at the start of `self` a ends at the end of `other`
    pub fn union(&self, other: &Self) -> Span {
        Span {
            line_start: self.line_start,
            column_start: self.column_start,
            line_end: other.line_end,
            column_end: other.column_end,
            source_id: self.source_id.clone(),
        }
    }

    /// Returns whether self finishes on a line whether other starts
    /// Not commutative
    pub fn is_on_same_line(&self, other: &Self) -> bool {
        self.line_end == other.line_start
    }

    pub fn is_on_one_line(&self) -> bool {
        self.line_start == self.line_end
    }

    /// Returns whether other fits inside of self
    /// Not commutative
    pub fn contains(&self, other: &Self) -> bool {
        let starts_before = if self.line_start < other.line_start {
            true
        } else if self.line_start == other.line_start {
            self.column_start <= other.column_start
        } else {
            false
        };
        let ends_after = if self.line_end > other.line_end {
            true
        } else if self.line_end == other.line_end {
            self.column_end >= other.column_end
        } else {
            false
        };
        starts_before && ends_after
    }

    /// Returns whether other fits inside of self
    /// Not Commutative
    pub fn intersects(&self, other: &Self) -> bool {
        if other.line_start > self.line_end {
            false
        } else if other.line_start == self.line_end {
            other.column_start <= self.column_end
        } else {
            self.contains(other)
        }
    }
}

// Note that this implementation ignores the source id from the span
// TODO I think this is sound not sure tho
impl Ord for Span {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.intersects(other) || other.intersects(self) {
            let cmp = Ord::cmp(&self.line_start, &other.line_end);
            if cmp != Ordering::Equal {
                Ord::cmp(&self.column_start, &other.column_end)
            } else {
                cmp
            }
        } else if self.line_end < other.line_start {
            Ordering::Less
        } else if self.line_end == other.line_start {
            Ord::cmp(&self.column_start, &other.column_end)
        } else {
            Ordering::Greater
        }
    }
}

impl PartialOrd for Span {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod span_tests {
    use super::{SourceId, Span};

    #[test]
    fn same_line() {
        let span = Span {
            line_start: 4,
            line_end: 5,
            column_start: 2,
            column_end: 2,
            source_id: SourceId::null(),
        };
        assert!(span.is_on_same_line(&Span {
            line_start: 5,
            line_end: 5,
            column_start: 5,
            column_end: 7,
            source_id: SourceId::null()
        }));
        assert!(!span.is_on_same_line(&Span {
            line_start: 20,
            line_end: 20,
            column_start: 1,
            column_end: 8,
            source_id: SourceId::null()
        }));
    }
    #[test]
    fn one_line() {
        let span = Span {
            line_start: 2,
            line_end: 2,
            column_start: 2,
            column_end: 2,
            source_id: SourceId::null(),
        };
        assert!(span.is_on_one_line());
        let span = Span {
            line_start: 4,
            line_end: 5,
            column_start: 2,
            column_end: 2,
            source_id: SourceId::null(),
        };
        assert!(!span.is_on_one_line());
    }

    #[test]
    fn adjacent() {
        let span = Span {
            line_start: 1,
            line_end: 1,
            column_start: 2,
            column_end: 3,
            source_id: SourceId::null(),
        };
        assert!(span.is_adjacent_to(&Span {
            line_start: 1,
            line_end: 1,
            column_start: 3,
            column_end: 5,
            source_id: SourceId::null()
        }));
    }

    #[test]
    fn contains() {
        let span = Span {
            line_start: 1,
            line_end: 10,
            column_start: 2,
            column_end: 3,
            source_id: SourceId::null(),
        };
        assert!(span.contains(&Span {
            line_start: 2,
            line_end: 4,
            column_start: 3,
            column_end: 5,
            source_id: SourceId::null()
        }));
    }

    #[test]
    fn intersects() {
        let span = Span {
            line_start: 1,
            line_end: 10,
            column_start: 2,
            column_end: 3,
            source_id: SourceId::null(),
        };
        assert!(span.intersects(&Span {
            line_start: 2,
            line_end: 3,
            column_start: 3,
            column_end: 5,
            source_id: SourceId::null()
        }));
        assert!(!span.intersects(&Span {
            line_start: 200,
            line_end: 300,
            column_start: 3,
            column_end: 5,
            source_id: SourceId::null()
        }));
    }

    #[test]
    fn less_than() {
        let span1 = Span {
            line_start: 1,
            line_end: 1,
            column_start: 2,
            column_end: 2,
            source_id: SourceId::null(),
        };
        let span2 = Span {
            line_start: 4,
            line_end: 6,
            column_start: 5,
            column_end: 2,
            source_id: SourceId::null(),
        };
        assert!(span1 < span2);
    }

    #[test]
    fn equal() {
        let span1 = Span {
            line_start: 2,
            line_end: 3,
            column_start: 2,
            column_end: 2,
            source_id: SourceId::null(),
        };
        let span2 = Span {
            line_start: 1,
            line_end: 6,
            column_start: 5,
            column_end: 2,
            source_id: SourceId::null(),
        };
        assert!(Ord::cmp(&span1, &span2).is_eq());
    }

    #[test]
    fn greater_than() {
        let span1 = Span {
            line_start: 8,
            line_end: 19,
            column_start: 6,
            column_end: 2,
            source_id: SourceId::null(),
        };
        let span2 = Span {
            line_start: 4,
            line_end: 6,
            column_start: 5,
            column_end: 9,
            source_id: SourceId::null(),
        };
        assert!(span1 > span2);
    }

    #[test]
    fn sort() {
        macro_rules! span {
            ($ls: literal, $le: literal, $cs: literal, $ce: literal) => {
                Span {
                    line_start: $ls,
                    line_end: $le,
                    column_start: $cs,
                    column_end: $ce,
                    source_id: SourceId::null(),
                }
            };
        }
        let span1 = span!(1, 2, 1, 5);
        let span2 = span!(2, 5, 1, 5);
        let span3 = span!(5, 9, 1, 5);
        let span4 = span!(10, 12, 1, 5);
        let span5 = span!(11, 14, 1, 5);
        let span6 = span!(14, 20, 7, 10);
        let span7 = span!(20, 42, 1, 5);
        let mut unsorted_spans = vec![
            span2.clone(),
            span4.clone(),
            span6.clone(),
            span5.clone(),
            span1.clone(),
            span3.clone(),
            span7.clone(),
        ];
        let sorted_spans = vec![span1, span2, span3, span4, span5, span6, span7];
        unsorted_spans.sort();
        assert_eq!(unsorted_spans, sorted_spans);
    }
}
