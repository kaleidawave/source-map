use crate::FileSystem;
use std::{fmt, path::PathBuf};

/// A identifier for a [crate::Source]
#[derive(PartialEq, Eq, Clone, Copy, Hash)]
#[cfg_attr(feature = "serde-serialize", derive(serde::Serialize))]
#[cfg_attr(target_family = "wasm", derive(tsify::Tsify))]
pub struct SourceId(pub(crate) u16);

impl fmt::Debug for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("SourceId({})", self.0))
    }
}

impl SourceId {
    /// Returns a [SourceId] handle that references a file and its content
    pub fn new(filesystem: &mut impl FileSystem, path: PathBuf, content: String) -> Self {
        filesystem.new_source_id(path, content)
    }
}

#[cfg(feature = "self-rust-tokenize")]
pub const SPAN_TOKEN_IDENT: &str = "CURRENT_SOURCE_ID";

#[cfg(feature = "self-rust-tokenize")]
impl self_rust_tokenize::SelfRustTokenize for SourceId {
    fn append_to_token_stream(
        &self,
        token_stream: &mut self_rust_tokenize::proc_macro2::TokenStream,
    ) {
        let current_source_id_reference = self_rust_tokenize::proc_macro2::Ident::new(
            SPAN_TOKEN_IDENT,
            self_rust_tokenize::proc_macro2::Span::call_site(),
        );
        self_rust_tokenize::TokenStreamExt::append(token_stream, current_source_id_reference);
    }
}
