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
pub struct LangSpec {
    pub id: LanguageId,
    pub extensions: &'static [&'static str],
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
