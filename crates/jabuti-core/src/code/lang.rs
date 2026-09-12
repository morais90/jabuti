use std::sync::LazyLock;

use tree_sitter::Query;

use crate::lang::LanguageId;
use crate::model::Concept;

#[derive(Debug)]
pub(crate) struct Queries {
    pub(crate) units: Query,
    pub(crate) comments: Query,
    pub(crate) decisions: Query,
    pub(crate) concepts: Query,
}

struct QuerySources {
    units: &'static str,
    comments: &'static str,
    decisions: &'static str,
    concepts: &'static str,
}

#[derive(Debug)]
pub(crate) struct ConditionalSpec {
    pub(crate) kind: &'static str,
    pub(crate) alternative_wrapper: &'static str,
    pub(crate) chains_alternative: bool,
    pub(crate) charge_alternative: bool,
}

#[derive(Debug)]
pub(crate) struct ContextualBoundary {
    pub(crate) kind: &'static str,
    pub(crate) parents: &'static [&'static str],
}

#[derive(Debug)]
pub(crate) struct CognitiveSpec {
    pub(crate) conditionals: &'static [ConditionalSpec],
    pub(crate) condition_field: &'static str,
    pub(crate) nesting_increments: &'static [&'static str],
    pub(crate) nesting_only: &'static [&'static str],
    pub(crate) logical_expression: &'static str,
    pub(crate) operator_field: &'static str,
    pub(crate) logical_operators: &'static [&'static str],
    pub(crate) boundaries: &'static [&'static str],
    pub(crate) contextual_boundaries: &'static [ContextualBoundary],
}

#[derive(Debug)]
pub(crate) struct Table {
    pub(crate) id: LanguageId,
    pub(crate) implicit_parameters: &'static [&'static str],
    pub(crate) parameter_containers: &'static [&'static str],
    pub(crate) implicit_parameter_patterns: &'static [&'static str],
    pub(crate) metadata_nodes: &'static [&'static str],
    pub(crate) decorators_before: &'static [&'static str],
    pub(crate) decorators_within: &'static [&'static str],
    pub(crate) concepts: &'static [Concept],
    pub(crate) language_specific_rules: &'static [crate::model::Rule],
    pub(crate) path_separator: &'static str,
    pub(crate) cognitive: CognitiveSpec,
    queries: LazyLock<Queries>,
}

impl Table {
    pub(crate) fn queries(&self) -> &Queries {
        &self.queries
    }
}

fn compile_queries(language: LanguageId, sources: &QuerySources) -> Queries {
    let spec = language.spec();

    Queries {
        units: spec.query("units", sources.units),
        comments: spec.query("comments", sources.comments),
        decisions: spec.query("decisions", sources.decisions),
        concepts: spec.query("concepts", sources.concepts),
    }
}

static KOTLIN: Table = Table {
    id: LanguageId::Kotlin,
    implicit_parameters: &[],
    parameter_containers: &["function_value_parameters"],
    implicit_parameter_patterns: &[],
    metadata_nodes: &["annotation", "modifiers"],
    decorators_before: &[],
    decorators_within: &["modifiers", "annotation"],
    concepts: &Concept::ALL,
    language_specific_rules: &[],
    path_separator: ".",
    cognitive: CognitiveSpec {
        conditionals: &[ConditionalSpec {
            kind: "if_expression",
            alternative_wrapper: "",
            charge_alternative: true,
            chains_alternative: true,
        }],
        condition_field: "condition",
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
        contextual_boundaries: &[],
        boundaries: &["function_declaration"],
    },
    queries: LazyLock::new(|| {
        compile_queries(
            LanguageId::Kotlin,
            &QuerySources {
                units: include_str!("queries/kotlin/units.scm"),
                comments: include_str!("queries/kotlin/comments.scm"),
                decisions: include_str!("queries/kotlin/decisions.scm"),
                concepts: include_str!("queries/kotlin/concepts.scm"),
            },
        )
    }),
};

