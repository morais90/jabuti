use std::collections::BTreeSet;

use jabuti_core::lang::LanguageId;
use jabuti_core::model::{Concept, ConceptBindings, Portability, Rule, RuleId};
use rstest::rstest;

#[rstest]
#[case("cyclomatic-complexity", Rule::CyclomaticComplexity)]
#[case("file-lines", Rule::FileLines)]
#[case("function-lines", Rule::FunctionLines)]
fn a_rule_is_found_by_the_id_it_publishes(#[case] id: &str, #[case] expected: Rule) {
    assert_eq!(Rule::from_id(id), Some(expected));
}

#[test]
fn an_id_no_rule_publishes_resolves_to_nothing() {
    assert_eq!(Rule::from_id("spline-reticulation"), None);
}

#[test]
fn no_two_rules_share_an_id() {
    let ids: BTreeSet<&str> = Rule::ALL.into_iter().map(Rule::id).collect();

    assert_eq!(ids.len(), Rule::ALL.len());
}

#[test]
fn every_native_rule_declares_its_portability_class() {
    let concept_bound: Vec<Rule> = Rule::ALL
        .into_iter()
        .filter(|rule| rule.portability() == Portability::ConceptBound)
        .collect();
    let language_specific: Vec<Rule> = Rule::ALL
        .into_iter()
        .filter(|rule| rule.portability() == Portability::LanguageSpecific)
        .collect();

    assert_eq!(
        concept_bound,
        [Rule::Assertion, Rule::ErrorMasking, Rule::Suppression]
    );
    assert_eq!(language_specific, []);
}

#[test]
fn concept_bindings_report_global_and_per_language_emptiness() {
    let mut bindings = ConceptBindings::default();
    assert_eq!(
        (
            bindings.is_empty(),
            bindings.is_empty_for(LanguageId::Rust),
            bindings.is_empty_for(LanguageId::TypeScript),
        ),
        (true, true, true)
    );

    bindings.set(
        LanguageId::Rust,
        Concept::ErrorDiscard,
        vec!["mycorp::discard".to_owned()],
    );

    assert_eq!(
        (
            bindings.is_empty(),
            bindings.is_empty_for(LanguageId::Rust),
            bindings.is_empty_for(LanguageId::TypeScript),
        ),
        (false, false, true)
    );
}

#[rstest]
#[case("function-lines", "function-lines")]
#[case("hotspot", "hotspot")]
fn a_native_identifier_round_trips(#[case] id: &str, #[case] expected: &str) {
    let parsed = RuleId::parse(id).expect("a native rule");

    assert_eq!(parsed.id(), expected);
    assert!(matches!(parsed, RuleId::Native(_)));
}

#[test]
fn a_slash_makes_an_identifier_belong_to_a_tool() {
    let parsed = RuleId::parse("clippy/needless_range_loop").expect("an external rule");

    assert_eq!(
        parsed,
        RuleId::External {
            tool: "clippy".to_owned(),
            lint: "needless_range_loop".to_owned(),
        }
    );
    assert_eq!(parsed.id(), "clippy/needless_range_loop");
}

#[rstest]
#[case("spline-reticulation")]
#[case("/needless_range_loop")]
#[case("clippy/")]
fn an_identifier_naming_nothing_is_rejected(#[case] id: &str) {
    assert_eq!(RuleId::parse(id), None);
}

#[test]
fn only_the_rules_compared_across_files_belong_to_the_repository() {
    let wide: Vec<Rule> = Rule::ALL
        .into_iter()
        .filter(|rule| rule.repository_wide())
        .collect();

    assert_eq!(
        wide,
        [Rule::DuplicateBlock, Rule::Hotspot, Rule::LayerViolation]
    );
}
