#[cfg(feature = "self-rust-tokenize")]
#[test]
fn to_tokens() {
    use self_rust_tokenize::SelfRustTokenize;
    use source_map::{Nullable, SourceId, SpanWithSource};

    let span = SpanWithSource {
        start: 10,
        end: 20,
        source: SourceId::NULL,
    };

    assert_eq!(
        span.to_tokens().to_string(),
        "BaseSpan { start : 10u32 , end : 20u32 , source : CURRENT_SOURCE_ID , }"
    );
}
