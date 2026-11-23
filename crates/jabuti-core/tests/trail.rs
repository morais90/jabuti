use jabuti_core::catalog::{Rule, RuleId};
use jabuti_core::model::{Detail, Finding, Span};
use jabuti_core::report::trail::{self, Commit, Line};

const SOURCE: &str = "/// Doubles a value.\nfn doubled(x: u8) -> u8 {\n    x * 2\n}\n\nfn tripled(x: u8) -> u8 {\n    x * 3\n}\n";

fn line(number: u32, text: &str, changed: bool) -> Line {
    Line {
        number,
        text: text.to_owned(),
        changed,
    }
}

fn finding(path: &str, start_line: u32, rule: Rule) -> Finding {
    Finding {
        rule: RuleId::Native(rule),
        path: path.to_owned(),
        span: Span {
            start_line,
            end_line: start_line + 2,
        },
        subject: Some("unit".to_owned()),
        detail: Detail::Threshold {
            measured: 5,
            limit: 4,
        },
    }
}

#[test]
fn an_excerpt_is_the_first_three_lines_of_a_unit_marked_where_changed() {
    let excerpt = trail::excerpt(SOURCE, 6, |number| number == 7);

    assert_eq!(
        excerpt,
        [
            line(6, "fn tripled(x: u8) -> u8 {", false),
            line(7, "    x * 3", true),
            line(8, "}", false),
        ]
    );
}

#[test]
fn an_excerpt_leaves_out_the_blank_lines_it_ends_on() {
    let excerpt = trail::excerpt(SOURCE, 3, |_| false);

    assert_eq!(excerpt, [line(3, "    x * 2", false), line(4, "}", false)]);
}

#[test]
fn an_excerpt_stops_where_the_file_does() {
    let excerpt = trail::excerpt("fn last() {}\n", 1, |_| true);

    assert_eq!(excerpt, [line(1, "fn last() {}", true)]);
}

#[test]
fn every_unit_shown_gets_one_excerpt_however_many_findings_it_carries() {
    let findings = [
        finding("src/a.rs", 2, Rule::CognitiveComplexity),
        finding("src/a.rs", 2, Rule::FunctionLines),
        finding("src/a.rs", 6, Rule::Parameters),
        finding("src/b.rs", 2, Rule::Parameters),
    ];

    assert_eq!(
        trail::units(&findings, 3),
        [("src/a.rs", 2), ("src/a.rs", 6)]
    );
}

#[test]
fn commits_are_read_as_their_short_hash_and_subject() {
    let log =
        "7f893ac\0test(server): checks the minimum\n5792121\0refactor: a\0subject with a nul\n";

    assert_eq!(
        trail::commits(log),
        [
            Commit {
                hash: "7f893ac".to_owned(),
                subject: "test(server): checks the minimum".to_owned(),
            },
            Commit {
                hash: "5792121".to_owned(),
                subject: "refactor: a\0subject with a nul".to_owned(),
            },
        ]
    );
}
