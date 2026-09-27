use std::collections::BTreeMap;

use jabuti_core::catalog::{Portability, Rule};
use jabuti_core::code::lang::{declared_fields, declared_node_kinds};
use jabuti_core::code::{concepts, duplication, metrics, support, units};
use jabuti_core::lang::{self, LanguageId};
use jabuti_core::policy::ConceptBindings;
use jabuti_core::syntax;

#[test]
fn every_language_compiles_every_query_the_context_declares() {
    for spec in lang::ALL {
        let parsed = syntax::parse("", spec).expect("an empty file parses");

        units::units(&parsed);
        metrics::comment_ranges(&parsed);
        metrics::decisions(&parsed);
        concepts::occurrences(&parsed, &ConceptBindings::default(), &BTreeMap::new());
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
fn every_language_reports_available_rules_from_its_tables() {
    for spec in lang::ALL {
        assert_eq!(
            support::available_rules(spec.id),
            Rule::ALL,
            "{:?}",
            spec.id
        );
    }
}

#[test]
fn each_portability_class_uses_its_own_availability_signal() {
    assert_eq!(
        [
            support::portability_available(Portability::Universal, false, false),
            support::portability_available(Portability::Universal, true, true),
            support::portability_available(Portability::ConceptBound, false, true),
            support::portability_available(Portability::ConceptBound, true, false),
            support::portability_available(Portability::LanguageSpecific, true, false),
            support::portability_available(Portability::LanguageSpecific, false, true),
        ],
        [true, true, false, true, false, true]
    );
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
