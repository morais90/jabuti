use std::path::Path;
use std::sync::OnceLock;

use tree_sitter::Query;

use crate::lang::LanguageId;

#[derive(Debug)]
pub(crate) struct Queries {
    pub(crate) units: Query,
    pub(crate) comments: Query,
    pub(crate) decisions: Query,
    pub(crate) masking: Query,
}

#[derive(Debug)]
pub(crate) struct CognitiveSpec {
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
pub(crate) struct Table {
    id: LanguageId,
    pub(crate) implicit_parameters: &'static [&'static str],
    pub(crate) metadata_nodes: &'static [&'static str],
    pub(crate) decorators_before: &'static [&'static str],
    pub(crate) decorators_within: &'static [&'static str],
    pub(crate) test_markers: &'static [&'static str],
    test_paths: &'static [&'static str],
    pub(crate) cognitive: CognitiveSpec,
    units_source: &'static str,
    comments_source: &'static str,
    decisions_source: &'static str,
    masking_source: &'static str,
    compiled: OnceLock<Queries>,
}

impl Table {
    pub(crate) fn queries(&self) -> &Queries {
        self.compiled.get_or_init(|| {
            let spec = self.id.spec();

            Queries {
                units: spec.query("units", self.units_source),
                comments: spec.query("comments", self.comments_source),
                decisions: spec.query("decisions", self.decisions_source),
                masking: spec.query("masking", self.masking_source),
            }
        })
    }
}

static KOTLIN: Table = Table {
    id: LanguageId::Kotlin,
    implicit_parameters: &[],
    metadata_nodes: &["annotation", "modifiers"],
    decorators_before: &[],
    decorators_within: &["modifiers", "annotation"],
    test_markers: &["@Test", "@ParameterizedTest", "@RepeatedTest"],
    test_paths: &["test", "tests", "*Test"],
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
    units_source: include_str!("queries/kotlin/units.scm"),
    comments_source: include_str!("queries/kotlin/comments.scm"),
    decisions_source: include_str!("queries/kotlin/decisions.scm"),
    masking_source: include_str!("queries/kotlin/masking.scm"),
    compiled: OnceLock::new(),
};

static RUST: Table = Table {
    id: LanguageId::Rust,
    implicit_parameters: &["self_parameter", "attribute_item"],
    metadata_nodes: &["attribute_item", "inner_attribute_item"],
    decorators_before: &["attribute_item"],
    decorators_within: &["inner_attribute_item"],
    test_markers: &[
        "test]",
        "[test(",
        "::test(",
        "bench]",
        "[bench(",
        "::bench(",
        "rstest(",
        "test_case",
        "cfg(test)",
    ],
    test_paths: &["tests", "benches", "examples"],
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
    units_source: include_str!("queries/rust/units.scm"),
    comments_source: include_str!("queries/rust/comments.scm"),
    decisions_source: include_str!("queries/rust/decisions.scm"),
    masking_source: include_str!("queries/rust/masking.scm"),
    compiled: OnceLock::new(),
};

pub(crate) fn table(language: LanguageId) -> &'static Table {
    match language {
        LanguageId::Kotlin => &KOTLIN,
        LanguageId::Rust => &RUST,
    }
}

pub fn declared_node_kinds(language: LanguageId) -> Vec<(&'static str, bool)> {
    let table = table(language);
    let cognitive = &table.cognitive;
    let named = [cognitive.conditional, cognitive.logical_expression]
        .into_iter()
        .chain(cognitive.nesting_increments.iter().copied())
        .chain(cognitive.nesting_only.iter().copied())
        .chain(cognitive.boundaries.iter().copied())
        .chain(table.implicit_parameters.iter().copied())
        .chain(some(cognitive.alternative_wrapper))
        .map(|kind| (kind, true));

    named
        .chain(cognitive.logical_operators.iter().map(|op| (*op, false)))
        .collect()
}

pub fn declared_fields(language: LanguageId) -> Vec<&'static str> {
    let cognitive = &table(language).cognitive;

    vec![cognitive.condition_field, cognitive.operator_field]
}

fn some(kind: &'static str) -> Option<&'static str> {
    (!kind.is_empty()).then_some(kind)
}

pub fn is_test_path(language: LanguageId, path: &Path) -> bool {
    path.components()
        .filter_map(|component| component.as_os_str().to_str())
        .any(|directory| {
            table(language)
                .test_paths
                .iter()
                .any(|name| names_match(name, directory))
        })
}

fn names_match(pattern: &str, directory: &str) -> bool {
    match pattern.strip_prefix('*') {
        Some(suffix) => directory.ends_with(suffix),
        None => directory == pattern,
    }
}
