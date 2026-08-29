mod common;

use std::ops::Range;

use common::{find_unit, line_index_of, parse_fixture, read_fixture, units_of};
use jabuti_core::metrics::{DecisionIndex, LineIndex, Loc};
use jabuti_core::model::{Decision, DecisionEffect, Span, Unit, UnitKind};
use rstest::rstest;

#[test]
fn every_line_of_a_file_is_counted_as_code_comment_or_blank() {
    let index = line_index_of("rust/loc.rs");

    let file = units_of("rust/loc.rs");

    assert_eq!(
        index.loc(file.span),
        Loc {
            total: 10,
            code: 4,
            comment: 4,
            blank: 2
        }
    );
}

#[test]
fn a_unit_is_counted_over_its_own_span_only() {
    let source = read_fixture("rust/loc.rs");
    let parsed = parse_fixture(&source);
    let index = LineIndex::new(&source, &parsed.comment_ranges());

    let file = parsed.units();
    let measured = find_unit(&file, "measured");

    assert_eq!(
        index.loc(measured.span),
        Loc {
            total: 6,
            code: 4,
            comment: 1,
            blank: 1
        }
    );
}

#[test]
fn a_line_holding_both_code_and_a_comment_counts_as_code() {
    let source = "fn noted() {\n    let value = 1; // note\n}\n";
    let parsed = parse_fixture(source);
    let index = LineIndex::new(source, &parsed.comment_ranges());

    assert_eq!(
        index.loc(parsed.units().span),
        Loc {
            total: 3,
            code: 3,
            comment: 0,
            blank: 0
        }
    );
}

#[test]
fn blank_lines_before_the_first_token_are_counted() {
    let source = "\n\nfn measured() {}\n";
    let parsed = parse_fixture(source);
    let index = LineIndex::new(source, &parsed.comment_ranges());

    assert_eq!(
        index.loc(parsed.units().span),
        Loc {
            total: 3,
            code: 1,
            comment: 0,
            blank: 2
        }
    );
}

#[rstest]
#[case("straight_line", 1)]
#[case("single_branch", 2)]
#[case("else_if_chain", 3)]
#[case("boolean_operators", 4)]
#[case("every_loop", 4)]
#[case("one_arm_match", 1)]
#[case("three_arm_match", 3)]
#[case("guarded_match", 4)]
#[case("holds_a_closure", 2)]
#[case("holds_a_nested_function", 1)]
#[case("inner", 2)]
fn cyclomatic_complexity_matches_the_derivation_in_the_fixture(
    #[case] unit_name: &str,
    #[case] expected: u32,
) {
    let source = read_fixture("rust/cyclomatic.rs");
    let parsed = parse_fixture(&source);
    let index = DecisionIndex::new(&parsed.decisions());

    let file = parsed.units();

    assert_eq!(index.cyclomatic(find_unit(&file, unit_name)), expected);
}

#[test]
fn a_decision_starting_on_the_first_byte_of_a_unit_belongs_to_it() {
    let index = DecisionIndex::new(&[branch_at(10)]);

    assert_eq!(index.cyclomatic(&unit_over(10..20)), 2);
}

#[test]
fn a_decision_starting_on_the_byte_after_a_unit_falls_outside_it() {
    let index = DecisionIndex::new(&[branch_at(20)]);

    assert_eq!(index.cyclomatic(&unit_over(10..20)), 1);
}

fn branch_at(position: usize) -> Decision {
    Decision {
        position,
        effect: DecisionEffect::Branch,
    }
}

fn unit_over(bytes: Range<usize>) -> Unit {
    Unit {
        name: None,
        kind: UnitKind::Function,
        span: Span {
            start_line: 1,
            end_line: 1,
        },
        bytes,
        children: Vec::new(),
    }
}

#[test]
fn a_file_carries_no_complexity_of_its_own() {
    let source = read_fixture("rust/cyclomatic.rs");
    let parsed = parse_fixture(&source);
    let index = DecisionIndex::new(&parsed.decisions());

    assert_eq!(index.cyclomatic(&parsed.units()), 1);
}

#[test]
fn the_three_line_kinds_always_add_up_to_the_total() {
    let index = line_index_of("rust/units.rs");

    let loc = index.loc(units_of("rust/units.rs").span);

    assert_eq!(loc.total, loc.code + loc.comment + loc.blank);
}
