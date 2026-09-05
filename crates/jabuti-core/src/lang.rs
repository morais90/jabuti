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

    pub fn spec(self) -> &'static LangSpec {
        match self {
            Self::Kotlin => &KOTLIN,
            Self::Rust => &RUST,
        }
    }
}

#[derive(Debug)]
pub struct LangSpec {
    pub id: LanguageId,
    pub grammar_version: &'static str,
    pub extensions: &'static [&'static str],
    grammar: fn() -> Language,
    loaded: OnceLock<Language>,
}

impl LangSpec {
    pub fn language(&self) -> &Language {
        self.loaded.get_or_init(self.grammar)
    }

    pub fn knows_node_kind(&self, kind: &str, named: bool) -> bool {
        !kind.is_empty() && self.language().id_for_node_kind(kind, named) != 0
    }

    pub fn knows_field(&self, field: &str) -> bool {
        self.language().field_id_for_name(field).is_some()
    }

    pub(crate) fn query(&self, name: &str, source: &str) -> Query {
        Query::new(self.language(), source)
            .unwrap_or_else(|error| panic!("{:?} {name} query does not compile: {error}", self.id))
    }
}

fn kotlin_grammar() -> Language {
    tree_sitter_kotlin_ng::LANGUAGE.into()
}

pub static KOTLIN: LangSpec = LangSpec {
    id: LanguageId::Kotlin,
    grammar_version: "1.1.0",
    extensions: &["kt", "kts"],
    grammar: kotlin_grammar,
    loaded: OnceLock::new(),
};

fn rust_grammar() -> Language {
    tree_sitter_rust::LANGUAGE.into()
}

pub static RUST: LangSpec = LangSpec {
    id: LanguageId::Rust,
    grammar_version: "0.24.2",
    extensions: &["rs"],
    grammar: rust_grammar,
    loaded: OnceLock::new(),
};

pub static ALL: &[&LangSpec] = &[&KOTLIN, &RUST];

pub fn detect(path: &Path) -> Option<&'static LangSpec> {
    let extension = path.extension()?.to_str()?;
    ALL.iter()
        .copied()
        .find(|spec| spec.extensions.contains(&extension))
}
