#![allow(clippy::useless_conversion)]

pub mod encodings;
mod filesystem;
mod lines_columns_indexes;
mod source_id;
mod span;
mod to_string;

use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
};

pub use filesystem::*;
pub use lines_columns_indexes::LineStarts;
pub use source_id::SourceId;
pub use span::{LineColumnPosition, LineColumnSpan, Position, Span};
pub use to_string::{Counter, StringWithSourceMap, ToString};

const BASE64_ALPHABET: &[u8; 64] =
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

#[derive(Debug)]
struct SourceMapping {
    pub(crate) on_output_column: usize,
    pub(crate) source_byte_start: usize,
    pub(crate) from_source: SourceId,
    // TODO are these needed
    // pub(crate) on_output_line: usize,
    // pub(crate) source_byte_end: usize,
}

#[derive(Debug)]
enum MappingOrBreak {
    Mapping(SourceMapping),
    Break,
}

/// Struct for building a [source map (v3)](https://sourcemaps.info/spec.html)
#[derive(Default)]
pub struct SourceMapBuilder {
    current_output_line: usize,
    current_output_column: usize,
    #[allow(dead_code)]
    last_output_line: Option<usize>,
    last_output_column: usize,
    mappings: Vec<MappingOrBreak>,
    used_sources: HashSet<SourceId>,
}

impl SourceMapBuilder {
    pub fn new() -> SourceMapBuilder {
        SourceMapBuilder::default()
    }

    // Record a new line was added to output
    pub fn add_new_line(&mut self) {
        self.current_output_line += 1;
        self.mappings.push(MappingOrBreak::Break);
        self.current_output_column = 0;
        self.last_output_column = 0;
    }

    // Record a new line was added to output
    pub fn add_to_column(&mut self, length: usize) {
        self.current_output_column += length;
    }

    /// Original line and original column are one indexed
    pub fn add_mapping(&mut self, source_position: &Span) {
        let Span {
            start: source_byte_start,
            // TODO should it read this
            end: _source_byte_end,
            source: from_source,
        } = source_position.clone();

        self.used_sources.insert(from_source);

        self.mappings.push(MappingOrBreak::Mapping(SourceMapping {
            from_source,
            source_byte_start: source_byte_start.try_into().unwrap(),
            on_output_column: self.current_output_column,
            // source_byte_end: *source_byte_end,
            // on_output_line: self.current_output_line,
        }));
    }

    /// Encodes the results into a string and builds the JSON representation thingy
    ///
    /// TODO not 100% certain that this code is a the correct implementation
    ///
    /// TODO are the accounts for SourceId::null valid here...?
    pub fn build(self, fs: &impl FileSystem) -> SourceMap {
        // Splits are indexes of new lines in the source
        let mut source_line_splits = HashMap::<SourceId, Vec<_>>::new();
        let mut sources = Vec::<SourceId>::new();

        for source_id in self.used_sources.into_iter().filter(|id| !id.is_null()) {
            let line_splits = fs.get_source_by_id(source_id, |source| source.line_starts.0.clone());

            source_line_splits.insert(source_id, line_splits);
            sources.push(source_id);
        }

        let mut mappings = String::new();

        let mut last_was_break = None::<bool>;
        let mut last_mapped_source_line = 0;
        let mut last_mapped_source_column = 0;
        let mut last_mapped_output_column = 0;

        for mapping in self.mappings {
            match mapping {
                MappingOrBreak::Mapping(mapping) => {
                    let SourceMapping {
                        on_output_column,
                        source_byte_start,
                        // TODO are these needed:
                        // on_output_line: _,
                        // source_byte_end: _,
                        from_source,
                    } = mapping;

                    if from_source.is_null() {
                        continue;
                    }

                    if let Some(false) = last_was_break {
                        mappings.push(',');
                    }

                    let output_column =
                        on_output_column as isize - last_mapped_output_column as isize;

                    vlq_encode_integer_to_buffer(&mut mappings, output_column);
                    last_mapped_output_column = on_output_column;

                    // Find index
                    // TODO faster
                    let idx = sources.iter().position(|sid| *sid == from_source).unwrap();

                    // Encode index of source
                    vlq_encode_integer_to_buffer(&mut mappings, idx as isize);

                    let line_splits_for_this_file = source_line_splits.get(&from_source).unwrap();

                    // TODO explain
                    let (source_line, source_column) = match line_splits_for_this_file.as_slice() {
                        [] => (0, source_byte_start),
                        [split] => {
                            if source_byte_start < *split {
                                (0, source_byte_start)
                            } else {
                                (1, source_byte_start - split)
                            }
                        }
                        splits => {
                            if source_byte_start < *splits.first().unwrap() {
                                (0, source_byte_start)
                            } else if source_byte_start > *splits.last().unwrap() {
                                (splits.len(), source_byte_start - splits.last().unwrap())
                            } else {
                                let splits =
                                    splits.windows(2).enumerate().find_map(|(line, window)| {
                                        if let [floor, ceil] = window {
                                            if *floor < source_byte_start
                                                && source_byte_start <= *ceil
                                            {
                                                Some((line + 1, source_byte_start - floor - 1))
                                            } else {
                                                None
                                            }
                                        } else {
                                            unreachable!()
                                        }
                                    });

                                if let Some(splits) = splits {
                                    splits
                                } else {
                                    eprintln!("Didn't find ...");
                                    (0, source_byte_start)
                                }
                            }
                        }
                    };

                    let source_line_diff = source_line as isize - last_mapped_source_line as isize;
                    vlq_encode_integer_to_buffer(&mut mappings, source_line_diff);

                    last_mapped_source_line = source_line;

                    let source_column_diff =
                        source_column as isize - last_mapped_source_column as isize;
                    vlq_encode_integer_to_buffer(&mut mappings, source_column_diff);

                    last_mapped_source_column = source_column;

                    // TODO names field?

                    last_was_break = Some(false);
                }
                MappingOrBreak::Break => {
                    mappings.push(';');
                    last_was_break = Some(true);
                    last_mapped_output_column = 0;
                }
            }
        }

        SourceMap { mappings, sources }
    }
}

#[derive(Clone)]
pub struct SourceMap {
    pub mappings: String,
    pub sources: Vec<SourceId>,
}

impl SourceMap {
    pub fn to_json(self, filesystem: &impl FileSystem) -> String {
        use std::fmt::Write;

        let Self {
            mappings,
            sources: sources_used,
        } = self;

        let (mut sources, mut sources_content) = (String::new(), String::new());
        for (idx, (path, content)) in sources_used
            .into_iter()
            .map(|source_id| filesystem.get_file_path_and_content(source_id))
            .enumerate()
        {
            if idx != 0 {
                sources.push(',');
                sources_content.push(',');
            }
            write!(
                sources,
                "\"{}\"",
                path.display().to_string().replace('\\', "/")
            )
            .unwrap();
            write!(
                sources_content,
                "\"{}\"",
                content
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('"', "\\\"")
            )
            .unwrap();
        }

        format!(
            r#"{{"version":3,"sourceRoot":"","sources":[{sources}],"sourcesContent":[{sources_content}],"names":[],"mappings":"{mappings}"}}"#,
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
