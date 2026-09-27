use std::fmt::Write as _;

use jabuti_core::catalog::{Concept, Rule, RuleId, Severity};
use jabuti_core::lang::{self, LanguageId};
use jabuti_core::model::{Detail, Finding, Span};
use jabuti_core::policy::{ConceptBindings, Policy, RuleConfig};

#[test]
fn the_default_policy_sets_every_rule_in_every_language() {
    let policy = Policy::default();
    let mut table = String::new();

    for spec in lang::ALL {
        for rule in Rule::ALL {
            let config = policy
                .config_for(spec.id, rule)
                .expect("every native rule has a default");
            writeln!(
                table,
                "{:<10} {:<22} {:<7} {}",
                spec.id.name(),
                rule.id(),
                config.severity.label(),
                config.limit
            )
            .expect("writing to a string never fails");
        }
    }

    insta::assert_snapshot!(table);
}

#[test]
fn only_a_rule_that_reports_is_active() {
    let policy = Policy::default();
    let active = |limit, severity| Some(RuleConfig { limit, severity });

    assert_eq!(
        [
            policy.active(Rule::DuplicateBlock),
            policy.active(Rule::Churn),
            policy.active_for(LanguageId::TypeScript, Rule::FunctionLines),
            policy.active_for(LanguageId::Rust, Rule::CyclomaticComplexity),
            policy.active(RuleId::parse("clippy/unwrap_used").expect("a tool lint")),
        ],
        [
            active(120, Severity::Warning),
            None,
            active(71, Severity::Warning),
            None,
            None,
        ]
    );
}

fn lint_at(path: &str) -> Finding {
    Finding {
        rule: RuleId::parse("clippy/unwrap_used").expect("a tool lint"),
        severity: Severity::Warning,
        path: path.to_owned(),
        span: Span {
            start_line: 3,
            end_line: 3,
        },
        subject: None,
        detail: Detail::Message {
            message: "used `unwrap()` on a `Result` value".to_owned(),
        },
    }
}

#[test]
fn a_tool_lint_follows_the_setting_for_the_language_of_its_file() {
    let lint = RuleId::parse("clippy/unwrap_used").expect("a tool lint");
    let mut policy = Policy::default();
    policy.set(
        lint.clone(),
        RuleConfig {
            limit: 0,
            severity: Severity::Error,
        },
    );
    policy.set_for(
        LanguageId::Rust,
        lint,
        RuleConfig {
            limit: 0,
            severity: Severity::Off,
        },
    );

    assert_eq!(
        [
            policy.admit(lint_at("src/lib.rs")),
            policy.admit(lint_at("build/Cargo.toml")),
        ],
        [
            None,
            Some(Finding {
                severity: Severity::Error,
                ..lint_at("build/Cargo.toml")
            }),
        ]
    );
}

#[test]
fn concept_bindings_report_global_and_per_language_emptiness() {
    let mut bindings = ConceptBindings::default();
    assert_eq!(
        (
            bindings.is_empty(),
            bindings.is_empty_for(LanguageId::Rust),
            bindings.is_empty_for(LanguageId::TypeScript),
        ),
        (true, true, true)
    );

    bindings.set(
        LanguageId::Rust,
        Concept::ErrorDiscard,
        vec!["mycorp::discard".to_owned()],
    );

    assert_eq!(
        (
            bindings.is_empty(),
            bindings.is_empty_for(LanguageId::Rust),
            bindings.is_empty_for(LanguageId::TypeScript),
        ),
        (false, false, true)
    );
}

#[test]
fn a_rule_is_enabled_where_any_language_reports_it_and_gates_where_any_fails_on_it() {
    let mut policy = Policy::default();
    policy.set_for(
        LanguageId::Kotlin,
        Rule::Churn,
        RuleConfig {
            limit: 0,
            severity: Severity::Error,
        },
    );
    let asked = |rule| (policy.enabled(rule), policy.gates(rule));

    assert_eq!(
        [
            asked(Rule::FunctionLines),
            asked(Rule::CyclomaticComplexity),
            asked(Rule::Churn),
        ],
        [(true, false), (false, false), (true, true)]
    );
}

#[test]
fn adjusting_per_language_reaches_every_calibrated_language_and_nothing_else() {
    let mut policy = Policy::default();

    policy.adjust_per_language(Rule::FunctionLines, |config| RuleConfig {
        severity: Severity::Error,
        ..config
    });

    let error = |limit| {
        Some(RuleConfig {
            limit,
            severity: Severity::Error,
        })
    };
    assert_eq!(
        [
            policy.config_for(LanguageId::TypeScript, Rule::FunctionLines),
            policy.config_for(LanguageId::Kotlin, Rule::FunctionLines),
            policy.config_for(LanguageId::Rust, Rule::FunctionLines),
            policy.config_for(LanguageId::TypeScript, Rule::CognitiveComplexity),
        ],
        [
            error(71),
            error(47),
            Some(RuleConfig {
                limit: 60,
                severity: Severity::Warning,
            }),
            Some(RuleConfig {
                limit: 18,
                severity: Severity::Warning,
            }),
        ]
    );
}
