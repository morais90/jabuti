mod coverage;

use std::collections::BTreeSet;

use jabuti_core::catalog::RuleId;
use jabuti_core::model::{Detail, Finding, Span};
use jabuti_core::tools;

fn recorded(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/tools/clippy/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).expect("fixture exists")
}

fn diagnostics() -> Vec<Finding> {
    let recorded = recorded("diagnostics.jsonl");

    tools::cargo_diagnostics("clippy", &recorded, |_| true)
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
            path: "src/lib.rs".to_owned(),
            span: Span {
                start_line: 3,
                end_line: 3
            },
            subject: None,
            detail: Detail::Message {
                message: "the loop variable `index` is only used to index `values`".to_owned()
            },
        })
    );
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
        tools::cargo_diagnostics("clippy", "not json at all\n", |_| true),
        []
    );
}

#[test]
fn a_warning_from_a_lint_the_tool_leaves_off_by_default_is_not_reported() {
    let recorded = recorded("diagnostics.jsonl");

    let found = tools::cargo_diagnostics("clippy", &recorded, |lint| {
        lint != "clippy::needless_range_loop"
    });

    assert_eq!(ids(&found), ["clippy/unused_variables"]);
}

#[test]
fn an_error_is_reported_whatever_the_lint() {
    let recorded = recorded("diagnostics.jsonl");

    let found = tools::cargo_diagnostics("clippy", &recorded, |_| false);

    assert_eq!(ids(&found), ["clippy/unused_variables"]);
}

#[test]
fn the_lints_a_tool_enables_by_default_are_read_from_its_listing() {
    let enabled = tools::enabled_by_default(&recorded("lint-levels.txt"));

    assert_eq!(
        enabled,
        BTreeSet::from([
            "clippy::approx_constant".to_owned(),
            "clippy::needless_range_loop".to_owned(),
            "dead_code".to_owned(),
            "unconditional_panic".to_owned(),
            "unused_variables".to_owned(),
        ])
    );
}

#[test]
fn a_forbidden_lint_counts_as_enabled_by_default() {
    let enabled = tools::enabled_by_default("    some-lint  forbid    never\n");

    assert_eq!(enabled, BTreeSet::from(["some_lint".to_owned()]));
}
