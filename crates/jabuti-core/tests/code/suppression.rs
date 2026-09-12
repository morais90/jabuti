use jabuti_core::code::concepts::Occurrence;
use jabuti_core::code::suppression;
use jabuti_core::lang::LanguageId;
use jabuti_core::model::{Concept, Detail, Finding, Rule, RuleId, Severity, Span};
use jabuti_core::policy::{Policy, RuleConfig};
use rstest::rstest;

fn occurrence(subject: &str) -> Occurrence {
    Occurrence {
        concept: Concept::Suppression,
        subject: subject.to_owned(),
        span: Span {
            start_line: 3,
            end_line: 3,
        },
        in_test: false,
    }
}

#[test]
fn a_suppression_concept_becomes_a_complete_finding() {
    let found = suppression::findings(
        "src/catalog.rs",
        LanguageId::Rust,
        &[occurrence("(dead_code)")],
        &Policy::default(),
    );

    assert_eq!(
        found,
        [Finding {
            rule: RuleId::Native(Rule::Suppression),
            severity: Severity::Warning,
            path: "src/catalog.rs".to_owned(),
            span: Span {
                start_line: 3,
                end_line: 3,
            },
            subject: Some("dead_code".to_owned()),
            detail: Detail::Message {
                message: "a linter diagnostic is suppressed instead of satisfied".to_owned(),
            },
        }]
    );
}

#[test]
fn an_occurrence_of_a_different_concept_is_left_out() {
    let other = Occurrence {
        concept: Concept::ErrorPanic,
        subject: "unwrap".to_owned(),
        span: Span {
            start_line: 1,
            end_line: 1,
        },
        in_test: false,
    };

    let found = suppression::findings(
        "src/catalog.rs",
        LanguageId::Rust,
        &[other, occurrence("(dead_code)")],
        &Policy::default(),
    );

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].subject, Some("dead_code".to_owned()));
}

#[test]
fn a_rule_switched_off_reports_nothing_however_much_is_suppressed() {
    let mut policy = Policy::default();
    policy.set(
        Rule::Suppression,
        RuleConfig {
            limit: 0,
            severity: Severity::Off,
        },
    );

    assert_eq!(
        suppression::findings(
            "src/catalog.rs",
            LanguageId::Rust,
            &[occurrence("(dead_code)")],
            &policy,
        ),
        []
    );
}

#[rstest]
#[case(
    LanguageId::TypeScript,
    "any",
    "any",
    "a type-checker diagnostic is suppressed instead of satisfied"
)]
#[case(
    LanguageId::TypeScript,
    "// @ts-ignore",
    "ts-ignore",
    "a linter diagnostic is suppressed instead of satisfied"
)]
#[case(
    LanguageId::Rust,
    "any",
    "any",
    "a linter diagnostic is suppressed instead of satisfied"
)]
#[case(
    LanguageId::Kotlin,
    "(\"UNCHECKED_CAST\")",
    "\"UNCHECKED_CAST\"",
    "a linter diagnostic is suppressed instead of satisfied"
)]
fn the_message_depends_on_the_language_and_the_cleaned_subject(
    #[case] language: LanguageId,
    #[case] raw: &str,
    #[case] expected_subject: &str,
    #[case] expected_message: &str,
) {
    let found = suppression::findings("src/live", language, &[occurrence(raw)], &Policy::default());

    assert_eq!(found[0].subject, Some(expected_subject.to_owned()));
    assert_eq!(
        found[0].detail,
        Detail::Message {
            message: expected_message.to_owned()
        }
    );
}

#[rstest]
#[case("// @ts-nocheck", "ts-nocheck")]
#[case("// eslint-disable-next-line no-console", "no-console")]
#[case(
    "// eslint-disable-next-line no-console -- legacy call site",
    "no-console"
)]
#[case("// eslint-disable-next-line", "eslint-disable-next-line")]
#[case("// just a note", "just a note")]
fn a_typescript_comment_is_cleaned_to_the_subject_it_names(
    #[case] raw: &str,
    #[case] expected: &str,
) {
    let found = suppression::findings(
        "src/live.ts",
        LanguageId::TypeScript,
        &[occurrence(raw)],
        &Policy::default(),
    );

    assert_eq!(found[0].subject, Some(expected.to_owned()));
}
