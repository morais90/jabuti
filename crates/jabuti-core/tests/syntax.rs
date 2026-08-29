mod common;

use jabuti_core::model::UnitKind;
use jabuti_core::syntax::SyntaxError;

use common::{find_unit, kinds, outline, parse_fixture, parse_fixture_result};

#[test]
fn the_unit_tree_mirrors_the_structure_of_the_source() {
    let file = parse_fixture("rust/units.rs");

    insta::assert_snapshot!(outline(&file));
}

#[test]
fn a_closure_inside_a_method_nests_under_that_method() {
    let file = parse_fixture("rust/units.rs");

    let doubled = find_unit(&file, "doubled");

    assert_eq!(kinds(&doubled.children), [UnitKind::Closure]);
}

#[test]
fn a_function_declared_inside_another_function_nests_under_it() {
    let file = parse_fixture("rust/units.rs");

    let outer = find_unit(&file, "outer");

    assert_eq!(kinds(&outer.children), [UnitKind::Function]);
    assert_eq!(outer.children[0].name.as_deref(), Some("inner"));
}

#[test]
fn source_that_does_not_parse_is_rejected_rather_than_measured() {
    let parsed = parse_fixture_result("rust/malformed.rs");

    assert!(matches!(parsed, Err(SyntaxError::Malformed)));
}
