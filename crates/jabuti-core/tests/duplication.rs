mod common;

use common::parse_fixture;
use jabuti_core::duplication::{self, FileFragments};
use jabuti_core::model::{Rule, Severity};
use jabuti_core::policy::{Policy, RuleConfig};

const HEADER: &str = "fn parse_header(input: &str) -> Option<String> {\n    let mut parts = input.splitn(2, ':');\n    let name = parts.next()?.trim().to_lowercase();\n    if name.is_empty() {\n        return None;\n    }\n    Some(name)\n}\n";

const RENAMED: &str = "fn read_pair(line: &str) -> Option<String> {\n    let mut pieces = line.splitn(2, ':');\n    let key = pieces.next()?.trim().to_lowercase();\n    if key.is_empty() {\n        return None;\n    }\n    Some(key)\n}\n";

const UNRELATED: &str = "fn total(values: &[i32]) -> i32 {\n    values.iter().sum()\n}\n";

fn fragments(path: &str, source: &str, minimum: u32) -> FileFragments {
    FileFragments {
        path: path.to_owned(),
        fragments: parse_fixture(source).fragments(minimum),
    }
}

fn reporting_above(minimum: u32) -> Policy {
    let mut policy = Policy::default();
    policy.set(
        Rule::DuplicateBlock,
        RuleConfig {
            limit: minimum,
            severity: Severity::Warning,
        },
    );
    policy
}

fn reported(files: &[FileFragments], minimum: u32) -> Vec<String> {
    duplication::duplicates(files, &reporting_above(minimum))
        .into_iter()
        .map(|finding| format!("{}:{}", finding.path, finding.span.start_line))
        .collect()
}

#[test]
fn a_copy_with_every_name_changed_is_still_a_copy() {
    let files = [
        fragments("src/a.rs", HEADER, 40),
        fragments("src/b.rs", RENAMED, 40),
    ];

    assert_eq!(reported(&files, 40), ["src/a.rs:1", "src/b.rs:1"]);
}

#[test]
fn code_that_merely_looks_similar_is_not_reported() {
    let files = [
        fragments("src/a.rs", HEADER, 40),
        fragments("src/c.rs", UNRELATED, 40),
    ];

    assert_eq!(reported(&files, 40), Vec::<String>::new());
}

#[test]
fn only_the_widest_repeated_region_is_reported_not_every_piece_of_it() {
    let files = [
        fragments("src/a.rs", HEADER, 10),
        fragments("src/b.rs", RENAMED, 10),
    ];

    assert_eq!(reported(&files, 10), ["src/a.rs:1", "src/b.rs:1"]);
}

#[test]
fn each_occurrence_is_told_where_its_twin_lives() {
    let files = [
        fragments("src/a.rs", HEADER, 40),
        fragments("src/b.rs", RENAMED, 40),
    ];

    let messages: Vec<String> = duplication::duplicates(&files, &reporting_above(40))
        .into_iter()
        .map(|finding| match finding.detail {
            jabuti_core::model::Detail::Message { message } => message,
            jabuti_core::model::Detail::Threshold { .. } => {
                unreachable!("duplication has no threshold")
            }
        })
        .collect();

    assert!(messages[0].contains("src/b.rs:1"), "{messages:?}");
    assert!(messages[1].contains("src/a.rs:1"), "{messages:?}");
}

#[test]
fn a_rule_switched_off_reports_nothing_however_much_is_repeated() {
    let mut policy = Policy::default();
    policy.set(
        Rule::DuplicateBlock,
        RuleConfig {
            limit: 10,
            severity: Severity::Off,
        },
    );

    let files = [
        fragments("src/a.rs", HEADER, 10),
        fragments("src/b.rs", RENAMED, 10),
    ];

    assert_eq!(duplication::duplicates(&files, &policy), []);
}

#[test]
fn an_attribute_is_metadata_and_does_not_make_two_functions_twins() {
    let first = "#[derive(Debug)]\n#[derive(Clone)]\nstruct A {\n    value: i32,\n}\n";
    let files = [fragments("src/a.rs", first, 5)];

    assert_eq!(reported(&files, 5), Vec::<String>::new());
}

#[test]
fn two_different_tokens_never_share_a_fingerprint() {
    let source = "fn f() {}\n";
    let parsed = parse_fixture(source);
    let fragments = parsed.fragments(1);

    let fingerprint = |bytes: std::ops::Range<usize>| {
        fragments
            .iter()
            .find(|fragment| fragment.bytes == bytes)
            .map(|fragment| fragment.hash)
            .expect("the token is a fragment of its own")
    };

    assert_ne!(fingerprint(4..5), fingerprint(5..6));
}