static RUST: Table = Table {
    id: LanguageId::Rust,
    implicit_parameters: &["self_parameter", "attribute_item"],
    parameter_containers: &["parameters", "closure_parameters"],
    implicit_parameter_patterns: &[],
    metadata_nodes: &["attribute_item", "inner_attribute_item"],
    decorators_before: &["attribute_item"],
    decorators_within: &["inner_attribute_item"],
    concepts: &Concept::ALL,
    language_specific_rules: &[],
    path_separator: "::",
    cognitive: CognitiveSpec {
        conditionals: &[ConditionalSpec {
            kind: "if_expression",
            alternative_wrapper: "else_clause",
            charge_alternative: true,
            chains_alternative: true,
        }],
        condition_field: "condition",
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
        contextual_boundaries: &[],
        boundaries: &["function_item"],
    },
    queries: LazyLock::new(|| {
        compile_queries(
            LanguageId::Rust,
            &QuerySources {
                units: include_str!("queries/rust/units.scm"),
                comments: include_str!("queries/rust/comments.scm"),
                decisions: include_str!("queries/rust/decisions.scm"),
                concepts: include_str!("queries/rust/concepts.scm"),
            },
        )
    }),
};

static TYPESCRIPT: Table = Table {
    id: LanguageId::TypeScript,
    implicit_parameters: &[],
    parameter_containers: &["formal_parameters"],
    implicit_parameter_patterns: &["this"],
    metadata_nodes: &["decorator"],
    decorators_before: &[],
    decorators_within: &["decorator"],
    concepts: &[Concept::ErrorSwallow, Concept::Suppression],
    language_specific_rules: &[],
    path_separator: ".",
    cognitive: CognitiveSpec {
        conditionals: &[
            ConditionalSpec {
                kind: "if_statement",
                alternative_wrapper: "else_clause",
                charge_alternative: true,
                chains_alternative: true,
            },
            ConditionalSpec {
                kind: "ternary_expression",
                alternative_wrapper: "",
                charge_alternative: false,
                chains_alternative: false,
            },
        ],
        condition_field: "condition",
        nesting_increments: &[
            "switch_statement",
            "while_statement",
            "do_statement",
            "for_statement",
            "for_in_statement",
            "catch_clause",
        ],
        nesting_only: &[
            "arrow_function",
            "function_expression",
            "generator_function",
        ],
        logical_expression: "binary_expression",
        operator_field: "operator",
        logical_operators: &["&&", "||", "??"],
        boundaries: &[
            "function_declaration",
            "generator_function_declaration",
            "method_definition",
            "function_signature",
            "method_signature",
            "abstract_method_signature",
        ],
        contextual_boundaries: &[
            ContextualBoundary {
                kind: "arrow_function",
                parents: &["variable_declarator", "public_field_definition", "pair"],
            },
            ContextualBoundary {
                kind: "function_expression",
                parents: &["variable_declarator"],
            },
            ContextualBoundary {
                kind: "generator_function",
                parents: &["variable_declarator"],
            },
        ],
    },
    queries: LazyLock::new(|| {
        compile_queries(
            LanguageId::TypeScript,
            &QuerySources {
                units: include_str!("queries/typescript/units.scm"),
                comments: include_str!("queries/typescript/comments.scm"),
                decisions: include_str!("queries/typescript/decisions.scm"),
                concepts: include_str!("queries/typescript/concepts.scm"),
            },
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

pub fn declared_node_kinds(language: LanguageId) -> Vec<(&'static str, bool)> {
    let table = table(language);
    let cognitive = &table.cognitive;
    let named = cognitive
        .conditionals
        .iter()
        .map(|conditional| conditional.kind)
        .chain([cognitive.logical_expression])
        .chain(cognitive.nesting_increments.iter().copied())
        .chain(cognitive.nesting_only.iter().copied())
        .chain(cognitive.contextual_boundaries.iter().flat_map(|boundary| {
            std::iter::once(boundary.kind).chain(boundary.parents.iter().copied())
        }))
        .chain(cognitive.boundaries.iter().copied())
        .chain(table.implicit_parameters.iter().copied())
        .chain(table.parameter_containers.iter().copied())
        .chain(table.implicit_parameter_patterns.iter().copied())
        .chain(
            cognitive
                .conditionals
                .iter()
                .filter_map(|conditional| some(conditional.alternative_wrapper)),
        )
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
