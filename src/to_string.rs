use crate::{FileSystem, SourceMap, SourceMapBuilder, SpanWithSource};

/// A trait for defining behavior of adding content to a buffer. As well as register markers for source maps
pub trait ToString {
    /// Append character
    fn push(&mut self, chr: char);

    /// Append a new line character
    fn push_new_line(&mut self);

    /// Use [ToString::push_str_contains_new_line] if `string` could contain new lines
    fn push_str(&mut self, string: &str);

    /// Used to push strings that may contain new lines
    fn push_str_contains_new_line(&mut self, string: &str);

    /// Adds a mapping of the from a original position in the source to the position in the current buffer
    ///
    /// **Should be called before adding new content**
    fn add_mapping(&mut self, source_span: &SpanWithSource);

    /// Some implementors might not ToString the whole input. This signals for users to end early as further usage
    /// of this trait has no effect
    fn halt(&self) -> bool {
        false
    }
}

// TODO clarify calls
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

    fn add_mapping(&mut self, _source_span: &SpanWithSource) {}
}

/// Building a source along with its source map
#[derive(Default)]
pub struct StringWithSourceMap(String, SourceMapBuilder);

impl StringWithSourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns source and the source map
    pub fn build(self, filesystem: &impl FileSystem) -> (String, SourceMap) {
        (self.0, self.1.build(filesystem))
    }

    #[cfg(feature = "inline-source-map")]
    /// Build the output and append the source map in base 64
    pub fn build_with_inline_source_map(self, filesystem: &impl FileSystem) -> String {
        use base64::Engine;

        let Self(mut source, source_map) = self;
        let built_source_map = source_map.build(filesystem);
        // Inline URL:
        source.push_str("\n//# sourceMappingURL=data:application/json;base64,");
        source.push_str(
            &base64::prelude::BASE64_STANDARD.encode(built_source_map.to_json(filesystem)),
        );
        source
    }
}

// TODO use ToString for self.0
impl ToString for StringWithSourceMap {
    fn push(&mut self, chr: char) {
        self.0.push(chr);
        self.1.add_to_column(chr.len_utf16());
    }

    fn push_new_line(&mut self) {
        self.0.push('\n');
        self.1.add_new_line();
    }

    fn push_str(&mut self, slice: &str) {
        self.0.push_str(slice);
        self.1.add_to_column(slice.chars().count());
    }

    fn push_str_contains_new_line(&mut self, slice: &str) {
        self.0.push_str(slice);
        for chr in slice.chars() {
            if chr == '\n' {
                self.1.add_new_line()
            }
        }
    }

    fn add_mapping(&mut self, source_span: &SpanWithSource) {
        self.1.add_mapping(source_span);
    }
}

/// Building a source along with its source map
///
/// Really for debug builds
#[derive(Default)]
pub struct StringWithOptionalSourceMap {
    pub source: String,
    pub source_map: Option<SourceMapBuilder>,
    pub quit_after: Option<usize>,
}

impl StringWithOptionalSourceMap {
    pub fn new(with_source_map: bool) -> Self {
        Self {
            source: String::new(),
            source_map: with_source_map.then(SourceMapBuilder::new),
            quit_after: None,
        }
    }

    /// Returns source and the source map
    pub fn build(self, filesystem: &impl FileSystem) -> (String, Option<SourceMap>) {
        (self.source, self.source_map.map(|sm| sm.build(filesystem)))
    }

    #[cfg(feature = "inline-source-map")]
    /// Build the output and append the source map in base 64
    pub fn build_with_inline_source_map(self, filesystem: &impl FileSystem) -> String {
        use base64::Engine;

        let Self {
            mut source,
            source_map,
            quit_after: _,
        } = self;
        let built_source_map = source_map.unwrap().build(filesystem);
        // Inline URL:
        source.push_str("\n//# sourceMappingURL=data:application/json;base64,");
        source.push_str(
            &base64::prelude::BASE64_STANDARD.encode(built_source_map.to_json(filesystem)),
        );
        source
    }
}

impl ToString for StringWithOptionalSourceMap {
    fn push(&mut self, chr: char) {
        self.source.push(chr);
        if let Some(ref mut sm) = self.source_map {
            sm.add_to_column(chr.len_utf16());
        }
    }

    fn push_new_line(&mut self) {
        self.source.push('\n');
        if let Some(ref mut sm) = self.source_map {
            sm.add_new_line();
        }
    }

    fn push_str(&mut self, slice: &str) {
        self.source.push_str(slice);
        if let Some(ref mut sm) = self.source_map {
            sm.add_to_column(slice.chars().count());
        }
    }

    fn push_str_contains_new_line(&mut self, slice: &str) {
        self.source.push_str(slice);
        if let Some(ref mut sm) = self.source_map {
            for chr in slice.chars() {
                if chr == '\n' {
                    sm.add_new_line()
                }
            }
        }
    }

    fn add_mapping(&mut self, source_span: &SpanWithSource) {
        if let Some(ref mut sm) = self.source_map {
            sm.add_mapping(source_span);
        }
    }

    fn halt(&self) -> bool {
        self.quit_after
            .map_or(false, |quit_after| self.source.len() > quit_after)
    }
}

/// Used for getting **byte count** of the result when built into string without building the string
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
        self.0 += string.chars().count();
    }

    fn push_str_contains_new_line(&mut self, string: &str) {
        self.0 += string.chars().count();
    }

    fn add_mapping(&mut self, _source_span: &SpanWithSource) {}
}

/// Counts text until a limit. Used for telling whether the text is greater than some threshold
pub struct MaxCounter {
    acc: usize,
    max: usize,
}

impl MaxCounter {
    pub fn new(max: usize) -> Self {
        Self { acc: 0, max }
    }

    /// TODO temp to see overshoot
    pub fn get_acc(&self) -> usize {
        self.acc
    }
}

impl ToString for MaxCounter {
    fn push(&mut self, chr: char) {
        self.acc += chr.len_utf8();
    }

    fn push_new_line(&mut self) {
        self.push('\n');
    }

    fn push_str(&mut self, string: &str) {
        self.acc += string.len();
    }

    fn push_str_contains_new_line(&mut self, string: &str) {
        self.acc += string.len();
    }

    fn add_mapping(&mut self, _source_span: &SpanWithSource) {}

    fn halt(&self) -> bool {
        self.acc > self.max
    }
}

#[cfg(test)]
mod to_string_tests {
    use super::*;

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
        assert_eq!(s.get_count(), "Hello World".chars().count());
    }

    #[test]
    fn max_counter() {
        let mut s = MaxCounter::new(14);
        serializer(&mut s);
        assert!(!s.halt());
        serializer(&mut s);
        assert!(s.halt());
    }
}
