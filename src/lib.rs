use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU8, Ordering},
        RwLock,
    },
};

mod source_map;
pub use source_map::SourceMap;

lazy_static! {
    pub static ref SOURCE_IDS: RwLock<HashMap<SourceId, (String, Option<String>)>> =
        RwLock::new(HashMap::new());
}

static SOURCE_ID_COUNTER: AtomicU8 = AtomicU8::new(1);

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct SourceId(pub u8);

impl SourceId {
    pub fn new() -> Self {
        Self(SOURCE_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    /// TODO
    pub fn null() -> Self {
        Self(0)
    }
}

/// A start and end line and column. Also contains trace of original source
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Span(pub usize, pub usize, pub usize, pub usize, pub SourceId);

impl Span {
    /// Returns whether the end of `self` is the start of `other`
    pub fn is_adjacent_to(&self, other: &Self) -> bool {
        self.4 == other.4 && self.2 == other.0 && self.3 == other.1
    }

    /// Returns a new [`Span`] which starts at the start of `self` a ends at the end of `other`
    pub fn union(&self, other: &Self) -> Span {
        Span(self.0, self.1, other.2, other.3, self.4.clone())
    }
}

/// A structure with a buffer and *optional* corresponding [`SourceMap`]
pub struct ToStringer<'a>(&'a mut String, Option<&'a mut SourceMap>);

impl<'a> ToStringer<'a> {
    pub fn with_source_map(buf: &'a mut String, source_map: &'a mut SourceMap) -> Self {
        Self(buf, Some(source_map))
    }
    
    pub fn without_source_map(buf: &'a mut String) -> Self {
        Self(buf, None)
    }
}

impl ToStringer<'_> {
    pub fn push(&mut self, chr: char) {
        if let Some(ref mut source_map) = self.1 {
            source_map.add_to_column(chr.len_utf16());
        }
        self.0.push(chr);
    }

    pub fn push_new_line(&mut self) {
        if let Some(ref mut source_map) = self.1 {
            source_map.add_new_line();
        }
        self.0.push('\n');
    }

    pub fn push_str(&mut self, slice: &str) {
        if let Some(ref mut source_map) = self.1 {
            source_map.add_to_column(slice.chars().count());
        }
        self.0.push_str(slice);
    }

    /// Used to push slices that may contain new lines
    pub fn push_str_contains_new_line(&mut self, slice: &str) {
        if let Some(source_map) = self.1.as_mut() {
            for chr in slice.chars() {
                if chr == '\n' {
                    source_map.add_new_line()
                }
            }
        }
        self.0.push_str(slice);
    }

    /// Adds a mapping of the from a original position in the source to the position in the current buffer
    pub fn add_mapping(
        &mut self,
        original_line: usize,
        original_column: usize,
        source_id: SourceId,
    ) {
        if let Some(ref mut source_map) = self.1 {
            source_map.add_mapping(original_line, original_column, source_id);
        }
    }
}
