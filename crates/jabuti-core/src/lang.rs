use std::path::Path;
use std::sync::LazyLock;

use tree_sitter::{Language, Query};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LanguageId {
    Kotlin,
    Rust,
    TypeScript,
}

impl LanguageId {
    pub fn name(self) -> &'static str {
        match self {
            Self::Kotlin => "kotlin",
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        ALL.iter().map(|spec| spec.id).find(|id| id.name() == name)
    }

    pub fn spec(self) -> &'static LangSpec {
        match self {
            Self::Kotlin => &KOTLIN,
            Self::Rust => &RUST,
            Self::TypeScript => &TYPESCRIPT,
        }
    }
}

#[derive(Debug)]
pub struct LangSpec {
    pub id: LanguageId,
    pub grammar_version: &'static str,
    pub extensions: &'static [&'static str],
    pub path_separator: &'static str,
    pub test_markers: &'static [&'static str],
    test_paths: &'static [&'static str],
    loaded: LazyLock<Language>,
}

impl LangSpec {
    pub fn language(&self) -> &Language {
        &self.loaded
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

    pub fn is_test_path(&self, path: &Path) -> bool {
        path.components()
            .filter_map(|component| component.as_os_str().to_str())
            .any(|directory| {
                self.test_paths
                    .iter()
                    .any(|name| names_match(name, directory))
            })
    }
}

fn names_match(pattern: &str, directory: &str) -> bool {
    match pattern.strip_prefix('*') {
        Some(suffix) => directory.ends_with(suffix),
        None => directory == pattern,
    }
}

fn kotlin_grammar() -> Language {
    tree_sitter_kotlin_ng::LANGUAGE.into()
}

pub static KOTLIN: LangSpec = LangSpec {
    id: LanguageId::Kotlin,
    grammar_version: "1.1.0",
    extensions: &["kt", "kts"],
    path_separator: ".",
    test_markers: &["@Test", "@ParameterizedTest", "@RepeatedTest"],
    test_paths: &["test", "tests", "*Test"],
    loaded: LazyLock::new(kotlin_grammar),
};

fn rust_grammar() -> Language {
    tree_sitter_rust::LANGUAGE.into()
}

pub static RUST: LangSpec = LangSpec {
    id: LanguageId::Rust,
    grammar_version: "0.24.2",
    extensions: &["rs"],
    path_separator: "::",
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
    loaded: LazyLock::new(rust_grammar),
};

fn typescript_grammar() -> Language {
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
}

pub static TYPESCRIPT: LangSpec = LangSpec {
    id: LanguageId::TypeScript,
    grammar_version: "0.23.2",
    extensions: &["ts"],
    path_separator: ".",
    test_markers: &[],
    test_paths: &["test", "tests", "__tests__", "*.test.ts", "*.spec.ts"],
    loaded: LazyLock::new(typescript_grammar),
};

pub static ALL: &[&LangSpec] = &[&KOTLIN, &RUST, &TYPESCRIPT];

pub fn detect(path: &Path) -> Option<&'static LangSpec> {
    let extension = path.extension()?.to_str()?;
    ALL.iter()
        .copied()
        .find(|spec| spec.extensions.contains(&extension))
}
