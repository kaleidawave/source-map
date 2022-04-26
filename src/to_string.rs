use crate::{SourceMapBuilder, Span};

/// A trait for defining behavior of adding content to a buffer
pub trait ToString {
    fn push(&mut self, chr: char);

    fn push_new_line(&mut self);

    fn push_str(&mut self, string: &str);

    /// Used to push strings that may contain new lines
    fn push_str_contains_new_line(&mut self, string: &str);

    /// Adds a mapping of the from a original position in the source to the position in the current buffer
    fn add_mapping(&mut self, source_span: &Span);
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

    fn add_mapping(&mut self, _source_span: &Span) {}
}

#[derive(Default)]
pub struct StringWithSourceMap(String, SourceMapBuilder);

impl StringWithSourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns source and the source map
    /// TODO better return type
    pub fn build(self) -> (String, String) {
        (self.0, self.1.build())
    }

    #[cfg(feature = "inline-source-map")]
    pub fn build_with_inline_source_map(self) -> String {
        let Self(mut source, source_map) = self;
        let built_source_map = source_map.build();
        source.push_str("\n//# sourceMappingURL=data:application/json;base64,");
        source.push_str(&base64::encode(built_source_map));
        source
    }
}

impl ToString for StringWithSourceMap {
    fn push(&mut self, chr: char) {
        self.1.add_to_column(chr.len_utf16());
        self.0.push(chr);
    }

    fn push_new_line(&mut self) {
        self.1.add_new_line();
        self.0.push('\n');
    }

    fn push_str(&mut self, slice: &str) {
        self.1.add_to_column(slice.chars().count());
        self.0.push_str(slice);
    }

    fn push_str_contains_new_line(&mut self, slice: &str) {
        for chr in slice.chars() {
            if chr == '\n' {
                self.1.add_new_line()
            }
        }
        self.0.push_str(slice);
    }

    fn add_mapping(&mut self, source_span: &Span) {
        self.1.add_mapping(source_span);
    }
}

/// Used for getting **byte count** of the result if serialized
#[derive(Default)]
pub struct Counter(usize);

impl Counter {
    pub fn new() -> Self {
        Self::default()
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

    fn add_mapping(&mut self, _source_span: &Span) {}
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
