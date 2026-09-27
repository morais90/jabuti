use jabuti_core::catalog::{Concept, Rule, RuleId, Severity};
use jabuti_core::code::concepts::Occurrence;
use jabuti_core::code::masking;
use jabuti_core::lang::LanguageId;
use jabuti_core::model::{Detail, Finding, Span};
use jabuti_core::policy::{Policy, RuleConfig};

fn panic_occurrence() -> Occurrence {
    Occurrence {
        concept: Concept::ErrorPanic,
        subject: "unwrap".to_owned(),
        span: Span {
            start_line: 5,
            end_line: 5,
        },
        in_test: false,
    }
}

#[test]
fn a_masking_concept_becomes_a_complete_finding() {
    let found = masking::findings(
        "src/catalog.rs",
        LanguageId::Rust,
        &[panic_occurrence()],
        &Policy::default(),
    );

    assert_eq!(
        found,
        [Finding {
            rule: RuleId::Native(Rule::ErrorMasking),
            severity: Severity::Warning,
            path: "src/catalog.rs".to_owned(),
            span: Span {
                start_line: 5,
                end_line: 5,
            },
            subject: Some("unwrap".to_owned()),
            detail: Detail::Message {
                message: "the failure becomes a panic".to_owned(),
            },
        }]
    );
}

#[test]
fn an_occurrence_tagged_as_test_code_produces_no_finding() {
    let occurrence = Occurrence {
        in_test: true,
        ..panic_occurrence()
    };

    assert_eq!(
        masking::findings(
            "tests/behaviour.rs",
            LanguageId::Rust,
            &[occurrence],
            &Policy::default(),
        ),
        []
    );
}

#[test]
fn a_rule_switched_off_reports_nothing_however_much_is_masked() {
    let mut policy = Policy::default();
    policy.set(
        Rule::ErrorMasking,
        RuleConfig {
            limit: 0,
            severity: Severity::Off,
        },
    );

    assert_eq!(
        masking::findings(
            "src/catalog.rs",
            LanguageId::Rust,
            &[panic_occurrence()],
            &policy,
        ),
        []
    );
}
