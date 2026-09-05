use std::ops::Range;

use jabuti_core::code::metrics::{
    self, CognitiveIndex, Decision, DecisionEffect, DecisionIndex, Increment, LineIndex, Loc,
};
use jabuti_core::code::units::{self, Unit};
use jabuti_core::model::{Span, UnitKind};
use rstest::rstest;

use super::common::{find_unit, kinds, line_index_of, parse_fixture, read_fixture, units_of};

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
    let index = LineIndex::new(&source, &metrics::comment_ranges(&parsed));

    let file = units::units(&parsed);
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
    let index = LineIndex::new(source, &metrics::comment_ranges(&parsed));

    assert_eq!(
        index.loc(units::units(&parsed).span),
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
    let index = LineIndex::new(source, &metrics::comment_ranges(&parsed));

    assert_eq!(
        index.loc(units::units(&parsed).span),
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
    let index = DecisionIndex::new(&metrics::decisions(&parsed));

    let file = units::units(&parsed);

    assert_eq!(index.cyclomatic(find_unit(&file, unit_name)), expected);
}

#[rstest]
#[case("straight_line", 0)]
#[case("single_if", 2)]
#[case("else_if_chain", 3)]
#[case("sequential_ifs", 3)]
#[case("nested_ifs", 6)]
#[case("nested_flow", 6)]
#[case("wide_match", 1)]
#[case("one_operator_run", 3)]
#[case("mixed_operator_runs", 4)]
#[case("holds_a_closure", 3)]
#[case("holds_a_nested_function", 0)]
#[case("inner", 2)]
#[case("conditional_inside_an_else_body", 5)]
fn cognitive_complexity_matches_the_derivation_in_the_fixture(
    #[case] unit_name: &str,
    #[case] expected: u32,
) {
    let source = read_fixture("rust/cognitive.rs");
    let parsed = parse_fixture(&source);
    let index = CognitiveIndex::new(&metrics::increments(&parsed));

    let file = units::units(&parsed);

    assert_eq!(index.cognitive(find_unit(&file, unit_name)), expected);
}

#[rstest]
#[case("takes_nothing", 0)]
#[case("takes_two", 2)]
#[case("method_ignores_self", 2)]
#[case("method_takes_only_self", 0)]
#[case("takes_a_closure", 0)]
#[case("takes_annotated_parameters", 2)]
fn a_unit_reports_the_parameters_it_declares(#[case] unit_name: &str, #[case] expected: u32) {
    let file = units_of("rust/parameters.rs");

    assert_eq!(find_unit(&file, unit_name).parameters, expected);
}

#[test]
fn a_closure_reports_its_own_parameters() {
    let file = units_of("rust/parameters.rs");
    let holder = find_unit(&file, "takes_a_closure");

    assert_eq!(kinds(&holder.children), [UnitKind::Closure]);
    assert_eq!(holder.children[0].parameters, 2);
}

#[test]
fn nesting_is_what_separates_cognitive_complexity_from_cyclomatic() {
    let source = read_fixture("rust/cognitive.rs");
    let parsed = parse_fixture(&source);
    let cognitive = CognitiveIndex::new(&metrics::increments(&parsed));
    let decisions = DecisionIndex::new(&metrics::decisions(&parsed));

    let file = units::units(&parsed);
    let flat = find_unit(&file, "sequential_ifs");
    let deep = find_unit(&file, "nested_ifs");

    assert_eq!(decisions.cyclomatic(flat), decisions.cyclomatic(deep));
    assert!(cognitive.cognitive(deep) > cognitive.cognitive(flat));
}

#[test]
fn a_files_total_is_the_sum_of_every_function_it_holds() {
    let source = read_fixture("rust/cognitive.rs");
    let parsed = parse_fixture(&source);
    let index = CognitiveIndex::new(&metrics::increments(&parsed));

    let file = units::units(&parsed);
    let summed: u32 = functions(&file)
        .iter()
        .map(|unit| index.cognitive(unit))
        .sum();

    assert_eq!(index.total(&file), summed);
    assert!(summed > 0, "the fixture has complexity to sum");
}

fn functions(unit: &Unit) -> Vec<&Unit> {
    let mut found = Vec::new();
    if unit.kind == UnitKind::Function {
        found.push(unit);
    }
    for child in &unit.children {
        found.extend(functions(child));
    }
    found
}

#[test]
fn a_wide_match_costs_one_however_many_arms_it_has() {
    let source = read_fixture("rust/cognitive.rs");
    let parsed = parse_fixture(&source);
    let file = units::units(&parsed);
    let wide = find_unit(&file, "wide_match");

    assert_eq!(
        CognitiveIndex::new(&metrics::increments(&parsed)).cognitive(wide),
        1
    );
    assert_eq!(
        DecisionIndex::new(&metrics::decisions(&parsed)).cyclomatic(wide),
        5
    );
}

#[test]
fn an_increment_on_the_first_byte_of_a_unit_belongs_to_it() {
    let index = CognitiveIndex::new(&[increment_at(10)]);

    assert_eq!(index.cognitive(&unit_over(10..20)), 1);
}

#[test]
fn an_increment_on_the_byte_after_a_unit_falls_outside_it() {
    let index = CognitiveIndex::new(&[increment_at(20)]);

    assert_eq!(index.cognitive(&unit_over(10..20)), 0);
}

fn increment_at(position: usize) -> Increment {
    Increment {
        position,
        amount: 1,
    }
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
        parameters: 0,
        children: Vec::new(),
    }
}

#[test]
fn a_file_carries_no_complexity_of_its_own() {
    let source = read_fixture("rust/cyclomatic.rs");
    let parsed = parse_fixture(&source);
    let index = DecisionIndex::new(&metrics::decisions(&parsed));

    assert_eq!(index.cyclomatic(&units::units(&parsed)), 1);
}

#[test]
fn the_three_line_kinds_always_add_up_to_the_total() {
    let index = line_index_of("rust/units.rs");

    let loc = index.loc(units_of("rust/units.rs").span);

    assert_eq!(loc.total, loc.code + loc.comment + loc.blank);
}
