use crate::{
    count_characters_on_last_line, FileSystem, SourceMap, SourceMapBuilder, SpanWithSource,
};

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
    fn should_halt(&self) -> bool {
        false
    }

    fn characters_on_current_line(&self) -> u32;
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

    fn characters_on_current_line(&self) -> u32 {
        count_characters_on_last_line(self)
    }
}

pub struct Writable<T: std::io::Write> {
    pub writable: T,
    pub length: u32,
    pub since_new_line: u32,
    pub source_map: Option<SourceMapBuilder>,
}

impl<T: std::io::Write> ToString for Writable<T> {
    fn push(&mut self, chr: char) {
        let mut buf = [0u8; 4]; // A char can be at most 4 bytes in UTF-8
        let buf = chr.encode_utf8(&mut buf).as_bytes();
        let char_size = chr.len_utf8();
        self.length += char_size as u32;
        self.since_new_line += char_size as u32;
        self.writable.write_all(buf).unwrap();
    }

    fn push_new_line(&mut self) {
        self.length += 1;
        self.writable.write_all(&[b'\n']).unwrap();
    }

    fn push_str(&mut self, string: &str) {
        self.length += string.len() as u32;
        self.since_new_line += string.len() as u32;
        self.writable.write_all(string.as_bytes()).unwrap();
    }

    fn push_str_contains_new_line(&mut self, slice: &str) {
        self.length += slice.len() as u32;
        self.writable.write_all(slice.as_bytes()).unwrap();
        if let Some(ref mut sm) = self.source_map {
            slice
                .chars()
                .filter(|chr| *chr == '\n')
                .for_each(|_| sm.add_new_line());
        }
        self.since_new_line = count_characters_on_last_line(slice);
    }

    fn add_mapping(&mut self, source_span: &SpanWithSource) {
        if let Some(ref mut sm) = self.source_map {
            sm.add_mapping(source_span, self.since_new_line);
        }
    }

    fn characters_on_current_line(&self) -> u32 {
        self.since_new_line
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
    pub since_new_line: u32,
}

impl StringWithOptionalSourceMap {
    pub fn new(with_source_map: bool) -> Self {
        Self {
            source: String::new(),
            source_map: with_source_map.then(SourceMapBuilder::new),
            quit_after: None,
            since_new_line: 0,
        }
    }

    /// Returns output and the source map
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
            since_new_line: _,
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
            slice
                .chars()
                .filter(|chr| *chr == '\n')
                .for_each(|_| sm.add_new_line());
        }
        self.since_new_line = count_characters_on_last_line(slice);
    }

    fn add_mapping(&mut self, source_span: &SpanWithSource) {
        if let Some(ref mut sm) = self.source_map {
            sm.add_mapping(source_span, self.since_new_line);
        }
    }

    fn should_halt(&self) -> bool {
        self.quit_after
            .map_or(false, |quit_after| self.source.len() > quit_after)
    }

    fn characters_on_current_line(&self) -> u32 {
        self.since_new_line
    }
}

/// Counts text until a limit. Used for telling whether the text is greater than some threshold
pub struct Counter {
    acc: usize,
    max: usize,
}

impl Counter {
    pub fn new(max: usize) -> Self {
        Self { acc: 0, max }
    }

    pub fn get_count(&self) -> usize {
        self.acc
    }
}

impl ToString for Counter {
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

    fn should_halt(&self) -> bool {
        self.acc > self.max
    }

    fn characters_on_current_line(&self) -> u32 {
        // TODO?
        0
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
        let mut s = Counter::new(usize::MAX);
        serializer(&mut s);
        assert_eq!(s.get_count(), "Hello World".chars().count());
    }

    #[test]
    fn max_counter() {
        let mut s = Counter::new(14);
        serializer(&mut s);
        assert!(!s.should_halt());
        serializer(&mut s);
        assert!(s.should_halt());
    }
}
