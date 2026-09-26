use std::fmt::Write as _;

use jabuti_core::lang;
use jabuti_core::model::Rule;
use jabuti_core::policy::Policy;

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
