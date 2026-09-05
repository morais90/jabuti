use std::sync::OnceLock;

use tree_sitter::Query;

use crate::lang::LanguageId;

#[derive(Debug)]
pub(crate) struct Table {
    pub(crate) id: LanguageId,
    pub(crate) decorators_before: &'static [&'static str],
    pub(crate) decorators_within: &'static [&'static str],
    pub(crate) annotation: &'static str,
    pub(crate) inert_decorators: &'static [&'static str],
    pub(crate) public_by_default: bool,
    pub(crate) restricting_modifiers: &'static [&'static str],
    references_source: &'static str,
    compiled: OnceLock<Query>,
}

static KOTLIN: Table = Table {
    id: LanguageId::Kotlin,
    decorators_before: &[],
    decorators_within: &["modifiers"],
    annotation: "annotation",
    inert_decorators: &[],
    public_by_default: true,
    restricting_modifiers: &["private", "internal", "protected"],
    references_source: include_str!("queries/kotlin/references.scm"),
    compiled: OnceLock::new(),
};

static RUST: Table = Table {
    id: LanguageId::Rust,
    decorators_before: &["attribute_item"],
    decorators_within: &[],
    annotation: "attribute",
    inert_decorators: &[
        "allow",
        "automatically_derived",
        "cfg",
        "cfg_attr",
        "cold",
        "deny",
        "deprecated",
        "derive",
        "doc",
        "expect",
        "forbid",
        "inline",
        "must_use",
        "non_exhaustive",
        "repr",
        "track_caller",
        "warn",
    ],
    public_by_default: false,
    restricting_modifiers: &[],
    references_source: include_str!("queries/rust/references.scm"),
    compiled: OnceLock::new(),
};

pub(crate) fn table(language: LanguageId) -> &'static Table {
    match language {
        LanguageId::Kotlin => &KOTLIN,
        LanguageId::Rust => &RUST,
    }
}

impl Table {
    pub(crate) fn references(&self) -> &Query {
        self.compiled
            .get_or_init(|| self.id.spec().query("references", self.references_source))
    }
}
