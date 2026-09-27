use std::sync::LazyLock;

use tree_sitter::Query;

use crate::lang::LanguageId;

#[derive(Debug)]
pub(crate) struct Table {
    pub(crate) id: LanguageId,
    pub(crate) decorators_before: &'static [&'static str],
    pub(crate) decorators_within: &'static [&'static str],
    pub(crate) decorators_direct: &'static [&'static str],
    pub(crate) annotation: &'static str,
    pub(crate) inert_decorators: &'static [&'static str],
    pub(crate) public_by_default: bool,
    pub(crate) visibility_modifiers: &'static [&'static str],
    pub(crate) public_modifiers: &'static [&'static str],
    pub(crate) restricting_modifiers: &'static [&'static str],
    pub(crate) public_inside: &'static [&'static str],
    pub(crate) public_wrappers: &'static [&'static str],
    pub(crate) public_transparents: &'static [&'static str],
    pub(crate) owner_bodies: &'static [&'static str],
    pub(crate) owner_declarations: &'static [&'static str],
    pub(crate) owner_field: &'static str,
    pub(crate) owner_name_wrappers: &'static [&'static str],
    references: LazyLock<Query>,
}

static KOTLIN: Table = Table {
    id: LanguageId::Kotlin,
    decorators_before: &[],
    decorators_within: &["modifiers"],
    decorators_direct: &[],
    annotation: "annotation",
    inert_decorators: &[],
    public_by_default: true,
    visibility_modifiers: &["visibility_modifier"],
    public_modifiers: &[],
    restricting_modifiers: &["private", "internal", "protected"],
    public_inside: &[],
    public_wrappers: &[],
    public_transparents: &[],
    owner_bodies: &["class_body"],
    owner_declarations: &["class_declaration", "object_declaration"],
    owner_field: "name",
    owner_name_wrappers: &[],
    references: LazyLock::new(|| {
        LanguageId::Kotlin
            .spec()
            .query("references", include_str!("queries/kotlin/references.scm"))
    }),
};

static RUST: Table = Table {
    id: LanguageId::Rust,
    decorators_before: &["attribute_item"],
    decorators_within: &[],
    decorators_direct: &[],
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
    visibility_modifiers: &["visibility_modifier"],
    public_modifiers: &["pub"],
    restricting_modifiers: &[],
    public_inside: &[],
    public_wrappers: &[],
    public_transparents: &[],
    owner_bodies: &["declaration_list"],
    owner_declarations: &["impl_item"],
    owner_field: "type",
    owner_name_wrappers: &["generic_type"],
    references: LazyLock::new(|| {
        LanguageId::Rust
            .spec()
            .query("references", include_str!("queries/rust/references.scm"))
    }),
};

static TYPESCRIPT: Table = Table {
    id: LanguageId::TypeScript,
    decorators_before: &["decorator"],
    decorators_within: &[],
    decorators_direct: &["decorator"],
    annotation: "decorator",
    inert_decorators: &[],
    public_by_default: false,
    visibility_modifiers: &["accessibility_modifier"],
    public_modifiers: &[],
    restricting_modifiers: &["private", "protected"],
    public_inside: &["class_body", "interface_body"],
    public_wrappers: &["export_statement"],
    public_transparents: &["lexical_declaration", "variable_declaration"],
    owner_bodies: &["class_body", "interface_body"],
    owner_declarations: &[
        "class_declaration",
        "abstract_class_declaration",
        "interface_declaration",
    ],
    owner_field: "name",
    owner_name_wrappers: &[],
    references: LazyLock::new(|| {
        LanguageId::TypeScript.spec().query(
            "references",
            include_str!("queries/typescript/references.scm"),
        )
    }),
};

pub(crate) fn table(language: LanguageId) -> &'static Table {
    match language {
        LanguageId::Kotlin => &KOTLIN,
        LanguageId::Rust => &RUST,
        LanguageId::TypeScript => &TYPESCRIPT,
    }
}

impl Table {
    pub(crate) fn references(&self) -> &Query {
        &self.references
    }
}
