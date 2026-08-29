mod common;

use jabuti_core::metrics::{LineIndex, Loc};

use common::{find_unit, line_index_of, parse_fixture, read_fixture, units_of};

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
fn the_three_line_kinds_always_add_up_to_the_total() {
    let index = line_index_of("rust/units.rs");

    let loc = index.loc(units_of("rust/units.rs").span);

    assert_eq!(loc.total, loc.code + loc.comment + loc.blank);
}
