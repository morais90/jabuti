use std::path::Path;
use std::sync::OnceLock;

use tree_sitter::{Language, Query};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LanguageId {
    Kotlin,
    Rust,
}

impl LanguageId {
    pub fn name(self) -> &'static str {
        match self {
            Self::Kotlin => "kotlin",
            Self::Rust => "rust",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        ALL.iter().map(|spec| spec.id).find(|id| id.name() == name)
    }
}

#[derive(Debug)]
pub(crate) struct Queries {
    pub(crate) language: Language,
    pub(crate) units: Query,
    pub(crate) comments: Query,
    pub(crate) decisions: Query,
    pub(crate) masking: Query,
}

#[derive(Debug)]
pub struct CognitiveSpec {
    pub(crate) conditional: &'static str,
    pub(crate) condition_field: &'static str,
    pub(crate) alternative_wrapper: &'static str,
    pub(crate) nesting_increments: &'static [&'static str],
    pub(crate) nesting_only: &'static [&'static str],
    pub(crate) logical_expression: &'static str,
    pub(crate) operator_field: &'static str,
    pub(crate) logical_operators: &'static [&'static str],
    pub(crate) boundaries: &'static [&'static str],
}

#[derive(Debug)]
pub struct LangSpec {
    pub id: LanguageId,
    pub grammar_version: &'static str,
    pub extensions: &'static [&'static str],
    pub(crate) implicit_parameters: &'static [&'static str],
    pub(crate) metadata_nodes: &'static [&'static str],
    pub(crate) decorators_before: &'static [&'static str],
    pub(crate) decorators_within: &'static [&'static str],
    pub(crate) test_markers: &'static [&'static str],
    pub(crate) test_paths: &'static [&'static str],
    pub(crate) cognitive: CognitiveSpec,
    units_source: &'static str,
    comments_source: &'static str,
    decisions_source: &'static str,
    masking_source: &'static str,
    grammar: fn() -> Language,
    compiled: OnceLock<Queries>,
}

impl LangSpec {
    pub fn knows_node_kind(&self, kind: &str, named: bool) -> bool {
        !kind.is_empty() && self.queries().language.id_for_node_kind(kind, named) != 0
    }

    pub fn knows_field(&self, field: &str) -> bool {
        self.queries().language.field_id_for_name(field).is_some()
    }

    pub fn declared_node_kinds(&self) -> Vec<(&'static str, bool)> {
        let cognitive = &self.cognitive;
        let named = [cognitive.conditional, cognitive.logical_expression]
            .into_iter()
            .chain(cognitive.nesting_increments.iter().copied())
            .chain(cognitive.nesting_only.iter().copied())
            .chain(cognitive.boundaries.iter().copied())
            .chain(self.implicit_parameters.iter().copied())
            .chain(some(cognitive.alternative_wrapper))
            .map(|kind| (kind, true));

        named
            .chain(cognitive.logical_operators.iter().map(|op| (*op, false)))
            .collect()
    }

    pub fn declared_fields(&self) -> Vec<&'static str> {
        vec![
            self.cognitive.condition_field,
            self.cognitive.operator_field,
        ]
    }

    pub(crate) fn queries(&self) -> &Queries {
        self.compiled.get_or_init(|| {
            let language = (self.grammar)();
            let units = compile(&language, self.units_source, self.id, "units");
            let comments = compile(&language, self.comments_source, self.id, "comments");
            let decisions = compile(&language, self.decisions_source, self.id, "decisions");
            let masking = compile(&language, self.masking_source, self.id, "masking");

            Queries {
                language,
                units,
                comments,
                decisions,
                masking,
            }
        })
    }
}

fn some(kind: &'static str) -> Option<&'static str> {
    (!kind.is_empty()).then_some(kind)
}

fn compile(language: &Language, source: &str, id: LanguageId, name: &str) -> Query {
    Query::new(language, source)
        .unwrap_or_else(|error| panic!("{id:?} {name} query does not compile: {error}"))
}

fn kotlin_grammar() -> Language {
    tree_sitter_kotlin_ng::LANGUAGE.into()
}

pub static KOTLIN: LangSpec = LangSpec {
    id: LanguageId::Kotlin,
    grammar_version: "1.1.0",
    extensions: &["kt", "kts"],
    implicit_parameters: &[],
    metadata_nodes: &["annotation", "modifiers"],
    decorators_before: &[],
    decorators_within: &["modifiers", "annotation"],
    test_markers: &["@Test", "@ParameterizedTest", "@RepeatedTest"],
    test_paths: &[
        "/src/test/",
        "/src/androidTest/",
        "/src/commonTest/",
        "/src/jvmTest/",
    ],
    cognitive: CognitiveSpec {
        conditional: "if_expression",
        condition_field: "condition",
        alternative_wrapper: "",
        nesting_increments: &[
            "when_expression",
            "while_statement",
            "do_while_statement",
            "for_statement",
            "catch_block",
        ],
        nesting_only: &["lambda_literal"],
        logical_expression: "binary_expression",
        operator_field: "operator",
        logical_operators: &["&&", "||"],
        boundaries: &["function_declaration"],
    },
    units_source: include_str!("../queries/kotlin/units.scm"),
    comments_source: include_str!("../queries/kotlin/comments.scm"),
    decisions_source: include_str!("../queries/kotlin/decisions.scm"),
    masking_source: include_str!("../queries/kotlin/masking.scm"),
    grammar: kotlin_grammar,
    compiled: OnceLock::new(),
};

fn rust_grammar() -> Language {
    tree_sitter_rust::LANGUAGE.into()
}

pub static RUST: LangSpec = LangSpec {
    id: LanguageId::Rust,
    grammar_version: "0.24.2",
    extensions: &["rs"],
    implicit_parameters: &["self_parameter", "attribute_item"],
    metadata_nodes: &["attribute_item", "inner_attribute_item"],
    decorators_before: &["attribute_item"],
    decorators_within: &["inner_attribute_item"],
    test_markers: &["#[test]", "#[bench]", "#[rstest]", "cfg(test)"],
    test_paths: &["/tests/", "/benches/", "/examples/"],
    cognitive: CognitiveSpec {
        conditional: "if_expression",
        condition_field: "condition",
        alternative_wrapper: "else_clause",
        nesting_increments: &[
            "match_expression",
            "while_expression",
            "for_expression",
            "loop_expression",
        ],
        nesting_only: &["closure_expression"],
        logical_expression: "binary_expression",
        operator_field: "operator",
        logical_operators: &["&&", "||"],
        boundaries: &["function_item"],
    },
    units_source: include_str!("../queries/rust/units.scm"),
    comments_source: include_str!("../queries/rust/comments.scm"),
    decisions_source: include_str!("../queries/rust/decisions.scm"),
    masking_source: include_str!("../queries/rust/masking.scm"),
    grammar: rust_grammar,
    compiled: OnceLock::new(),
};

pub static ALL: &[&LangSpec] = &[&KOTLIN, &RUST];

impl LangSpec {
    pub fn is_test_path(&self, path: &Path) -> bool {
        let shown = path.to_string_lossy().replace('\\', "/");

        self.test_paths.iter().any(|marker| shown.contains(marker))
    }
}

pub fn detect(path: &Path) -> Option<&'static LangSpec> {
    let extension = path.extension()?.to_str()?;
    ALL.iter()
        .copied()
        .find(|spec| spec.extensions.contains(&extension))
}
