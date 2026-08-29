use std::path::Path;

use tree_sitter::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageId {
    Rust,
}

pub struct LangSpec {
    pub id: LanguageId,
    pub extensions: &'static [&'static str],
    pub units_query: &'static str,
    pub comments_query: &'static str,
    grammar: fn() -> Language,
}

impl LangSpec {
    pub fn language(&self) -> Language {
        (self.grammar)()
    }
}

fn rust_grammar() -> Language {
    tree_sitter_rust::LANGUAGE.into()
}

pub static RUST: LangSpec = LangSpec {
    id: LanguageId::Rust,
    extensions: &["rs"],
    units_query: include_str!("../queries/rust/units.scm"),
    comments_query: include_str!("../queries/rust/comments.scm"),
    grammar: rust_grammar,
};

pub static ALL: &[&LangSpec] = &[&RUST];

pub fn detect(path: &Path) -> Option<&'static LangSpec> {
    let extension = path.extension()?.to_str()?;
    ALL.iter()
        .copied()
        .find(|spec| spec.extensions.contains(&extension))
}
