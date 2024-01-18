/// Given a file, removes whitespace and returns the result along with a source map
#[cfg(all(feature = "inline-source-map", feature = "global-source-filesystem"))]
fn main() {
    use source_map::{
        global_store::GlobalStore, FileSystem, SourceId, SpanWithSource, StringWithSourceMap,
        ToString,
    };
    use std::{env::args, fs};

    /// A simple string split, returns chunk plus byte indexes of chunk
    fn n_words_a_line(fs: &mut impl FileSystem, string: &str, output: &mut impl ToString) {
        let source_id = SourceId::new(fs, "file.txt".into(), string.to_owned());

        for (idx, chunk) in string
            .split(char::is_whitespace)
            .filter(|s| !s.is_empty())
            .enumerate()
        {
            // Compute the start position in the string using pointer offsets
            let start = chunk.as_ptr() as u32 - string.as_ptr() as u32;
            let base_span = SpanWithSource {
                start,
                end: start + chunk.len() as u32,
                source: source_id,
            };
            output.add_mapping(&base_span);
            output.push_str(chunk);
            output.push(' ');

            const WORDS_PER_LINE: usize = 5;
            if idx % WORDS_PER_LINE + 1 == WORDS_PER_LINE {
                output.push_new_line();
            }
        }
    }

    let mut source_map = StringWithSourceMap::new();

    let mut arguments = args().skip(1);
    let (input, output) = (
        arguments.next().expect("Expected input path argument"),
        arguments.next().expect("Expected output path argument"),
    );
    let file_as_string = fs::read_to_string(input).expect("Invalid path");

    let mut fs = GlobalStore;

    n_words_a_line(&mut fs, &file_as_string, &mut source_map);

    fs::write(output, source_map.build_with_inline_source_map(&fs)).expect("Write failed");
}

#[cfg(not(all(feature = "inline-source-map", feature = "global-source-filesystem")))]
fn main() {
    panic!("Enable 'inline-source-map' for this demo");
}
