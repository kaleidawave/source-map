use source_map::{SourceId, Span, StringWithSourceMap, ToString};

/// A simple string split, returns chunk plus byte indexes of chunk
mod split_indices {
    use std::{ops::Range, str::CharIndices};

    pub struct SplitIndices<'a, T: Fn(char) -> bool> {
        pub string: &'a str,
        pub function: T,
        pub last_match: usize,
        pub char_iterator: CharIndices<'a>,
        pub exhausted: bool,
    }

    pub fn split_indices_from_str<'a, T: Fn(char) -> bool>(
        string: &'a str,
        function: T,
    ) -> SplitIndices<'a, T> {
        SplitIndices {
            string,
            function,
            last_match: 0,
            char_iterator: string.char_indices(),
            exhausted: false,
        }
    }

    impl<'a, T: Fn(char) -> bool> Iterator for SplitIndices<'a, T> {
        type Item = (Range<usize>, &'a str);

        fn next(&mut self) -> Option<Self::Item> {
            if self.exhausted {
                return None;
            }
            let Self {
                char_iterator,
                function,
                ..
            } = self;

            let find_map = char_iterator
                .by_ref()
                .find_map(|(idx, char)| (function)(char).then(|| (idx, char)));

            if let Some((idx, char)) = find_map {
                let start = self.last_match;
                let end = idx + char.len_utf8();
                self.last_match = end;
                let range = start..idx;
                Some((range.clone(), &self.string[range]))
            } else {
                let range = self.last_match..self.string.len();
                self.exhausted = true;
                Some((range.clone(), &self.string[range]))
            }
        }
    }
}

/// Given a file, removes whitespace and returns the result along with a source map
fn main() {
    use split_indices::split_indices_from_str;
    use std::fs;

    fn remove_whitespace(string: &str, output: &mut impl ToString) {
        let source_id = SourceId::new("file.txt".into(), string.to_owned());

        for (range, chunk) in split_indices_from_str(string, char::is_whitespace) {
            if !chunk.is_empty() {
                output.add_mapping(&Span {
                    start: range.start,
                    end: range.end,
                    source_id,
                });
                output.push_str(chunk);
            }
        }
    }

    let mut source_map = StringWithSourceMap::new();
    remove_whitespace(&fs::read_to_string("file.txt").unwrap(), &mut source_map);
    let output = source_map.build_with_inline_source_map();
    fs::write("file-out.txt", output).unwrap();
}
