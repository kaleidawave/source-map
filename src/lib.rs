use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU8, Ordering},
        RwLock,
    },
};

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

    /// **ONLY FOR TESTING METHODS**
    pub const fn null() -> Self {
        Self(0)
    }
}

/// A start and end line and column. Also contains trace of original source
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Span {
	pub line_start: usize, 
	pub column_start: usize, 
	pub line_end: usize, 
	pub column_end: usize, 
	pub source_id: SourceId
}

impl Span {
    /// Returns whether the end of `self` is the start of `other`
    pub fn is_adjacent_to(&self, other: &Self) -> bool {
        self.source_id == other.source_id && self.line_end == other.line_start && self.column_end == other.column_start
    }

    /// Returns a new [`Span`] which starts at the start of `self` a ends at the end of `other`
    pub fn union(&self, other: &Self) -> Span {
        Span {
            line_start: self.line_start, 
            column_start: self.column_start, 
            line_end: other.line_end, 
            column_end: other.column_end, 
            source_id: self.source_id.clone()
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

const BASE64_ALPHABET: &'static [u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Adapted from [vlq](https://github.com/Rich-Harris/vlq/blob/822db3f22bf09148b84e8ef58878d11f3bcd543e/src/vlq.ts#L63)
fn vlq_encode_integer_to_buffer(buf: &mut String, mut value: isize) {
    if value.is_negative() {
        value = (-value << 1) | 1;
    } else {
        value <<= 1;
    };

    loop {
        let mut clamped = value & 31;
        value >>= 5;
        if value > 0 {
            clamped |= 32;
        }
        buf.push(BASE64_ALPHABET[clamped as usize] as char);
        if value <= 0 {
            break;
        }
    }
}

/// Struct for building a [source map (v3)](https://sourcemaps.info/spec.html)
pub struct SourceMap {
    /// The mappings as String
    buf: String,
    /// Current line & column of the output
    line: u16,
    column: u8,
    /// The last line a mapping was added to. Used to decide whether to add segment separator ','
    last_line: Option<u16>,
    last_column: isize,
    /// The current position in source. Used for relativeness
    last_source_line: u16,
    last_source_column: isize,
    sources: Vec<(String, Option<String>)>,
    /// Maps source ids to position in sources vector
    sources_map: HashMap<SourceId, u8>,
}

impl SourceMap {
    pub fn new() -> Self {
        SourceMap {
            buf: String::new(),
            line: 0,
            last_line: None,
            column: 0,
            last_column: 0,
            last_source_line: 0,
            last_source_column: 0,
            sources: Vec::new(),
            sources_map: HashMap::new(),
        }
    }

    /// Original line and original column are one indexed
    pub fn add_mapping(
        &mut self,
        original_line: usize,
        original_column: usize,
        source_id: SourceId,
    ) {
        if let Some(ref mut last_line) = self.last_line {
            if *last_line == self.line {
                self.buf.push(',');
            }
            *last_line = self.line;
        } else {
            self.last_line = Some(self.line);
        }
        let buf = &mut self.buf;
        // Add column - self.last_column as isize
        let column_offset = self.column as isize - self.last_column as isize;
        vlq_encode_integer_to_buffer(buf, column_offset);
        // If the source in map
        if let Some(idx) = self.sources_map.get(&source_id) {
            vlq_encode_integer_to_buffer(buf, *idx as isize);
        } else {
            // Else get it from the global
            let source_name = SOURCE_IDS.read().unwrap().get(&source_id).unwrap().clone();
            // And add it to the map
            self.sources.push(source_name);
            let idx = (self.sources.len() - 1) as u8;
            self.sources_map.insert(source_id, idx);
            vlq_encode_integer_to_buffer(buf, idx as isize);
        }
        vlq_encode_integer_to_buffer(
            buf,
            original_line as isize - 1 - self.last_source_line as isize,
        );
        vlq_encode_integer_to_buffer(
            buf,
            original_column as isize - self.last_source_column as isize,
        );
        self.last_source_line = original_line as u16 - 1;
        self.last_source_column = original_column as isize;
        self.last_column = self.column as isize;
    }

    pub fn add_new_line(&mut self) {
        self.line += 1;
        self.buf.push(';');
        self.column = 0;
        self.last_column = 0;
    }

    pub fn add_to_column(&mut self, length: usize) {
        self.column += length as u8;
    }

    // TODO kinda temp
    pub fn add_source(&mut self, name: String, content: Option<String>) {
        self.sources.push((name, content))
    }

    pub fn to_string(self) -> String {
        let mut source_names = String::new();
        let mut source_contents = String::new();
        for (idx, (source_name, source_content)) in self.sources.iter().enumerate() {
            source_names.push('"');
            source_names.push_str(&source_name.replace('\\', "\\\\"));
            source_names.push('"');
            source_contents.push('"');
            if let Some(content) = &source_content {
                source_contents.push_str(&content.replace('\n', "\\n").replace('\r', ""));
            }
            source_contents.push('"');
            if idx < self.sources.len() - 1 {
                source_names.push(',');
                source_contents.push(',');
            }
        }
        format!(
            r#"{{"version":3,"sourceRoot":"","sources":[{}],"sourcesContent":[{}],"names":[],"mappings":"{}"}}"#,
            source_names, source_contents, self.buf
        )
    }
}

#[cfg(test)]
mod source_map_tests {
    use super::{vlq_encode_integer_to_buffer, Span, SourceId};

    fn vlq_encode_integer(value: isize) -> String {
        let mut buf = String::new();
        vlq_encode_integer_to_buffer(&mut buf, value);
        buf
    }

    #[test]
    fn vlq_encoder() {
        assert_eq!(vlq_encode_integer(0), "A");
        assert_eq!(vlq_encode_integer(1), "C");
        assert_eq!(vlq_encode_integer(-1), "D");
        assert_eq!(vlq_encode_integer(123), "2H");
        assert_eq!(vlq_encode_integer(123456789), "qxmvrH");
    }

    #[test]
    fn same_line() {
        let span = Span {
            line_start: 4,
            line_end: 5,
            column_start: 2,
            column_end: 2,
            source_id: SourceId::null()
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
            source_id: SourceId::null()
        };
        assert!(span.is_on_one_line());
        let span = Span {
            line_start: 4,
            line_end: 5,
            column_start: 2,
            column_end: 2,
            source_id: SourceId::null()
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
            source_id: SourceId::null()
        };
        assert!(span.is_adjacent_to(&Span {
            line_start: 1,
            line_end: 1,
            column_start: 3,
            column_end: 5,
            source_id: SourceId::null()
        }));
    }
}
