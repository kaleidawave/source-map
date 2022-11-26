# Source map

[![](https://img.shields.io/crates/v/source-map)](https://crates.io/crates/source-map)

Utilities for building source maps (v3) for a compiler and handling source location representations

### Includes:
- `Span`, a structure which represents a section of a specific source
- `SourceId`, a identifier for a source file
- `StringWithSourceMap`, along with the `ToString` trait makes generating string representations with and adding source markings trivial
- With the `lsp-types-morphisms` features makes converting between the position information in [lsp-types](https://docs.rs/crate/lsp-types/latest) easy

### Example:

```
git clone https://github.com/kaleidawave/source-map
cd source-map
cargo run -F inline-source-map --example source_map_creation -- LICENSE LICENSE.map
```

View bindings by uploading `LICENSE.map` with https://evanw.github.io/source-map-visualization/