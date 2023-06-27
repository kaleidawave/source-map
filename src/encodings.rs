pub trait StringEncoding {
    fn new() -> Self;

    fn get_encoded_length(string: &str) -> usize;

    fn encoded_length_to_byte_count(string: &str, length: usize) -> usize;
}

#[derive(Debug, PartialEq, Eq)]
pub struct ByteWiseEncoding;

impl StringEncoding for ByteWiseEncoding {
    fn new() -> Self {
        Self
    }

    fn get_encoded_length(string: &str) -> usize {
        string.len()
    }

    fn encoded_length_to_byte_count(_string: &str, length: usize) -> usize {
        length
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Utf8;

impl StringEncoding for Utf8 {
    fn new() -> Self {
        Self
    }

    fn get_encoded_length(string: &str) -> usize {
        string.chars().count()
    }

    fn encoded_length_to_byte_count(string: &str, length: usize) -> usize {
        string.chars().take(length).map(char::len_utf8).count()
    }
}
