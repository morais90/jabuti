use jabuti_core::catalog::{Rule, RuleId, Severity};
use jabuti_core::model::{Detail, Finding, Span};

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
