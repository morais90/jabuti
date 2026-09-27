use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use jabuti_core::catalog::{Rule, Severity, UnitKind};
use jabuti_core::code::{self, Measured, Reviewed, Scan};
use jabuti_core::lang;
use jabuti_core::model::Span;
use jabuti_core::policy::{ConceptBindings, Policy, RuleConfig};
use jabuti_core::report::{Outcome, Scanned};
use jabuti_core::syntax::{self, Text};

const GUARDED: &str = "#[allow(dead_code)]
fn read(value: Option<u32>) -> u32 {
    if value.is_some() {
        return value.unwrap();
    }
    0
}

fn small() {}
";

const DOUBLING: &str = "fn total(values: &[u32]) -> u32 {
    let mut sum = 0;
    for value in values {
        sum += value * 2;
    }
    sum
}
";

fn text(shown: &str, source: &str) -> Text {
    Text {
        path: PathBuf::from("/project").join(shown),
        shown: shown.to_owned(),
        spec: &lang::RUST,
        source: source.to_owned(),
    }
}

fn policy(rules: &[(Rule, u32, Severity)]) -> Policy {
    let mut policy = Policy::default();
    for &(rule, limit, severity) in rules {
        policy.set(rule, RuleConfig { limit, severity });
    }
    policy
}

fn reviewed(text: &Text, request: &Scan<'_>) -> Reviewed {
    let parsed = syntax::parse(&text.source, text.spec).expect("fixture parses cleanly");

    code::reviewed(text, &parsed, &BTreeMap::new(), request)
}

fn fired(outcome: &Outcome) -> Vec<(&str, String, Option<&str>)> {
    let mut fired: Vec<(&str, String, Option<&str>)> = outcome
        .findings
        .iter()
        .map(|finding| {
            (
                finding.path.as_str(),
                finding.rule.id(),
                finding.subject.as_deref(),
            )
        })
        .collect();
    fired.sort();
    fired
}

fn read_units(outcome: &Outcome) -> Vec<(&str, UnitKind, Option<&str>)> {
    outcome
        .readings
        .iter()
        .map(|reading| {
            (
                reading.path.as_str(),
                reading.kind,
                reading.subject.as_deref(),
            )
        })
        .collect()
}

#[test]
fn a_file_is_counted_measured_and_judged_whole() {
    let policy = policy(&[
        (Rule::FunctionLines, 2, Severity::Warning),
        (Rule::DuplicateBlock, 0, Severity::Off),
    ]);
    let churn = BTreeMap::from([(PathBuf::from("/project/src/lib.rs"), 5)]);
    let request = Scan {
        policy: &policy,
        bindings: &ConceptBindings::default(),
        touched: None,
        churn: &churn,
    };
    let lib = text("src/lib.rs", GUARDED);

    let (outcome, measured) = code::scan(vec![reviewed(&lib, &request)], &request);

    assert_eq!(outcome.scanned, Scanned { files: 1, units: 2 });
    assert_eq!(
        fired(&outcome),
        [
            ("src/lib.rs", "error-masking".to_owned(), Some("unwrap")),
            ("src/lib.rs", "function-lines".to_owned(), Some("read")),
            ("src/lib.rs", "suppression".to_owned(), Some("dead_code")),
        ]
    );
    assert_eq!(
        read_units(&outcome),
        [
            ("src/lib.rs", UnitKind::File, None),
            ("src/lib.rs", UnitKind::Function, Some("read")),
            ("src/lib.rs", UnitKind::Function, Some("small")),
        ]
    );
    assert_eq!(
        measured,
        [Measured {
            path: "src/lib.rs".to_owned(),
            span: Span {
                start_line: 1,
                end_line: 9,
            },
            churn: 5,
            complexity: 1,
        }]
    );
}

#[test]
fn test_code_may_mask_an_error_and_a_file_git_never_saw_has_no_churn() {
    let policy = policy(&[
        (Rule::FunctionLines, 2, Severity::Warning),
        (Rule::DuplicateBlock, 0, Severity::Off),
    ]);
    let request = Scan {
        policy: &policy,
        bindings: &ConceptBindings::default(),
        touched: None,
        churn: &BTreeMap::new(),
    };
    let helper = text("tests/helper.rs", GUARDED);

    let (outcome, measured) = code::scan(vec![reviewed(&helper, &request)], &request);

    assert_eq!(
        fired(&outcome),
        [
            ("tests/helper.rs", "function-lines".to_owned(), Some("read")),
            (
                "tests/helper.rs",
                "suppression".to_owned(),
                Some("dead_code")
            ),
        ]
    );
    assert_eq!(
        measured.iter().map(|file| file.churn).collect::<Vec<_>>(),
        [0]
    );
}

#[test]
fn only_the_touched_files_are_reviewed_unless_duplication_reads_them_all() {
    let paths = [
        PathBuf::from("/project/a.rs"),
        PathBuf::from("/project/b.rs"),
    ];
    let touched = BTreeSet::from([PathBuf::from("/project/a.rs")]);
    let quiet = policy(&[(Rule::DuplicateBlock, 0, Severity::Off)]);
    let duplicating = policy(&[(Rule::DuplicateBlock, 20, Severity::Warning)]);
    let scope = |policy: &Policy, touched: Option<&BTreeSet<PathBuf>>| {
        Scan {
            policy,
            bindings: &ConceptBindings::default(),
            touched,
            churn: &BTreeMap::new(),
        }
        .scope(&paths)
    };
    let every = BTreeSet::from(paths.clone());

    assert_eq!(scope(&quiet, Some(&touched)), touched);
    assert_eq!(scope(&duplicating, Some(&touched)), every);
    assert_eq!(scope(&quiet, None), every);
}

#[test]
fn an_untouched_file_feeds_duplication_without_being_counted_or_judged() {
    let policy = policy(&[
        (Rule::DuplicateBlock, 20, Severity::Warning),
        (Rule::FunctionLines, 2, Severity::Warning),
    ]);
    let touched = BTreeSet::from([PathBuf::from("/project/a.rs")]);
    let request = Scan {
        policy: &policy,
        bindings: &ConceptBindings::default(),
        touched: Some(&touched),
        churn: &BTreeMap::new(),
    };
    let files = [text("a.rs", DOUBLING), text("b.rs", DOUBLING)];

    let (outcome, measured) = code::scan(
        files.iter().map(|file| reviewed(file, &request)).collect(),
        &request,
    );

    assert_eq!(outcome.scanned, Scanned { files: 1, units: 1 });
    assert_eq!(
        fired(&outcome),
        [
            ("a.rs", "duplicate-block".to_owned(), None),
            ("a.rs", "function-lines".to_owned(), Some("total")),
            ("b.rs", "duplicate-block".to_owned(), None),
        ]
    );
    assert_eq!(
        measured
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        ["a.rs", "b.rs"]
    );
}

#[test]
fn nothing_is_fragmented_while_duplication_is_switched_off() {
    let policy = policy(&[(Rule::DuplicateBlock, 20, Severity::Off)]);
    let request = Scan {
        policy: &policy,
        bindings: &ConceptBindings::default(),
        touched: None,
        churn: &BTreeMap::new(),
    };
    let files = [text("a.rs", DOUBLING), text("b.rs", DOUBLING)];

    let (outcome, _) = code::scan(
        files.iter().map(|file| reviewed(file, &request)).collect(),
        &request,
    );

    assert_eq!(fired(&outcome), []);
}
