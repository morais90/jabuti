use jabuti_core::code::duplication::{self, FileFragments};
use jabuti_core::model::{Rule, Severity};
use jabuti_core::policy::{Policy, RuleConfig};

use super::common::parse_fixture;

const HEADER: &str = "fn parse_header(input: &str) -> Option<String> {\n    let mut parts = input.splitn(2, ':');\n    let name = parts.next()?.trim().to_lowercase();\n    if name.is_empty() {\n        return None;\n    }\n    Some(name)\n}\n";

const RENAMED: &str = "fn read_pair(line: &str) -> Option<String> {\n    let mut pieces = line.splitn(2, ':');\n    let key = pieces.next()?.trim().to_lowercase();\n    if key.is_empty() {\n        return None;\n    }\n    Some(key)\n}\n";

const COMMENTED: &str = "fn read_pair(line: &str) -> Option<String> {\n    // split once\n    let mut pieces = line.splitn(2, ':');\n    let key = pieces.next()?.trim().to_lowercase();\n    if key.is_empty() {\n        return None;\n    }\n    Some(key)\n}\n";

const EXTRA_PARAMETER: &str = "fn other(line: &str, extra: u8) -> Option<String> {\n    let mut pieces = line.splitn(2, ':');\n    let key = pieces.next()?.trim().to_lowercase();\n    if key.is_empty() {\n        return None;\n    }\n    Some(key)\n}\n";

const UNRELATED: &str = "fn total(values: &[i32]) -> i32 {\n    values.iter().sum()\n}\n";

fn fragments(path: &str, source: &str, minimum: u32) -> FileFragments {
    FileFragments {
        path: path.to_owned(),
        fragments: duplication::fragments(&parse_fixture(source), minimum),
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
    let fragments = duplication::fragments(&parsed, 0);

    let fingerprint = |bytes: std::ops::Range<usize>| {
        fragments
            .iter()
            .find(|fragment| fragment.bytes == bytes)
            .map(|fragment| fragment.hash)
            .expect("the token is a fragment of its own")
    };

    assert_ne!(fingerprint(4..5), fingerprint(5..6));
}

fn messages(files: &[FileFragments], minimum: u32) -> Vec<String> {
    duplication::duplicates(files, &reporting_above(minimum))
        .into_iter()
        .map(|finding| match finding.detail {
            jabuti_core::model::Detail::Message { message } => {
                format!("{} {message}", finding.path)
            }
            jabuti_core::model::Detail::Threshold { .. } => {
                unreachable!("duplication has no threshold")
            }
        })
        .collect()
}

#[test]
fn a_comment_added_to_a_copy_does_not_hide_it() {
    let files = [
        fragments("src/a.rs", HEADER, 40),
        fragments("src/b.rs", COMMENTED, 40),
    ];

    assert_eq!(reported(&files, 40), ["src/a.rs:1", "src/b.rs:1"]);
}

#[test]
fn a_narrower_copy_is_reported_on_its_own_rather_than_folded_into_a_wider_one() {
    let files = [
        fragments("src/a.rs", HEADER, 40),
        fragments("src/b.rs", RENAMED, 40),
        fragments("src/c.rs", EXTRA_PARAMETER, 40),
    ];

    let messages = messages(&files, 40);

    assert!(messages[0].contains("src/b.rs:1"), "{messages:?}");
    assert!(!messages[0].contains("src/c.rs"), "{messages:?}");
    assert!(messages[2].contains("src/a.rs:1"), "{messages:?}");
    assert!(messages[2].contains("src/b.rs:1"), "{messages:?}");
}

#[test]
fn the_node_count_describes_the_region_shared_with_the_places_it_names() {
    let files = [
        fragments("src/a.rs", HEADER, 40),
        fragments("src/b.rs", RENAMED, 40),
        fragments("src/c.rs", EXTRA_PARAMETER, 40),
    ];

    let messages = messages(&files, 40);
    let nodes = |message: &str| {
        message
            .split_whitespace()
            .find_map(|word| word.parse::<u32>().ok())
            .expect("the message opens with a node count")
    };

    assert!(nodes(&messages[0]) > nodes(&messages[2]), "{messages:?}");
}

#[test]
fn a_twin_is_never_listed_twice_when_a_wider_copy_already_covers_it() {
    let files = [
        fragments("src/a.rs", HEADER, 40),
        fragments("src/b.rs", RENAMED, 40),
    ];

    let messages = messages(&files, 40);

    assert_eq!(messages[0].matches("src/b.rs").count(), 1, "{messages:?}");
}

#[test]
fn a_long_family_of_copies_names_a_few_and_counts_the_rest() {
    let files: Vec<FileFragments> = (0..6)
        .map(|index| fragments(&format!("src/copy{index}.rs"), HEADER, 40))
        .collect();

    let messages = messages(&files, 40);

    assert!(
        messages[0].ends_with("and 2 more (limit 40)"),
        "{messages:?}"
    );
}

#[test]
fn a_block_of_exactly_the_limit_is_left_alone() {
    let largest = duplication::fragments(&parse_fixture(HEADER), 0)
        .iter()
        .map(|fragment| fragment.nodes)
        .max()
        .expect("the file is a fragment of its own");

    let at_limit = [
        fragments("src/a.rs", HEADER, largest),
        fragments("src/b.rs", RENAMED, largest),
    ];
    assert_eq!(reported(&at_limit, largest), Vec::<String>::new());

    let over_limit = [
        fragments("src/a.rs", HEADER, largest - 1),
        fragments("src/b.rs", RENAMED, largest - 1),
    ];
    assert_eq!(
        reported(&over_limit, largest - 1),
        ["src/a.rs:1", "src/b.rs:1"]
    );
}

#[test]
fn a_family_that_fits_the_message_is_listed_without_a_tally() {
    let files: Vec<FileFragments> = (0..4)
        .map(|index| fragments(&format!("src/copy{index}.rs"), HEADER, 40))
        .collect();

    let messages = messages(&files, 40);

    assert!(!messages[0].contains("more"), "{messages:?}");
    assert!(
        messages[0].ends_with("src/copy3.rs:1 (limit 40)"),
        "{messages:?}"
    );
}
