use std::sync::OnceLock;

use tree_sitter::Query;

use crate::lang::LanguageId;

#[derive(Debug)]
struct Table {
    id: LanguageId,
    references_source: &'static str,
    compiled: OnceLock<Query>,
}

static KOTLIN: Table = Table {
    id: LanguageId::Kotlin,
    references_source: include_str!("queries/kotlin/references.scm"),
    compiled: OnceLock::new(),
};

static RUST: Table = Table {
    id: LanguageId::Rust,
    references_source: include_str!("queries/rust/references.scm"),
    compiled: OnceLock::new(),
};

pub(crate) fn references(language: LanguageId) -> &'static Query {
    let table = match language {
        LanguageId::Kotlin => &KOTLIN,
        LanguageId::Rust => &RUST,
    };

    table
        .compiled
        .get_or_init(|| table.id.spec().query("references", table.references_source))
}
