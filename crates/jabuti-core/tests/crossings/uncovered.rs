use std::path::Path;

use jabuti_core::catalog::{Rule, RuleId};
use jabuti_core::crossings::uncovered::{self, FileUnderCoverage};
use jabuti_core::lang::LanguageId;
use jabuti_core::model::{Detail, Finding, Span};
use jabuti_core::policy::{Policy, RuleConfig};
use jabuti_core::tools::coverage::{Coverage, FileCoverage, Format};

fn lcov() -> Coverage {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tools/coverage/report.lcov");
    let text = std::fs::read_to_string(path).expect("fixture exists");

    Coverage::parse(Format::Lcov, &text).expect("the fixture is LCOV")
}

fn reporting(enabled: bool) -> Policy {
    let mut policy = Policy::default();
    policy.set(Rule::UncoveredNewCode, RuleConfig { limit: 0, enabled });
    policy
}

fn lib() -> FileCoverage {
    lcov()
        .file(Path::new("/home/someone/checkout/src/lib.rs"))
        .cloned()
        .expect("the file is in the report")
}

fn uncovered(added: impl Fn(u32) -> bool, policy: &Policy) -> Vec<Finding> {
    uncovered_around(&[], added, policy)
}

fn uncovered_around(
    compile_time: &[Span],
    added: impl Fn(u32) -> bool,
    policy: &Policy,
) -> Vec<Finding> {
    let file = FileUnderCoverage {
        path: "src/lib.rs",
        language: LanguageId::Rust,
        compile_time,
    };

    uncovered::findings(&file, &lib(), added, policy)
}

fn finding(start_line: u32, end_line: u32, message: &str) -> Finding {
    Finding {
        rule: RuleId::Native(Rule::UncoveredNewCode),
        path: "src/lib.rs".to_owned(),
        span: Span {
            start_line,
            end_line,
        },
        subject: None,
        detail: Detail::Message {
            message: message.to_owned(),
        },
    }
}

#[test]
fn consecutive_uncovered_new_lines_are_reported_once_as_a_stretch() {
    let found = uncovered(|line| (5..=11).contains(&line), &reporting(true));

    assert_eq!(found, [finding(5, 11, "5 lines run by no test")]);
}

#[test]
fn a_covered_line_in_between_splits_the_stretch() {
    let found = uncovered(|line| (1..=14).contains(&line), &reporting(true));

    assert_eq!(
        found,
        [
            finding(5, 11, "5 lines run by no test"),
            finding(14, 14, "1 line run by no test"),
        ]
    );
}

#[test]
fn a_line_that_was_not_added_is_never_reported_even_when_uncovered() {
    let found = uncovered(|line| line == 14, &reporting(true));

    assert_eq!(found, [finding(14, 14, "1 line run by no test")]);
}

#[test]
fn a_line_no_test_can_reach_is_not_uncovered_when_it_was_not_instrumented() {
    let found = uncovered(|line| line == 4 || line == 8, &reporting(true));

    assert_eq!(found, []);
}

#[test]
fn a_switched_off_rule_reports_nothing() {
    assert_eq!(uncovered(|_| true, &reporting(false)), []);
}

#[test]
fn a_language_that_switches_the_rule_off_reports_nothing_for_its_files() {
    let mut policy = reporting(true);
    policy.set_for(
        LanguageId::Rust,
        Rule::UncoveredNewCode,
        RuleConfig {
            limit: 0,
            enabled: false,
        },
    );

    assert_eq!(uncovered(|_| true, &policy), []);
}

#[test]
fn code_that_may_run_at_compile_time_and_never_ran_is_left_out() {
    let constant = Span {
        start_line: 9,
        end_line: 11,
    };

    let found = uncovered_around(
        &[constant],
        |line| (5..=11).contains(&line),
        &reporting(true),
    );

    assert_eq!(found, [finding(5, 7, "3 lines run by no test")]);
}

#[test]
fn code_that_may_run_at_compile_time_but_also_ran_is_judged_like_any_other() {
    let partly_run = Span {
        start_line: 1,
        end_line: 11,
    };

    let found = uncovered_around(
        &[partly_run],
        |line| (5..=11).contains(&line),
        &reporting(true),
    );

    assert_eq!(found, [finding(5, 11, "5 lines run by no test")]);
}
