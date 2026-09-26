use std::collections::BTreeSet;

use jabuti_core::lang::LanguageId;
use jabuti_core::model::{
    Concept, ConceptBindings, Detail, Finding, Portability, Rule, RuleId, Severity, Span,
};
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

fn at(path: &str, lines: (u32, u32), rule: RuleId, subject: Option<&str>) -> Finding {
    Finding {
        rule,
        severity: Severity::Warning,
        path: path.to_owned(),
        span: Span {
            start_line: lines.0,
            end_line: lines.1,
        },
        subject: subject.map(str::to_owned),
        detail: Detail::Message {
            message: "the failure is dropped without being read".to_owned(),
        },
    }
}

fn clippy(lint: &str) -> RuleId {
    RuleId::External {
        tool: "clippy".to_owned(),
        lint: lint.to_owned(),
    }
}

fn threshold(finding: Finding, measured: u32, severity: Severity) -> Finding {
    Finding {
        severity,
        detail: Detail::Threshold {
            measured,
            limit: 60,
        },
        ..finding
    }
}

fn tied_on_one_line() -> Vec<Finding> {
    let function = |rule| at("src/lib.rs", (4, 20), RuleId::Native(rule), Some("run"));
    let masking = |subject| {
        at(
            "src/lib.rs",
            (4, 4),
            Rule::ErrorMasking.into(),
            Some(subject),
        )
    };

    vec![
        at("src/lib.rs", (4, 4), clippy("unwrap_used"), None),
        at("src/lib.rs", (4, 4), clippy("expect_used"), None),
        threshold(function(Rule::FunctionLines), 71, Severity::Error),
        threshold(function(Rule::FunctionLines), 71, Severity::Warning),
        threshold(function(Rule::FunctionLines), 64, Severity::Warning),
        threshold(function(Rule::CognitiveComplexity), 9, Severity::Warning),
        masking("unwrap"),
        masking("expect"),
        at(
            "src/cli.rs",
            (9, 9),
            Rule::ErrorMasking.into(),
            Some("unwrap"),
        ),
    ]
}

#[test]
fn findings_settle_into_one_order_whatever_order_they_arrive_in() {
    let mut forward = tied_on_one_line();
    let mut reversed: Vec<Finding> = tied_on_one_line().into_iter().rev().collect();

    forward.sort();
    reversed.sort();

    let function = |rule| at("src/lib.rs", (4, 20), RuleId::Native(rule), Some("run"));
    let masking = |subject| {
        at(
            "src/lib.rs",
            (4, 4),
            Rule::ErrorMasking.into(),
            Some(subject),
        )
    };
    let expected = vec![
        at(
            "src/cli.rs",
            (9, 9),
            Rule::ErrorMasking.into(),
            Some("unwrap"),
        ),
        masking("expect"),
        masking("unwrap"),
        at("src/lib.rs", (4, 4), clippy("expect_used"), None),
        at("src/lib.rs", (4, 4), clippy("unwrap_used"), None),
        threshold(function(Rule::CognitiveComplexity), 9, Severity::Warning),
        threshold(function(Rule::FunctionLines), 64, Severity::Warning),
        threshold(function(Rule::FunctionLines), 71, Severity::Warning),
        threshold(function(Rule::FunctionLines), 71, Severity::Error),
    ];
    assert_eq!(forward, expected);
    assert_eq!(reversed, expected);
}
