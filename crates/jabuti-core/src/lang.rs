use std::path::Path;
use std::sync::OnceLock;

use tree_sitter::{Language, Query};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageId {
    Rust,
}

#[derive(Debug)]
pub(crate) struct Queries {
    pub(crate) language: Language,
    pub(crate) units: Query,
    pub(crate) comments: Query,
    pub(crate) decisions: Query,
}

#[derive(Debug)]
pub struct CognitiveSpec {
    pub(crate) conditional: &'static str,
    pub(crate) alternative_field: &'static str,
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
    pub extensions: &'static [&'static str],
    pub(crate) cognitive: CognitiveSpec,
    units_source: &'static str,
    comments_source: &'static str,
    decisions_source: &'static str,
    grammar: fn() -> Language,
    compiled: OnceLock<Queries>,
}

impl LangSpec {
    pub(crate) fn queries(&self) -> &Queries {
        self.compiled.get_or_init(|| {
            let language = (self.grammar)();
            let units = compile(&language, self.units_source, self.id, "units");
            let comments = compile(&language, self.comments_source, self.id, "comments");
            let decisions = compile(&language, self.decisions_source, self.id, "decisions");

            Queries {
                language,
                units,
                comments,
                decisions,
            }
        })
    }
}

fn compile(language: &Language, source: &str, id: LanguageId, name: &str) -> Query {
    Query::new(language, source)
        .unwrap_or_else(|error| panic!("{id:?} {name} query does not compile: {error}"))
}

fn rust_grammar() -> Language {
    tree_sitter_rust::LANGUAGE.into()
}

pub static RUST: LangSpec = LangSpec {
    id: LanguageId::Rust,
    extensions: &["rs"],
    cognitive: CognitiveSpec {
        conditional: "if_expression",
        alternative_field: "alternative",
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
    grammar: rust_grammar,
    compiled: OnceLock::new(),
};

pub static ALL: &[&LangSpec] = &[&RUST];

pub fn detect(path: &Path) -> Option<&'static LangSpec> {
    let extension = path.extension()?.to_str()?;
    ALL.iter()
        .copied()
        .find(|spec| spec.extensions.contains(&extension))
}
