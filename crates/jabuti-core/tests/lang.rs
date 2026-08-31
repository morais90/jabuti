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
