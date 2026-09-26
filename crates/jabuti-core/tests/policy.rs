use std::fmt::Write as _;

use jabuti_core::lang::{self, LanguageId};
use jabuti_core::model::{Rule, RuleId, Severity};
use jabuti_core::policy::{Policy, RuleConfig};

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
