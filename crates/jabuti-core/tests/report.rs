use std::collections::BTreeMap;
use std::ffi::OsStr;

use jabuti_core::catalog::{Rule, RuleId, UnitKind};
use jabuti_core::model::{Detail, Finding, Reading, Span, Unreadable};
use jabuti_core::report::trail::{Commit, Line, Trail};
use jabuti_core::report::{self, Outcome, Rhythm, Scanned};

fn finding(line: u32, subject: Option<&str>) -> Finding {
    Finding {
        rule: RuleId::Native(Rule::FunctionLines),
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

fn tool_finding() -> Finding {
    Finding {
        rule: RuleId::External {
            tool: "clippy".to_owned(),
            lint: "needless_range_loop".to_owned(),
        },
        path: "src/lib.rs".to_owned(),
        span: Span {
            start_line: 3,
            end_line: 3,
        },
        subject: None,
        detail: Detail::Message {
            message: "the loop variable is only used to index".to_owned(),
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
    let rendered = report::agent(judged(&[], &[]), 40);

    assert_eq!(rendered, "No findings across 42 files and 378 units.\n");
}

#[test]
fn a_finding_names_its_rule_location_and_both_numbers() {
    let rendered = report::agent(judged(&[finding(120, Some("handle_request"))], &[]), 40);

    assert_eq!(
        rendered,
        "1 finding to fix across 42 files and 378 units.\n\
         \n\
         src/handler.rs:120  function-lines  handle_request  measured 71, limit 60\n"
    );
}

#[test]
fn a_finding_without_a_subject_leaves_no_gap_where_the_name_would_be() {
    let rendered = report::agent(judged(&[finding(1, None)], &[]), 40);

    assert!(
        rendered.ends_with("src/handler.rs:1  function-lines  measured 71, limit 60\n"),
        "{rendered}"
    );
}

#[test]
fn output_beyond_the_limit_is_replaced_by_a_count_of_what_was_withheld() {
    let findings: Vec<Finding> = (1..=10).map(|line| finding(line, Some("wide"))).collect();

    let rendered = report::agent(judged(&findings, &[]), 3);

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
fn any_finding_fails_the_gate() {
    assert!(!report::fails(&[]));
    assert!(report::fails(&[finding(1, None)]));
}

#[test]
fn the_json_report_carries_a_schema_a_summary_and_the_findings() {
    let rendered = report::json(judged(&[finding(120, Some("handle_request"))], &[]));

    insta::assert_snapshot!(rendered);
}

#[test]
fn a_finding_from_a_tool_serialises_its_message_instead_of_a_threshold() {
    let reported = tool_finding();

    let rendered = report::json(judged(&[reported], &[]));

    assert!(
        rendered.contains("\"rule\": \"clippy/needless_range_loop\""),
        "{rendered}"
    );
    assert!(
        rendered.contains("\"message\": \"the loop variable is only used to index\""),
        "{rendered}"
    );
    assert!(!rendered.contains("measured"), "{rendered}");
}

#[test]
fn the_sarif_report_carries_every_finding_and_unreadable_file() {
    let findings = [finding(120, Some("handle_request")), tool_finding()];
    let unreadable = [
        unreadable("src/theme.kt", 51),
        unreadable("src/nav.kt", 314),
    ];

    let rendered = report::sarif(judged(&findings, &unreadable));

    insta::assert_snapshot!(rendered);
}

#[test]
fn a_clean_sarif_report_keeps_the_complete_empty_envelope() {
    let rendered = report::sarif(judged(&[], &[]));

    let expected = r#"{
  "$schema": "https://docs.oasis-open.org/sarif/sarif/v2.1.0/errata01/os/schemas/sarif-schema-2.1.0.json",
  "version": "2.1.0",
  "runs": [
    {
      "tool": {
        "driver": {
          "name": "jabuti",
          "semanticVersion": "0.1.0",
          "informationUri": "https://github.com/morais90/jabuti",
          "rules": []
        }
      },
      "invocations": [
        {
          "executionSuccessful": true,
          "toolExecutionNotifications": []
        }
      ],
      "results": []
    }
  ]
}
"#;

    assert_eq!(rendered, expected);
}

#[test]
fn sarif_artifact_uris_encode_bytes_and_normalise_the_platform_separator() {
    let mut reported = finding(7, None);
    reported.path = format!(
        "src{}safe-._~ space#100% café.rs",
        std::path::MAIN_SEPARATOR
    );

    let rendered = report::sarif(judged(&[reported], &[]));

    insta::assert_snapshot!(rendered);
}

#[test]
fn sarif_rule_indices_point_into_lexically_sorted_descriptors() {
    let findings = [finding(120, Some("handle_request")), tool_finding()];
    let rendered = report::sarif(judged(&findings, &[]));
    let document: serde_json::Value = serde_json::from_str(&rendered).expect("valid JSON");
    let run = &document["runs"][0];

    assert_eq!(
        run["tool"]["driver"]["rules"],
        serde_json::json!([
            { "id": "clippy/needless_range_loop" },
            { "id": "function-lines" }
        ])
    );
    assert_eq!(run["results"][0]["ruleId"], "function-lines");
    assert_eq!(run["results"][0]["ruleIndex"], 1);
    assert_eq!(run["results"][1]["ruleId"], "clippy/needless_range_loop");
    assert_eq!(run["results"][1]["ruleIndex"], 0);
}

#[test]
fn structurally_distinct_rules_with_the_same_public_id_share_one_descriptor() {
    let mut first = tool_finding();
    first.rule = RuleId::External {
        tool: "a".to_owned(),
        lint: "b/c".to_owned(),
    };
    let mut second = tool_finding();
    second.rule = RuleId::External {
        tool: "a/b".to_owned(),
        lint: "c".to_owned(),
    };
    let rendered = report::sarif(judged(&[first, second], &[]));
    let document: serde_json::Value = serde_json::from_str(&rendered).expect("valid JSON");
    let run = &document["runs"][0];

    assert_eq!(
        run["tool"]["driver"]["rules"],
        serde_json::json!([{ "id": "a/b/c" }])
    );
    assert_eq!(run["results"][0]["ruleId"], "a/b/c");
    assert_eq!(run["results"][0]["ruleIndex"], 0);
    assert_eq!(run["results"][1]["ruleId"], "a/b/c");
    assert_eq!(run["results"][1]["ruleIndex"], 0);
}

#[test]
fn measures_are_reported_for_every_unit_whatever_the_thresholds_say() {
    let rendered = report::measures(
        &[Reading {
            path: "src/lib.rs".to_owned(),
            line: 1,
            subject: Some("wide".to_owned()),
            kind: UnitKind::Function,
            values: BTreeMap::from([("parameters", 5), ("lines", 3)]),
        }],
        &[],
    );

    insta::assert_snapshot!(rendered);
}

fn unreadable(path: &str, line: u32) -> Unreadable {
    Unreadable {
        path: path.to_owned(),
        reason: format!("unreadable syntax from line {line}"),
    }
}

#[test]
fn a_clean_run_still_names_every_file_it_could_not_read() {
    let rendered = report::agent(judged(&[], &[unreadable("src/theme.kt", 51)]), 40);

    assert_eq!(
        rendered,
        "No findings across 42 files and 378 units.\n\
         \n\
         1 file was not measured, so nothing above accounts for it.\n\
         src/theme.kt  unreadable syntax from line 51\n"
    );
}

#[test]
fn unreadable_files_are_listed_after_the_findings_they_are_not_part_of() {
    let rendered = report::agent(
        judged(
            &[finding(120, Some("handle_request"))],
            &[
                unreadable("src/theme.kt", 51),
                unreadable("src/nav.kt", 314),
            ],
        ),
        40,
    );

    assert_eq!(
        rendered,
        "1 finding to fix across 42 files and 378 units.\n\
         \n\
         src/handler.rs:120  function-lines  handle_request  measured 71, limit 60\n\
         \n\
         2 files were not measured, so nothing above accounts for them.\n\
         src/theme.kt  unreadable syntax from line 51\n\
         src/nav.kt  unreadable syntax from line 314\n"
    );
}

#[test]
fn the_list_of_unreadable_files_obeys_the_same_limit_the_findings_do() {
    let skipped: Vec<Unreadable> = (1..=10)
        .map(|line| unreadable("src/wide.kt", line))
        .collect();

    let rendered = report::agent(judged(&[], &skipped), 3);

    assert_eq!(
        rendered
            .lines()
            .filter(|line| line.starts_with("src/wide.kt"))
            .count(),
        3
    );
    assert!(
        rendered.contains("7 further files not shown."),
        "{rendered}"
    );
}

#[test]
fn the_json_report_carries_every_unreadable_file_and_counts_them_in_the_summary() {
    let rendered = report::json(judged(&[], &[unreadable("src/theme.kt", 51)]));

    insta::assert_snapshot!(rendered);
}

#[test]
fn measures_name_the_files_no_measurement_could_be_taken_from() {
    let rendered = report::measures(&[], &[unreadable("src/theme.kt", 51)]);

    insta::assert_snapshot!(rendered);
}

fn reading(path: &str, line: u32, kind: UnitKind) -> Reading {
    Reading {
        path: path.to_owned(),
        line,
        subject: None,
        kind,
        values: BTreeMap::from([("lines", line)]),
    }
}

#[test]
fn an_outcome_puts_its_findings_readings_and_unreadable_files_in_one_order() {
    let mut outcome = Outcome {
        findings: vec![
            finding(120, Some("handle_request")),
            tool_finding(),
            finding(12, Some("parse")),
        ],
        context: vec![note("src/b.rs"), note("src/a.rs")],
        readings: vec![
            reading("app-b/c.rs", 1, UnitKind::File),
            reading("app/c.rs", 1, UnitKind::File),
            reading("src/b.rs", 1, UnitKind::File),
            reading("src/b.rs", 1, UnitKind::Function),
            reading("src/a.rs", 1, UnitKind::File),
            reading("src/a.rs", 9, UnitKind::Function),
            reading("src/a.rs", 4, UnitKind::Function),
        ],
        scanned: scanned(),
        unreadable: vec![unreadable("src/theme.kt", 51), unreadable("src/app.ts", 3)],
    };

    outcome.order();

    assert_eq!(
        outcome,
        Outcome {
            findings: vec![
                finding(12, Some("parse")),
                finding(120, Some("handle_request")),
                tool_finding(),
            ],
            context: vec![note("src/a.rs"), note("src/b.rs")],
            readings: vec![
                reading("app/c.rs", 1, UnitKind::File),
                reading("app-b/c.rs", 1, UnitKind::File),
                reading("src/a.rs", 1, UnitKind::File),
                reading("src/a.rs", 9, UnitKind::Function),
                reading("src/a.rs", 4, UnitKind::Function),
                reading("src/b.rs", 1, UnitKind::File),
                reading("src/b.rs", 1, UnitKind::Function),
            ],
            scanned: scanned(),
            unreadable: vec![unreadable("src/app.ts", 3), unreadable("src/theme.kt", 51)],
        }
    );
}

type Told = (Rule, &'static str, u32, Option<&'static str>, &'static str);

fn measured((rule, path, line, subject, values): Measured) -> Finding {
    Finding {
        rule: RuleId::Native(rule),
        path: path.to_owned(),
        span: Span {
            start_line: line,
            end_line: line + 20,
        },
        subject: Some(subject.to_owned()),
        detail: Detail::Threshold {
            measured: values.0,
            limit: values.1,
        },
    }
}

fn told((rule, path, line, subject, message): Told) -> Finding {
    Finding {
        rule: RuleId::Native(rule),
        path: path.to_owned(),
        span: Span {
            start_line: line,
            end_line: line,
        },
        subject: subject.map(str::to_owned),
        detail: Detail::Message {
            message: message.to_owned(),
        },
    }
}

type Measured = (Rule, &'static str, u32, &'static str, (u32, u32));

const MEASURED: [Measured; 6] = [
    (
        Rule::CognitiveComplexity,
        "registry/src/ai.rs",
        469,
        "remembered",
        (8, 7),
    ),
    (
        Rule::FunctionLines,
        "registry/src/ai.rs",
        525,
        "chat",
        (108, 60),
    ),
    (
        Rule::FunctionLines,
        "server/src/api/assistant.rs",
        564,
        "turn",
        (167, 60),
    ),
    (
        Rule::CognitiveComplexity,
        "server/src/api/assistant.rs",
        564,
        "turn",
        (31, 7),
    ),
    (
        Rule::Parameters,
        "server/src/api/assistant.rs",
        762,
        "store",
        (5, 4),
    ),
    (
        Rule::FunctionLines,
        "server/src/api/policy.rs",
        897,
        "updates",
        (61, 60),
    ),
];

fn change() -> Vec<Finding> {
    let mut findings: Vec<Finding> = MEASURED.into_iter().map(measured).collect();
    findings.push(told((
        Rule::Suppression,
        "server/src/api/policy.rs",
        161,
        Some("clippy::struct_excessive_bools, reason = \"one switch per device\""),
        "a linter diagnostic is suppressed instead of satisfied",
    )));
    findings.push(told((
        Rule::DuplicateBlock,
        "server/src/api/policy.rs",
        456,
        None,
        "157 nodes repeated at server/src/storage.rs:115 (limit 120)",
    )));
    findings.push(tool_finding());
    findings.sort();
    findings
}

fn shown_escapes(rendered: &str) -> String {
    rendered.replace('\x1b', "␛")
}

fn judged<'a>(findings: &'a [Finding], unreadable: &'a [Unreadable]) -> report::Judgement<'a> {
    report::Judgement {
        findings,
        context: &[],
        unreadable,
        scanned: scanned(),
    }
}

fn view(since: Option<&str>, color: bool) -> report::View<'_> {
    report::View {
        since,
        color,
        links: Some(std::path::Path::new("/work/ünïcode project")),
        rhythm: None,
        trail: None,
    }
}

fn rhythms() -> BTreeMap<String, Rhythm> {
    BTreeMap::from([
        (
            "server/src/api/policy.rs".to_owned(),
            Rhythm {
                weeks: [0, 0, 0, 0, 0, 0, 0, 13, 5, 7, 8, 7],
            },
        ),
        (
            "server/src/api/assistant.rs".to_owned(),
            Rhythm {
                weeks: [0, 0, 0, 0, 0, 0, 0, 4, 5, 1, 6, 4],
            },
        ),
        (
            "registry/src/ai.rs".to_owned(),
            Rhythm {
                weeks: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 4],
            },
        ),
    ])
}

#[test]
fn a_human_report_groups_findings_by_file_with_one_aligned_line_each() {
    let rendered = report::human(
        judged(&change(), &[unreadable("src/api/index.ts", 3)]),
        40,
        view(Some("main"), false),
    );

    insta::assert_snapshot!(rendered);
}

#[test]
fn a_colored_human_report_paints_and_links_what_the_plain_one_writes() {
    let findings = change();
    let colored = report::human(judged(&findings, &[]), 40, view(None, true));
    let plain = report::human(judged(&findings, &[]), 40, view(None, false));

    let stripped = strip_escapes(&colored);
    let (body, footer) = stripped.trim_end().rsplit_once('\n').expect("a footer");
    let (plain_body, plain_footer) = plain.trim_end().rsplit_once('\n').expect("a footer");

    assert_eq!(body, plain_body);
    assert_eq!(footer.replacen("   ", " · ", 1).trim_start(), plain_footer);
    insta::assert_snapshot!(shown_escapes(&colored));
}

fn strip_escapes(rendered: &str) -> String {
    let mut plain = String::new();
    let mut rest = rendered;
    while let Some(start) = rest.find('\x1b') {
        plain.push_str(&rest[..start]);
        let escape = &rest[start..];
        let end = if escape.starts_with("\x1b]") {
            escape.find("\x1b\\").map(|close| close + 2)
        } else {
            escape.find('m').map(|close| close + 1)
        };
        rest = &escape[end.expect("every escape is closed")..];
    }
    plain.push_str(rest);
    plain
}

#[test]
fn a_clean_human_run_is_one_line_naming_the_scope_and_its_seal() {
    let rendered = report::human(judged(&[], &[]), 40, view(Some("main"), false));

    assert_eq!(
        rendered,
        "⬢ jabuti  since main · nothing to fix · 42 files · 378 units · seal 40f3·adcd\n"
    );
}

#[test]
fn a_clean_human_run_still_names_every_file_it_could_not_read() {
    let rendered = report::human(
        judged(&[], &[unreadable("src/broken.rs", 1)]),
        40,
        view(None, false),
    );

    insta::assert_snapshot!(rendered);
}

#[test]
fn a_human_report_beyond_the_limit_says_how_many_findings_it_withheld() {
    let rendered = report::human(judged(&change(), &[]), 3, view(None, false));

    insta::assert_snapshot!(rendered);
}

#[test]
fn the_seal_changes_with_anything_the_verdict_contains() {
    let findings = change();
    let seal = |findings: &[Finding], unreadable: &[Unreadable]| {
        let rendered = report::human(judged(findings, unreadable), 40, view(None, false));
        rendered
            .lines()
            .last()
            .map(str::to_owned)
            .expect("a footer")
    };

    let whole = seal(&findings, &[]);

    assert_eq!(whole, seal(&findings, &[]));
    assert_ne!(whole, seal(&findings[1..], &[]));
    assert_ne!(
        whole,
        seal(&findings, &[unreadable("src/broken.rs", 1)]).replace("1 unreadable · ", "")
    );
}

#[test]
fn color_is_used_on_a_terminal_unless_no_color_is_set_to_something() {
    let unset: Option<&OsStr> = None;

    assert!(report::colored(true, unset));
    assert!(report::colored(true, Some(OsStr::new(""))));
    assert!(!report::colored(true, Some(OsStr::new("1"))));
    assert!(!report::colored(false, unset));
}

#[test]
fn a_terminal_that_declares_itself_dumb_is_not_read_by_a_person() {
    let unset: Option<&OsStr> = None;

    assert!(report::interactive(true, unset));
    assert!(report::interactive(true, Some(OsStr::new("xterm-ghostty"))));
    assert!(!report::interactive(true, Some(OsStr::new("dumb"))));
    assert!(!report::interactive(
        false,
        Some(OsStr::new("xterm-ghostty"))
    ));
}

#[test]
fn text_taken_from_the_repository_cannot_drive_the_terminal() {
    let findings = [told((
        Rule::Suppression,
        "src/\x1b[2Jlib.rs",
        3,
        Some("dead_code, reason = \"\x1b]52;c;aGk=\x07\""),
        "a linter diagnostic is suppressed instead of satisfied",
    ))];

    let rendered = report::human(judged(&findings, &[]), 40, view(Some("\x1bmain"), true));

    insta::assert_snapshot!(shown_escapes(&rendered));
}

#[test]
fn findings_without_a_subject_on_one_line_each_keep_their_line() {
    let findings = [
        told((
            Rule::DuplicateBlock,
            "src/lib.rs",
            1,
            None,
            "157 nodes repeated at src/main.rs:4",
        )),
        told((
            Rule::Assertion,
            "src/lib.rs",
            1,
            None,
            "the test runs and asserts nothing",
        )),
    ];

    let rendered = report::human(judged(&findings, &[]), 40, view(None, false));

    assert_eq!(rendered.matches("\n  :1  ").count(), 2, "{rendered}");
}

#[test]
fn two_units_sharing_a_name_each_keep_their_own_line_and_name() {
    let findings = [
        measured((Rule::Parameters, "src/lib.rs", 12, "new", (6, 4))),
        measured((Rule::Parameters, "src/lib.rs", 48, "new", (5, 4))),
    ];

    let rendered = report::human(judged(&findings, &[]), 40, view(None, false));

    insta::assert_snapshot!(rendered);
}

#[test]
fn a_human_report_ends_with_how_many_findings_there_are_to_fix() {
    let findings = [measured((
        Rule::FunctionLines,
        "src/lib.rs",
        1,
        "wide",
        (22, 5),
    ))];

    let rendered = report::human(judged(&findings, &[]), 40, view(None, false));

    assert!(rendered.contains("\n1 to fix · seal "), "{rendered}");
}

#[test]
fn each_file_leads_with_its_weekly_commits_and_the_busiest_comes_first() {
    let rhythms = rhythms();
    let view = report::View {
        rhythm: Some(&rhythms),
        ..view(None, false)
    };

    let rendered = report::human(judged(&change(), &[]), 40, view);

    insta::assert_snapshot!(rendered);
}

#[test]
fn quiet_weeks_are_dimmed_and_busy_ones_painted() {
    let rhythms = rhythms();
    let view = report::View {
        rhythm: Some(&rhythms),
        ..view(None, true)
    };
    let findings = [measured((
        Rule::Parameters,
        "registry/src/ai.rs",
        223,
        "bill",
        (5, 4),
    ))];

    let rendered = report::human(judged(&findings, &[]), 40, view);

    insta::assert_snapshot!(shown_escapes(&rendered));
}

#[test]
fn the_files_shown_are_those_of_the_findings_within_the_limit_each_once() {
    let findings = change();

    assert_eq!(
        report::shown_files(&judged(&findings, &[]), 5),
        ["registry/src/ai.rs", "server/src/api/assistant.rs"]
    );
}

#[test]
fn the_note_of_what_was_withheld_lines_up_with_the_rows_under_a_rhythm() {
    let rhythms = rhythms();
    let view = report::View {
        rhythm: Some(&rhythms),
        ..view(None, false)
    };

    let rendered = report::human(judged(&change(), &[]), 2, view);

    assert!(
        rendered.contains("\n              7 more not shown."),
        "{rendered}"
    );
}

fn code(number: u32, text: &str, changed: bool) -> Line {
    Line {
        number,
        text: text.to_owned(),
        changed,
    }
}

fn commit(hash: &str, subject: &str) -> Commit {
    Commit {
        hash: hash.to_owned(),
        subject: subject.to_owned(),
    }
}

fn trail() -> Trail {
    Trail {
        excerpts: BTreeMap::from([
            (
                ("registry/src/ai.rs".to_owned(), 469),
                vec![
                    code(469, "fn remembered(turns: &[Turn]) -> Vec<Turn> {", false),
                    code(470, "\tlet kept = turns.len().min(KEPT);", true),
                    code(471, "    turns[turns.len() - kept..].to_vec()", true),
                ],
            ),
            (
                ("registry/src/ai.rs".to_owned(), 525),
                vec![code(
                    525,
                    "pub async fn chat(state: State, body: Json<Ask>) -> Response {",
                    false,
                )],
            ),
        ]),
        commits: vec![
            commit(
                "7f893ac",
                "feat(registry): remember the last turns of a chat",
            ),
            commit("5792121", "refactor: split the chat handler"),
        ],
    }
}

#[test]
fn a_verbose_report_opens_with_the_commits_and_shows_the_code_of_each_unit() {
    let trail = trail();
    let view = report::View {
        trail: Some(&trail),
        ..view(Some("main"), false)
    };

    let rendered = report::human(judged(&change(), &[]), 40, view);

    insta::assert_snapshot!(rendered);
}

#[test]
fn a_verbose_report_paints_changed_lines_apart_from_their_context() {
    let trail = trail();
    let view = report::View {
        trail: Some(&trail),
        ..view(Some("main"), true)
    };

    let rendered = report::human(judged(&change(), &[]), 40, view);

    insta::assert_snapshot!(shown_escapes(&rendered));
}

#[test]
fn the_timeline_shows_five_commits_and_counts_the_rest() {
    let commits = |count: usize| Trail {
        excerpts: BTreeMap::new(),
        commits: (0..count)
            .map(|index| commit(&format!("{index:07}"), "work"))
            .collect(),
    };
    let ending = |count: usize| {
        let trail = commits(count);
        let view = report::View {
            trail: Some(&trail),
            ..view(None, false)
        };
        let rendered = report::human(judged(&change(), &[]), 40, view);
        let lines: Vec<&str> = rendered
            .lines()
            .skip(1)
            .take_while(|line| !line.is_empty())
            .collect();
        (lines.len(), lines[lines.len() - 2].to_owned())
    };

    assert_eq!(ending(5), (6, "  │ 0000004  work".to_owned()));
    assert_eq!(ending(6), (7, "  │ 1 more commit".to_owned()));
    assert_eq!(ending(7), (7, "  │ 2 more commits".to_owned()));
}

#[test]
fn without_commits_in_the_change_there_is_no_timeline() {
    let trail = Trail::default();
    let view = report::View {
        trail: Some(&trail),
        ..view(None, false)
    };

    let rendered = report::human(judged(&change(), &[]), 40, view);

    assert!(
        rendered.starts_with("⬢ jabuti  42 files · 378 units\n\n"),
        "{rendered}"
    );
}

#[test]
fn a_long_line_of_code_is_clipped_to_seventy_two_columns() {
    let long = "x".repeat(73);
    let exact = "y".repeat(72);
    let trail = Trail {
        excerpts: BTreeMap::from([(
            ("registry/src/ai.rs".to_owned(), 469),
            vec![code(469, &exact, false), code(470, &long, false)],
        )]),
        commits: Vec::new(),
    };
    let view = report::View {
        trail: Some(&trail),
        ..view(None, false)
    };

    let rendered = report::human(judged(&change(), &[]), 40, view);

    assert!(
        rendered.contains(&format!(" 469 │ {exact}\n")),
        "{rendered}"
    );
    assert!(
        rendered.contains(&format!(" 470 │ {}…\n", "x".repeat(71))),
        "{rendered}"
    );
}

#[test]
fn code_and_commit_subjects_cannot_reach_the_terminal_as_control_sequences() {
    let trail = Trail {
        excerpts: BTreeMap::from([(
            ("registry/src/ai.rs".to_owned(), 469),
            vec![code(469, "let bell = \"\x07\x1b[2J\";", true)],
        )]),
        commits: vec![commit("7f893ac", "clear \x1b]0;title\x07 the screen")],
    };
    let view = report::View {
        trail: Some(&trail),
        ..view(None, false)
    };

    let rendered = report::human(judged(&change(), &[]), 40, view);

    assert!(
        rendered.contains(" 469 ┃ let bell = \"\u{fffd}\u{fffd}[2J\";\n"),
        "{rendered}"
    );
    assert!(
        rendered.contains("7f893ac  clear \u{fffd}]0;title\u{fffd} the screen\n"),
        "{rendered}"
    );
}

#[test]
fn code_cannot_reorder_itself_on_screen_with_bidirectional_controls() {
    let trail = Trail {
        excerpts: BTreeMap::from([(
            ("registry/src/ai.rs".to_owned(), 469),
            vec![code(
                469,
                "/*\u{202e} } if admin {\u{2066} */ \u{200f}x\u{2069}",
                false,
            )],
        )]),
        commits: Vec::new(),
    };
    let view = report::View {
        trail: Some(&trail),
        ..view(None, false)
    };

    let rendered = report::human(judged(&change(), &[]), 40, view);

    assert!(
        rendered.contains(" 469 │ /*\u{fffd} } if admin {\u{fffd} */ \u{fffd}x\u{fffd}\n"),
        "{rendered}"
    );
}

#[test]
fn a_clean_verbose_report_still_shows_the_commits_of_the_change() {
    let trail = Trail {
        excerpts: BTreeMap::new(),
        commits: vec![commit("7f893ac", "feat: a clean change")],
    };
    let view = report::View {
        trail: Some(&trail),
        ..view(Some("main"), false)
    };

    let rendered = report::human(judged(&[], &[]), 40, view);

    assert_eq!(
        rendered,
        "⬢ jabuti  since main · nothing to fix · 42 files · 378 units · seal 40f3·adcd\n  ┬ 7f893ac  feat: a clean change\n  ┴\n"
    );
}

#[test]
fn a_unit_whose_code_could_not_be_read_leaves_no_gap_behind_it() {
    let trail = Trail {
        excerpts: BTreeMap::from([(("registry/src/ai.rs".to_owned(), 469), Vec::new())]),
        commits: Vec::new(),
    };
    let view = report::View {
        trail: Some(&trail),
        ..view(None, false)
    };

    let rendered = report::human(judged(&change(), &[]), 40, view);

    assert!(rendered.contains("8/7\n  :525  chat"), "{rendered}");
}

fn note(path: &str) -> Finding {
    Finding {
        rule: RuleId::Native(Rule::NewDependency),
        path: path.to_owned(),
        span: Span {
            start_line: 2,
            end_line: 2,
        },
        subject: None,
        detail: Detail::Message {
            message: "now depends on src/git.rs".to_owned(),
        },
    }
}

fn hotspot(path: &str) -> Finding {
    Finding {
        rule: RuleId::Native(Rule::Hotspot),
        path: path.to_owned(),
        span: Span {
            start_line: 1,
            end_line: 40,
        },
        subject: None,
        detail: Detail::Threshold {
            measured: 92,
            limit: 90,
        },
    }
}

fn with_context<'a>(findings: &'a [Finding], context: &'a [Finding]) -> report::Judgement<'a> {
    report::Judgement {
        context,
        ..judged(findings, &[])
    }
}

#[test]
fn an_outcome_sets_what_does_not_fail_the_run_apart_from_its_findings() {
    let mut outcome = Outcome {
        findings: vec![
            note("src/a.rs"),
            finding(12, Some("parse")),
            hotspot("src/a.rs"),
        ],
        ..Outcome::default()
    };

    outcome.set_context_apart();

    assert_eq!(
        outcome,
        Outcome {
            findings: vec![finding(12, Some("parse"))],
            context: vec![note("src/a.rs"), hotspot("src/a.rs")],
            ..Outcome::default()
        }
    );
}

#[test]
fn the_files_shown_include_those_with_only_context_each_once_in_path_order() {
    let findings = [finding(12, Some("parse"))];
    let context = [
        note("src/report.rs"),
        hotspot("src/report.rs"),
        note("src/a.rs"),
    ];

    assert_eq!(
        report::shown_files(&with_context(&findings, &context), 40),
        ["src/a.rs", "src/handler.rs", "src/report.rs"]
    );
}

#[test]
fn the_agent_lists_context_apart_after_what_there_is_to_fix() {
    let findings = [finding(12, Some("parse"))];
    let context = [hotspot("src/report.rs"), note("src/report.rs")];

    insta::assert_snapshot!(report::agent(with_context(&findings, &context), 40));
}

#[test]
fn the_agent_shows_context_even_when_there_is_nothing_to_fix() {
    let context = [note("src/report.rs")];

    insta::assert_snapshot!(report::agent(with_context(&[], &context), 40));
}

#[test]
fn json_carries_the_context_beside_the_findings() {
    let context = [note("src/report.rs")];

    insta::assert_snapshot!(report::json(with_context(&[], &context)));
}

#[test]
fn sarif_marks_context_as_a_note_and_findings_as_errors() {
    let findings = [finding(12, Some("parse"))];
    let context = [note("src/report.rs")];

    insta::assert_snapshot!(report::sarif(with_context(&findings, &context)));
}

#[test]
fn the_human_report_shows_context_under_its_file_and_counts_it_apart() {
    let findings = [finding(12, Some("parse"))];
    let context = [
        hotspot("src/report.rs"),
        note("src/report.rs"),
        note("src/server/api.rs"),
    ];

    insta::assert_snapshot!(report::human(
        with_context(&findings, &context),
        40,
        view(None, false)
    ));
}

#[test]
fn the_human_report_with_only_context_says_there_is_nothing_to_fix() {
    let context = [note("src/report.rs")];

    insta::assert_snapshot!(report::human(
        with_context(&[], &context),
        40,
        view(None, false)
    ));
}

#[test]
fn the_agent_says_how_many_notes_of_context_it_withheld() {
    let context = [note("src/a.rs"), note("src/b.rs"), note("src/c.rs")];

    assert_eq!(
        report::agent(with_context(&[], &context), 2),
        "No findings across 42 files and 378 units.\n\n\
         Context, which does not fail the run:\n\
         src/a.rs:2  new-dependency  now depends on src/git.rs\n\
         src/b.rs:2  new-dependency  now depends on src/git.rs\n\
         1 further note not shown.\n"
    );
}

#[test]
fn an_outcome_is_judged_on_exactly_what_it_holds() {
    let outcome = Outcome {
        findings: vec![finding(12, Some("parse"))],
        context: vec![note("src/a.rs")],
        unreadable: vec![unreadable("src/theme.kt", 51)],
        scanned: scanned(),
        ..Outcome::default()
    };

    let judgement = outcome.judgement();

    assert_eq!(
        (
            judgement.findings,
            judgement.context,
            judgement.unreadable,
            judgement.scanned
        ),
        (
            &[finding(12, Some("parse"))][..],
            &[note("src/a.rs")][..],
            &[unreadable("src/theme.kt", 51)][..],
            scanned()
        )
    );
}

#[test]
fn the_human_report_counts_withheld_notes_of_context_too() {
    let findings = [finding(12, Some("parse"))];
    let context = [note("src/a.rs"), note("src/b.rs"), note("src/c.rs")];

    let rendered = report::human(with_context(&findings, &context), 1, view(None, false));

    assert!(
        rendered.contains("  2 more not shown. Narrow the scope with a path argument.\n"),
        "{rendered}"
    );
}

#[test]
fn a_blank_line_of_code_under_a_unit_ends_at_its_gutter() {
    let trail = Trail {
        excerpts: BTreeMap::from([(
            ("registry/src/ai.rs".to_owned(), 469),
            vec![
                code(469, "fn remembered(turns: &[Turn]) -> Vec<Turn> {", false),
                code(470, "", false),
            ],
        )]),
        commits: Vec::new(),
    };
    let view = report::View {
        trail: Some(&trail),
        ..view(None, false)
    };

    let rendered = report::human(judged(&change(), &[]), 40, view);

    assert!(rendered.contains("      470 │\n"), "{rendered}");
}
