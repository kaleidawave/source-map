mod source_id;
mod span;
mod to_string;

pub use source_id::SourceId;
pub use span::Span;
pub use to_string::{ToString, Counter, SourceMapBuilder};

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
    /// Maps source ids to position in sources vector
    sources_used: Vec<SourceId>,
}

impl SourceMap {
    pub fn new() -> SourceMap {
        SourceMap {
            buf: String::new(),
            line: 0,
            last_line: None,
            column: 0,
            last_column: 0,
            last_source_line: 0,
            last_source_column: 0,
            sources_used: Vec::new(),
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
        // Find idx or insert source and get id
        let idx = if let Some(idx) = self.sources_used.iter().position(|sid| *sid == source_id) {
            idx
        } else {
            self.sources_used.push(source_id);
            self.sources_used.len() - 1
        };
        // Encode index of source
        vlq_encode_integer_to_buffer(buf, idx as isize);

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

    pub fn to_string(self) -> String {
        let mut source_names = String::new();
        let mut source_contents = String::new();
        for (idx, source_id) in self.sources_used.iter().enumerate() {
            let (source_path, source_content) = source_id.get_file().unwrap();
            source_names.push('"');
            source_names.push_str(source_path.to_str().unwrap());
            source_names.push('"');

            source_contents.push('"');
            source_contents.push_str(&source_content.replace('\n', "\\n").replace('\r', ""));
            source_contents.push('"');

            if idx < self.sources_used.len() - 1 {
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
    use super::vlq_encode_integer_to_buffer;

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
}
