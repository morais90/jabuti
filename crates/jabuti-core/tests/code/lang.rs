use std::path::Path;

use jabuti_core::code::lang::{declared_fields, declared_node_kinds, is_test_path};
use jabuti_core::code::{duplication, masking, metrics, units};
use jabuti_core::lang::{self, LanguageId};
use jabuti_core::syntax;

#[test]
fn every_language_compiles_every_query_the_context_declares() {
    for spec in lang::ALL {
        let parsed = syntax::parse("", spec).expect("an empty file parses");

        units::units(&parsed);
        metrics::comment_ranges(&parsed);
        metrics::decisions(&parsed);
        masking::maskings(&parsed);
        duplication::fragments(&parsed, 0);
    }
}

#[test]
fn every_node_kind_a_language_names_exists_in_its_grammar() {
    for spec in lang::ALL {
        let declared = declared_node_kinds(spec.id);
        assert!(!declared.is_empty(), "{:?} declares nothing", spec.id);

        for (kind, named) in declared {
            assert!(
                spec.knows_node_kind(kind, named),
                "{:?} names {kind}, which its grammar does not have",
                spec.id
            );
        }

        let fields = declared_fields(spec.id);
        assert!(!fields.is_empty(), "{:?} declares no fields", spec.id);

        for field in fields {
            assert!(spec.knows_field(field), "{:?} names field {field}", spec.id);
        }
    }
}

#[test]
fn a_language_that_wraps_its_else_branch_declares_the_wrapper() {
    assert!(
        declared_node_kinds(LanguageId::Rust).contains(&("else_clause", true)),
        "rust wraps the else branch and must say so"
    );
    assert!(
        !declared_node_kinds(LanguageId::Kotlin)
            .iter()
            .any(|(kind, _)| *kind == "else_clause"),
        "kotlin has no wrapper to declare"
    );
}

#[test]
fn a_file_under_a_test_directory_is_recognised_by_its_path() {
    assert!(is_test_path(
        LanguageId::Rust,
        Path::new("crates/x/tests/behaviour.rs")
    ));
    assert!(is_test_path(
        LanguageId::Rust,
        Path::new("crates/x/benches/speed.rs")
    ));
    assert!(!is_test_path(
        LanguageId::Rust,
        Path::new("crates/x/src/live.rs")
    ));

    assert!(is_test_path(
        LanguageId::Kotlin,
        Path::new("app/src/test/kotlin/T.kt")
    ));
    assert!(is_test_path(
        LanguageId::Kotlin,
        Path::new("app/src/androidTest/kotlin/T.kt")
    ));
    assert!(!is_test_path(
        LanguageId::Kotlin,
        Path::new("app/src/main/kotlin/T.kt")
    ));
}
