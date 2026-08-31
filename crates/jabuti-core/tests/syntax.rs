mod common;

use common::{find_unit, kinds, outline, parse_fixture, parse_outcome, units_of};
use jabuti_core::lang;
use jabuti_core::model::{Span, UnitKind};
use jabuti_core::syntax::{self, SyntaxError};

#[test]
fn the_unit_tree_mirrors_the_structure_of_the_source() {
    let file = units_of("rust/units.rs");

    insta::assert_snapshot!(outline(&file));
}

#[test]
fn a_closure_inside_a_method_nests_under_that_method() {
    let file = units_of("rust/units.rs");

    let doubled = find_unit(&file, "doubled");

    assert_eq!(kinds(&doubled.children), [UnitKind::Closure]);
}

#[test]
fn a_function_declared_inside_another_function_nests_under_it() {
    let file = units_of("rust/units.rs");

    let outer = find_unit(&file, "outer");

    assert_eq!(kinds(&outer.children), [UnitKind::Function]);
    assert_eq!(outer.children[0].name.as_deref(), Some("inner"));
}

#[test]
fn a_file_that_opens_with_blank_lines_still_starts_at_line_one() {
    let file = parse_fixture("\n\nfn measured() {}\n").units();

    assert_eq!(
        file.span,
        Span {
            start_line: 1,
            end_line: 3
        }
    );
}

#[test]
fn a_file_holding_only_whitespace_spans_the_lines_it_has() {
    let file = parse_fixture("   \n   \n").units();

    assert_eq!(
        file.span,
        Span {
            start_line: 1,
            end_line: 2
        }
    );
}

#[test]
fn every_registered_language_compiles_its_queries() {
    for spec in lang::ALL {
        assert!(syntax::parse("", spec).is_ok(), "{:?}", spec.id);
    }
}

#[test]
fn source_that_does_not_parse_is_rejected_and_says_where() {
    let parsed = parse_outcome("rust/malformed.rs");

    assert!(
        matches!(parsed, Err(SyntaxError::Malformed { line: 1 })),
        "{parsed:?}"
    );
}
