mod common;

use common::parse_fixture;
use jabuti_core::metrics::{DecisionIndex, LineIndex};
use jabuti_core::model::{Rule, Severity};
use jabuti_core::policy::{FileUnderReview, Policy, RuleConfig};

fn function_of(body_lines: usize) -> String {
    let body = "    let value = 1;\n".repeat(body_lines);

    format!("fn measured() {{\n{body}}}\n")
}

fn findings_for(source: &str, policy: &Policy) -> Vec<(Rule, u32, u32)> {
    let parsed = parse_fixture(source);
    let lines = LineIndex::new(source, &parsed.comment_ranges());
    let decisions = DecisionIndex::new(&parsed.decisions());

    let file = FileUnderReview {
        path: "measured.rs".to_owned(),
        units: parsed.units(),
        lines: &lines,
        decisions: &decisions,
    };

    policy
        .evaluate(&file)
        .into_iter()
        .map(|finding| (finding.rule, finding.measured, finding.limit))
        .collect()
}

fn only(rule: Rule, config: RuleConfig) -> Policy {
    let mut policy = Policy::default();
    for other in Rule::ALL {
        policy.set(
            other,
            RuleConfig {
                limit: 0,
                severity: Severity::Off,
            },
        );
    }
    policy.set(rule, config);
    policy
}

#[test]
fn a_unit_over_the_limit_is_reported_with_what_it_measured() {
    let policy = only(
        Rule::FunctionLines,
        RuleConfig {
            limit: 10,
            severity: Severity::Warning,
        },
    );

    assert_eq!(
        findings_for(&function_of(20), &policy),
        [(Rule::FunctionLines, 22, 10)]
    );
}

#[test]
fn a_unit_exactly_on_the_limit_is_not_reported() {
    let policy = only(
        Rule::FunctionLines,
        RuleConfig {
            limit: 22,
            severity: Severity::Warning,
        },
    );

    assert_eq!(findings_for(&function_of(20), &policy), []);
}

#[test]
fn a_rule_switched_off_reports_nothing_however_far_over_the_limit() {
    let policy = only(
        Rule::FunctionLines,
        RuleConfig {
            limit: 1,
            severity: Severity::Off,
        },
    );

    assert_eq!(findings_for(&function_of(50), &policy), []);
}

#[test]
fn length_is_measured_on_functions_and_not_on_the_types_that_hold_them() {
    let policy = only(
        Rule::FunctionLines,
        RuleConfig {
            limit: 5,
            severity: Severity::Warning,
        },
    );
    let source = format!("impl Wide {{\n{}}}\n", "    fn tiny() {}\n".repeat(20));

    assert_eq!(findings_for(&source, &policy), []);
}

#[test]
fn the_default_policy_reports_only_function_length() {
    let reported: Vec<Rule> = Rule::ALL
        .into_iter()
        .filter(|rule| {
            Policy::default()
                .config(*rule)
                .is_some_and(|config| config.severity != Severity::Off)
        })
        .collect();

    assert_eq!(reported, [Rule::FunctionLines]);
}
