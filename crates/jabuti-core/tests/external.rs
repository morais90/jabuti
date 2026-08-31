use jabuti_core::external;
use jabuti_core::model::{Detail, Finding, RuleId, Severity, Span};

fn diagnostics() -> Vec<Finding> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/clippy/diagnostics.jsonl"
    );
    let recorded = std::fs::read_to_string(path).expect("fixture exists");

    external::cargo_diagnostics("clippy", &recorded)
}

fn ids(findings: &[Finding]) -> Vec<String> {
    findings.iter().map(|finding| finding.rule.id()).collect()
}

#[test]
fn a_diagnostic_becomes_a_finding_carrying_the_tools_own_message() {
    let found = diagnostics();

    assert_eq!(
        found.first(),
        Some(&Finding {
            rule: RuleId::External {
                tool: "clippy".to_owned(),
                lint: "needless_range_loop".to_owned(),
            },
            severity: Severity::Warning,
            path: "src/lib.rs".to_owned(),
            span: Span {
                start_line: 3,
                end_line: 3
            },
            subject: None,
            detail: Detail::Message(
                "the loop variable `index` is only used to index `values`".to_owned()
            ),
        })
    );
}

#[test]
fn the_severity_is_the_one_the_tool_reported() {
    let levels: Vec<Severity> = diagnostics()
        .iter()
        .map(|finding| finding.severity)
        .collect();

    assert_eq!(levels, [Severity::Warning, Severity::Error]);
}

#[test]
fn a_lint_keeps_its_name_without_the_tools_own_prefix() {
    assert_eq!(
        ids(&diagnostics()),
        ["clippy/needless_range_loop", "clippy/unused_variables"]
    );
}

#[test]
fn the_same_diagnostic_reported_twice_is_recorded_once() {
    let repeated = ids(&diagnostics())
        .iter()
        .filter(|id| *id == "clippy/needless_range_loop")
        .count();

    assert_eq!(repeated, 1);
}

#[test]
fn a_message_with_nothing_to_anchor_it_is_left_out() {
    let ids = ids(&diagnostics());

    assert!(!ids.contains(&"clippy/orphaned".to_owned()), "{ids:?}");
    assert_eq!(ids.len(), 2);
}

#[test]
fn output_that_is_not_a_diagnostic_is_ignored_rather_than_failing() {
    assert_eq!(
        external::cargo_diagnostics("clippy", "not json at all\n"),
        []
    );
}
