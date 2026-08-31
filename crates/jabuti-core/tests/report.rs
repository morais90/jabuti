use jabuti_core::model::{Detail, Finding, Rule, RuleId, Severity, Span};
use jabuti_core::report::{self, Scanned};

fn finding(severity: Severity, line: u32, subject: Option<&str>) -> Finding {
    Finding {
        rule: RuleId::Native(Rule::FunctionLines),
        severity,
        path: "src/handler.rs".to_owned(),
        span: Span {
            start_line: line,
            end_line: line + 10,
        },
        subject: subject.map(str::to_owned),
        detail: Detail::Threshold {
            measured: 71,
            limit: 60,
        },
    }
}

fn scanned() -> Scanned {
    Scanned {
        files: 42,
        units: 378,
    }
}

#[test]
fn a_clean_run_says_so_in_a_single_line() {
    let rendered = report::agent(&[], scanned(), 40);

    assert_eq!(rendered, "No findings across 42 files and 378 units.\n");
}

#[test]
fn a_finding_names_its_severity_rule_location_and_both_numbers() {
    let rendered = report::agent(
        &[finding(Severity::Error, 120, Some("handle_request"))],
        scanned(),
        40,
    );

    assert_eq!(
        rendered,
        "1 error and 0 warnings across 42 files and 378 units.\n\
         \n\
         src/handler.rs:120  error  function-lines  handle_request  measured 71, limit 60\n"
    );
}

#[test]
fn a_finding_without_a_subject_leaves_no_gap_where_the_name_would_be() {
    let rendered = report::agent(&[finding(Severity::Warning, 1, None)], scanned(), 40);

    assert!(
        rendered.ends_with("src/handler.rs:1  warning  function-lines  measured 71, limit 60\n"),
        "{rendered}"
    );
}

#[test]
fn output_beyond_the_limit_is_replaced_by_a_count_of_what_was_withheld() {
    let findings: Vec<Finding> = (1..=10)
        .map(|line| finding(Severity::Warning, line, Some("wide")))
        .collect();

    let rendered = report::agent(&findings, scanned(), 3);

    assert_eq!(
        rendered
            .lines()
            .filter(|line| line.contains("wide"))
            .count(),
        3
    );
    assert!(
        rendered.contains("7 further findings not shown"),
        "{rendered}"
    );
}

#[test]
fn only_an_error_severity_finding_fails_the_gate() {
    assert!(!report::has_errors(&[finding(Severity::Warning, 1, None)]));
    assert!(report::has_errors(&[finding(Severity::Error, 1, None)]));
}
