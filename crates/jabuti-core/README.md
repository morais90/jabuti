# jabuti-core

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/williandmorais/jabuti/blob/main/LICENSE)

The analysis engine behind [jabuti](https://github.com/williandmorais/jabuti), a code verification
tool built for AI coding agents.

This crate parses source with [tree-sitter](https://tree-sitter.github.io/) and exposes it as a
nested tree of units — file, module, type, function, closure — then measures that tree.

```rust
use jabuti_core::{lang, metrics::LineIndex, syntax};

let source = std::fs::read_to_string("src/main.rs")?;
let parsed = syntax::parse(&source, &lang::RUST)?;

let file = parsed.units();
let lines = LineIndex::new(&source, &parsed.comment_ranges());

println!("{:?}", lines.loc(file.span));
# Ok::<(), Box<dyn std::error::Error>>(())
```

Two decisions shape the API.

**Language knowledge is data, not code.** What counts as a function, a comment or a branch is
declared in tree-sitter queries, and one shared algorithm consumes them. Supporting another language
means adding query files, not writing another analyzer.

**Source that does not parse cleanly is rejected.** tree-sitter returns a tree even for broken
input, so measuring without checking would report confident numbers computed over rubble. `parse`
returns an error instead, and the caller decides what to do about it.

Analyses share one parse: `parse` hands back the parsed source, and the unit tree, comment ranges
and everything downstream are derived from it.

## Status

Early, and the API will change. Line counting is in place; complexity, structural and process
metrics are being added in that order.

## License

Apache-2.0.
