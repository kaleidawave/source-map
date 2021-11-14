use crate::{SourceId, SourceMap};

pub trait ToString {
    fn push(&mut self, chr: char);

    fn push_new_line(&mut self);

    fn push_str(&mut self, string: &str);

    /// Used to push strings that may contain new lines
    fn push_str_contains_new_line(&mut self, string: &str);

    /// Adds a mapping of the from a original position in the source to the position in the current buffer
    fn add_mapping(&mut self, original_line: usize, original_column: usize, source_id: SourceId);
}

impl ToString for String {
    fn push(&mut self, chr: char) {
        self.push(chr);
    }

    fn push_new_line(&mut self) {
        self.push('\n');
    }

    fn push_str(&mut self, string: &str) {
        self.push_str(string)
    }

    fn push_str_contains_new_line(&mut self, string: &str) {
        self.push_str(string)
    }

    fn add_mapping(
        &mut self,
        _original_line: usize,
        _original_column: usize,
        _source_id: SourceId,
    ) {
    }
}

/// A structure with a buffer and *optional* corresponding [`SourceMap`]
pub struct SourceMapBuilder<'a>(&'a mut String, &'a mut SourceMap);

impl<'a> SourceMapBuilder<'a> {
    pub fn new(buf: &'a mut String, source_map: &'a mut SourceMap) -> Self {
        Self(buf, source_map)
    }
}

impl SourceMapBuilder<'_> {
    pub fn push(&mut self, chr: char) {
        self.1.add_to_column(chr.len_utf16());
        self.0.push(chr);
    }

    pub fn push_new_line(&mut self) {
        self.1.add_new_line();
        self.0.push('\n');
    }

    pub fn push_str(&mut self, slice: &str) {
        self.1.add_to_column(slice.chars().count());
        self.0.push_str(slice);
    }

    pub fn push_str_contains_new_line(&mut self, slice: &str) {
        for chr in slice.chars() {
            if chr == '\n' {
                self.1.add_new_line()
            }
        }
        self.0.push_str(slice);
    }

    pub fn add_mapping(
        &mut self,
        original_line: usize,
        original_column: usize,
        source_id: SourceId,
    ) {
        self.1
            .add_mapping(original_line, original_column, source_id);
    }
}

/// Used for getting **byte count** of the result if serialized
pub struct Counter(usize);

impl Counter {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn get_count(self) -> usize {
        self.0
    }
}

impl ToString for Counter {
    fn push(&mut self, chr: char) {
        self.0 += chr.len_utf8();
    }

    fn push_new_line(&mut self) {
        self.push('\n');
    }

    fn push_str(&mut self, string: &str) {
        self.0 += string.len();
    }

    fn push_str_contains_new_line(&mut self, string: &str) {
        self.0 += string.len();
    }

    fn add_mapping(
        &mut self,
        _original_line: usize,
        _original_column: usize,
        _source_id: SourceId,
    ) {
    }
}

#[cfg(test)]
mod to_string_tests {
    use super::{Counter, ToString};

    fn serializer<T: ToString>(t: &mut T) {
        t.push_str("Hello");
        t.push(' ');
        t.push_str("World");
    }

    #[test]
    fn string_concatenation() {
        let mut s = String::new();
        serializer(&mut s);
        assert_eq!(&s, "Hello World");
    }

    #[test]
    fn counting() {
        let mut s = Counter::new();
        serializer(&mut s);
        assert_eq!(s.get_count(), 11);
    }
}
