mod common;

use common::parse_fixture;
use jabuti_core::model::{Detail, MaskingKind, Rule, Severity};
use jabuti_core::policy::{Policy, RuleConfig};
use jabuti_core::{lang, masking, syntax};

fn masked(source: &str) -> Vec<(MaskingKind, String)> {
    parse_fixture(source)
        .maskings()
        .into_iter()
        .map(|masking| (masking.kind, masking.construct))
        .collect()
}

fn masked_kotlin(source: &str) -> Vec<(MaskingKind, String)> {
    syntax::parse(source, &lang::KOTLIN)
        .expect("fixture parses cleanly")
        .maskings()
        .into_iter()
        .map(|masking| (masking.kind, masking.construct))
        .collect()
}

fn panics(construct: &str) -> (MaskingKind, String) {
    (MaskingKind::Panic, construct.to_owned())
}

fn discards(construct: &str) -> (MaskingKind, String) {
    (MaskingKind::Discard, construct.to_owned())
}

#[test]
fn turning_a_failure_into_a_panic_is_masking_it() {
    let source = "fn f() {\n    let a = g().unwrap();\n    let b = g().expect(\"gone\");\n}\n";

    assert_eq!(masked(source), [panics("unwrap"), panics("expect")]);
}

#[test]
fn dropping_a_failure_without_reading_it_is_masking_it() {
    let source = "fn f() {\n    let _ = g();\n    let c = g().ok();\n}\n";

    assert_eq!(masked(source), [discards("_"), discards("ok")]);
}

#[test]
fn a_binding_that_merely_starts_with_an_underscore_is_not_a_discard() {
    let source = "fn f() {\n    let _unused = g();\n}\n";

    assert_eq!(masked(source), []);
}

#[test]
fn an_empty_error_arm_swallows_the_failure_but_a_handled_one_does_not() {
    let swallowed = "fn f() {\n    match g() { Ok(v) => v, Err(_) => {} };\n}\n";
    let handled = "fn f() {\n    match g() { Ok(v) => v, Err(e) => handle(e) };\n}\n";

    assert_eq!(
        masked(swallowed),
        [(MaskingKind::Swallow, "Err".to_owned())]
    );
    assert_eq!(masked(handled), []);
}

#[test]
fn a_method_whose_name_merely_contains_unwrap_is_left_alone() {
    let source = "fn f() {\n    let a = h.unwrapped();\n    let b = h.expected();\n}\n";

    assert_eq!(masked(source), []);
}

#[test]
fn a_failure_inside_a_test_function_is_the_assertion_not_a_mask() {
    let source = "#[test]\nfn checks() {\n    let a = g().unwrap();\n}\n";

    assert_eq!(masked(source), []);
}

#[test]
fn a_failure_inside_a_test_module_is_left_alone_too() {
    let source =
        "#[cfg(test)]\nmod tests {\n    fn helper() {\n        let a = g().unwrap();\n    }\n}\n";

    assert_eq!(masked(source), []);
}

#[test]
fn production_code_beside_a_test_module_is_still_read() {
    let source = "fn live() {\n    let a = g().unwrap();\n}\n\n#[cfg(test)]\nmod tests {\n    fn helper() {\n        let b = g().unwrap();\n    }\n}\n";

    assert_eq!(masked(source), [panics("unwrap")]);
}

#[test]
fn kotlin_masks_a_failure_with_the_assertion_operator() {
    let source = "fun f() {\n    val a = g()!!\n}\n";

    assert_eq!(masked_kotlin(source), [panics("!!")]);
}

#[test]
fn kotlin_reports_an_empty_catch_but_not_one_that_recovers() {
    let swallowed = "fun f() {\n    try {\n        g()\n    } catch (e: Exception) {\n    }\n}\n";
    let handled = "fun f() {\n    try {\n        g()\n    } catch (e: Exception) {\n        recover(e)\n    }\n}\n";

    assert_eq!(
        masked_kotlin(swallowed),
        [(MaskingKind::Swallow, "catch".to_owned())]
    );
    assert_eq!(masked_kotlin(handled), []);
}

#[test]
fn kotlin_reports_a_result_turned_into_nothing() {
    let source = "fun f() {\n    val b = runCatching { g() }.getOrNull()\n}\n";

    assert_eq!(masked_kotlin(source), [discards("getOrNull")]);
}

#[test]
fn kotlin_leaves_an_annotated_test_alone() {
    let source = "class T {\n    @Test\n    fun checks() {\n        val a = g()!!\n    }\n}\n";

    assert_eq!(masked_kotlin(source), []);
}

#[test]
fn a_finding_names_the_construct_and_what_it_costs() {
    let maskings = parse_fixture("fn f() {\n    let a = g().unwrap();\n}\n").maskings();
    let findings = masking::findings("src/a.rs", lang::RUST.id, &maskings, &Policy::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].subject.as_deref(), Some("unwrap"));
    assert_eq!(
        findings[0].detail,
        Detail::Message {
            message: "the failure becomes a panic".to_owned()
        }
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

    let maskings = parse_fixture("fn f() {\n    let a = g().unwrap();\n}\n").maskings();

    assert_eq!(
        masking::findings("src/a.rs", lang::RUST.id, &maskings, &policy),
        []
    );
}
