use std::path::Path;

use jabuti_core::lang::{self, LanguageId};
use rstest::rstest;

#[test]
fn a_rust_file_is_detected_by_its_extension() {
    let spec = lang::detect(Path::new("crates/jabuti-core/src/syntax.rs")).expect("rust is known");

    assert_eq!(spec.id, LanguageId::Rust);
    assert_eq!(spec.extensions, ["rs"]);
}

#[test]
fn a_file_of_an_unsupported_language_is_not_detected() {
    assert!(lang::detect(Path::new("README.md")).is_none());
}

#[test]
fn a_file_without_an_extension_is_not_detected() {
    assert!(lang::detect(Path::new("justfile")).is_none());
}

#[rstest]
#[case(LanguageId::Kotlin, "kotlin")]
#[case(LanguageId::Rust, "rust")]
fn a_language_answers_to_its_own_name(#[case] id: LanguageId, #[case] name: &str) {
    assert_eq!(id.name(), name);
    assert_eq!(LanguageId::from_name(name), Some(id));
}

#[test]
fn a_name_no_language_publishes_resolves_to_nothing() {
    assert_eq!(LanguageId::from_name("cobol"), None);
}

#[test]
fn no_two_languages_share_a_name_or_an_extension() {
    let mut names = std::collections::BTreeSet::new();
    let mut extensions = std::collections::BTreeSet::new();

    for spec in lang::ALL {
        assert!(names.insert(spec.id.name()), "{:?}", spec.id);
        for extension in spec.extensions {
            assert!(extensions.insert(*extension), "{extension}");
        }
    }
}

#[test]
fn the_grammar_version_a_language_reports_is_the_one_we_depend_on() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("the manifest is readable");

    for spec in lang::ALL {
        let crate_name = match spec.id {
            LanguageId::Kotlin => "tree-sitter-kotlin-ng",
            LanguageId::Rust => "tree-sitter-rust",
        };
        let Some(declared) = manifest
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{crate_name} = ")))
            .map(|value| value.trim_matches('"'))
        else {
            panic!("{crate_name} is not a plain dependency line");
        };

        assert_eq!(spec.grammar_version, declared, "{:?}", spec.id);
    }
}

#[rstest]
#[case("if_expression", true, true)]
#[case("spline_expression", true, false)]
#[case("", true, false)]
#[case("&&", false, true)]
#[case("<=>", false, false)]
fn a_grammar_answers_whether_it_has_a_node_kind(
    #[case] kind: &str,
    #[case] named: bool,
    #[case] known: bool,
) {
    assert_eq!(lang::RUST.knows_node_kind(kind, named), known);
}

#[test]
fn an_empty_name_is_never_a_node_kind_the_grammar_has() {
    assert!(!lang::RUST.knows_node_kind("", true));
    assert!(!lang::KOTLIN.knows_node_kind("", true));
}

#[rstest]
#[case("condition", true)]
#[case("operator", true)]
#[case("spline", false)]
fn a_grammar_answers_whether_it_has_a_field(#[case] field: &str, #[case] known: bool) {
    assert_eq!(lang::RUST.knows_field(field), known);
}
