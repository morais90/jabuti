use std::collections::BTreeMap;

use jabuti_core::catalog::{Rule, Severity};
use jabuti_core::code::{assertion, concepts, units};
use jabuti_core::lang::{self, LangSpec};
use jabuti_core::model::Finding;
use jabuti_core::policy::{ConceptBindings, Policy, RuleConfig};
use jabuti_core::syntax;

fn findings_for(source: &str, spec: &'static LangSpec, policy: &Policy) -> Vec<Finding> {
    let parsed = syntax::parse(source, spec).expect("source parses cleanly");
    let unit = units::units(&parsed);
    let occurrences = concepts::occurrences(&parsed, &ConceptBindings::default(), &BTreeMap::new());

    assertion::findings(
        &assertion::Input {
            path: "src/live",
            parsed: &parsed,
            unit: &unit,
            occurrences: &occurrences,
        },
        policy,
    )
}

fn subjects(findings: &[Finding]) -> Vec<Option<String>> {
    findings
        .iter()
        .map(|finding| finding.subject.clone())
        .collect()
}

#[test]
fn an_empty_rust_test_is_reported() {
    let findings = findings_for("#[test]\nfn checks() {}\n", &lang::RUST, &Policy::default());

    assert_eq!(subjects(&findings), [Some("checks".to_owned())]);
}

#[test]
fn a_rust_test_with_a_direct_assertion_is_not_reported() {
    let findings = findings_for(
        "#[test]\nfn checks() {\n    assert_eq!(1, 1);\n}\n",
        &lang::RUST,
        &Policy::default(),
    );

    assert_eq!(findings, []);
}

#[test]
fn a_rust_test_calling_a_same_file_helper_that_asserts_is_not_reported() {
    let source = "fn responds_with(status: u16) {\n    assert_eq!(status, 201);\n}\n\n#[test]\nfn checks() {\n    responds_with(201);\n}\n";
    let findings = findings_for(source, &lang::RUST, &Policy::default());

    assert_eq!(findings, []);
}

#[test]
fn a_rust_test_calling_a_helper_that_does_not_assert_is_reported() {
    let source =
        "fn helper() {\n    println!(\"hi\");\n}\n\n#[test]\nfn checks() {\n    helper();\n}\n";
    let findings = findings_for(source, &lang::RUST, &Policy::default());

    assert_eq!(subjects(&findings), [Some("checks".to_owned())]);
}

#[test]
fn a_wrong_kind_candidate_sharing_the_helpers_name_does_not_satisfy_the_lookup() {
    let source = "mod helper {\n    #[test]\n    fn inner() {\n        assert!(true);\n    }\n}\n\nfn helper() {}\n\n#[test]\nfn checks() {\n    helper();\n}\n";
    let findings = findings_for(source, &lang::RUST, &Policy::default());

    assert_eq!(subjects(&findings), [Some("checks".to_owned())]);
}

#[test]
fn same_named_helpers_in_different_modules_do_not_collide() {
    let source = "mod a {\n    fn setup() {}\n\n    #[test]\n    fn checks() {\n        setup();\n    }\n}\n\nmod b {\n    fn setup() {\n        assert!(true);\n    }\n\n    #[test]\n    fn also_checks() {\n        setup();\n    }\n}\n";
    let findings = findings_for(source, &lang::RUST, &Policy::default());

    assert_eq!(subjects(&findings), [Some("checks".to_owned())]);
}

#[test]
fn a_rust_test_marked_should_panic_needs_no_body_assertion() {
    let findings = findings_for(
        "#[test]\n#[should_panic]\nfn checks() {\n    do_it();\n}\n",
        &lang::RUST,
        &Policy::default(),
    );

    assert_eq!(findings, []);
}

#[test]
fn an_assertion_elsewhere_in_the_file_does_not_satisfy_an_unrelated_test() {
    let source = "#[test]\nfn asserts() {\n    assert!(true);\n}\n\n#[test]\nfn empty() {}\n";
    let findings = findings_for(source, &lang::RUST, &Policy::default());

    assert_eq!(subjects(&findings), [Some("empty".to_owned())]);
}

#[test]
fn a_non_test_rust_function_is_never_reported() {
    let findings = findings_for("fn helper() {}\n", &lang::RUST, &Policy::default());

    assert_eq!(findings, []);
}

#[test]
fn a_rule_switched_off_reports_nothing_however_many_tests_are_empty() {
    let mut policy = Policy::default();
    policy.set(
        Rule::Assertion,
        RuleConfig {
            limit: 0,
            severity: Severity::Off,
        },
    );

    let findings = findings_for("#[test]\nfn checks() {}\n", &lang::RUST, &policy);

    assert_eq!(findings, []);
}

#[test]
fn a_kotlin_test_calling_a_same_file_helper_that_asserts_is_not_reported() {
    let source = "fun helperAsserts() {\n    assertTrue(true)\n}\n\nclass LiveTest {\n    @Test\n    fun checks() {\n        helperAsserts()\n    }\n}\n";
    let findings = findings_for(source, &lang::KOTLIN, &Policy::default());

    assert_eq!(findings, []);
}

#[test]
fn an_anonymous_typescript_test_reports_no_subject() {
    let source = "it('does something', () => {\n});\n";
    let findings = findings_for(source, &lang::TYPESCRIPT, &Policy::default());

    assert_eq!(subjects(&findings), [None]);
}

#[test]
fn a_typescript_test_with_expect_is_not_reported() {
    let source = "it('does something', () => {\n    expect(1).toBe(1);\n});\n";
    let findings = findings_for(source, &lang::TYPESCRIPT, &Policy::default());

    assert_eq!(findings, []);
}

#[test]
fn a_typescript_test_calling_a_helper_that_does_not_assert_is_reported() {
    let source = "function helper(): void {\n    console.log('hi');\n}\n\nit('does something', () => {\n    helper();\n});\n";
    let findings = findings_for(source, &lang::TYPESCRIPT, &Policy::default());

    assert_eq!(subjects(&findings), [None]);
}
