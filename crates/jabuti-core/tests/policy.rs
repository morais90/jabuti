mod common;

use common::parse_fixture;
use jabuti_core::metrics::{CognitiveIndex, DecisionIndex, LineIndex};
use jabuti_core::model::{Detail, Finding, Rule, RuleId, Severity, Span};
use jabuti_core::policy::{FileUnderReview, Policy, RuleConfig};

fn function_of(body_lines: usize) -> String {
    let body = "    let value = 1;\n".repeat(body_lines);

    format!("fn measured() {{\n{body}}}\n")
}

fn findings_for(source: &str, policy: &Policy) -> Vec<(RuleId, u32, u32)> {
    let parsed = parse_fixture(source);
    let lines = LineIndex::new(source, &parsed.comment_ranges());
    let decisions = DecisionIndex::new(&parsed.decisions());
    let cognitive = CognitiveIndex::new(&parsed.increments());

    let file = FileUnderReview {
        path: "measured.rs".to_owned(),
        units: parsed.units(),
        lines: &lines,
        decisions: &decisions,
        cognitive: &cognitive,
        churn: 0,
    };

    policy
        .evaluate(&file)
        .into_iter()
        .map(|finding| match finding.detail {
            Detail::Threshold { measured, limit } => (finding.rule, measured, limit),
            Detail::Message(_) => unreachable!("native rules report a threshold"),
        })
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
        [(RuleId::Native(Rule::FunctionLines), 22, 10)]
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
fn the_default_policy_reports_four_of_the_seven_rules() {
    let reported: Vec<Rule> = Rule::ALL
        .into_iter()
        .filter(|rule| {
            Policy::default()
                .config(*rule)
                .is_some_and(|config| config.severity != Severity::Off)
        })
        .collect();

    assert_eq!(
        reported,
        [
            Rule::Hotspot,
            Rule::CognitiveComplexity,
            Rule::FunctionLines,
            Rule::Parameters
        ]
    );
}

#[test]
fn no_rule_defaults_to_failing_the_run() {
    let severities: Vec<Severity> = Rule::ALL
        .into_iter()
        .filter_map(|rule| Policy::default().config(rule))
        .map(|config| config.severity)
        .collect();

    assert!(!severities.contains(&Severity::Error), "{severities:?}");
}

fn reported_by(tool: &str, lint: &str, severity: Severity) -> Finding {
    Finding {
        rule: RuleId::External {
            tool: tool.to_owned(),
            lint: lint.to_owned(),
        },
        severity,
        path: "src/lib.rs".to_owned(),
        span: Span {
            start_line: 3,
            end_line: 3,
        },
        subject: None,
        detail: Detail::Message("the loop variable is only used to index".to_owned()),
    }
}

#[test]
fn a_finding_no_rule_mentions_keeps_the_severity_it_arrived_with() {
    let reported = reported_by("clippy", "needless_range_loop", Severity::Error);

    assert_eq!(
        Policy::default().admit(reported.clone()),
        Some(reported.clone())
    );
}

#[test]
fn a_finding_the_configuration_switches_off_is_dropped() {
    let reported = reported_by("clippy", "needless_range_loop", Severity::Error);
    let mut policy = Policy::default();
    policy.set(
        reported.rule.clone(),
        RuleConfig {
            limit: 0,
            severity: Severity::Off,
        },
    );

    assert_eq!(policy.admit(reported), None);
}

#[test]
fn the_configuration_can_soften_a_severity_the_tool_chose() {
    let reported = reported_by("clippy", "needless_range_loop", Severity::Error);
    let mut policy = Policy::default();
    policy.set(
        reported.rule.clone(),
        RuleConfig {
            limit: 0,
            severity: Severity::Warning,
        },
    );

    let admitted = policy.admit(reported).expect("still reported");

    assert_eq!(admitted.severity, Severity::Warning);
    assert_eq!(
        admitted.detail,
        Detail::Message("the loop variable is only used to index".to_owned())
    );
}
