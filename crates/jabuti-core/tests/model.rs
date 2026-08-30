use std::collections::BTreeSet;

use jabuti_core::model::Rule;
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
