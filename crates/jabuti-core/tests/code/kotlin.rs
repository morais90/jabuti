use jabuti_core::code::metrics::{self, CognitiveIndex, DecisionIndex, LineIndex, Loc};
use jabuti_core::code::units::{self, Unit};
use jabuti_core::model::UnitKind;
use jabuti_core::{lang, syntax};
use rstest::rstest;

use super::common::{self, find_unit, kinds, outline};

fn parse_kotlin(source: &str) -> syntax::Parsed<'_> {
    syntax::parse(source, &lang::KOTLIN).expect("the fixture parses cleanly")
}

fn kotlin_units() -> Unit {
    units::units(&parse_kotlin(&common::read_fixture("kotlin/units.kt")))
}

#[test]
fn a_kotlin_file_is_detected_by_its_extension() {
    let spec = lang::detect(std::path::Path::new("src/Main.kt")).expect("kotlin is known");

    assert_eq!(spec.id, lang::LanguageId::Kotlin);
}

#[test]
fn the_unit_tree_mirrors_the_structure_of_the_source() {
    insta::assert_snapshot!(outline(&kotlin_units()));
}

#[test]
fn a_lambda_inside_a_method_nests_under_that_method() {
    let file = kotlin_units();

    assert_eq!(
        kinds(&find_unit(&file, "doubled").children),
        [UnitKind::Closure]
    );
}

#[test]
fn a_function_declared_inside_another_function_nests_under_it() {
    let file = kotlin_units();

    assert_eq!(
        kinds(&find_unit(&file, "outer").children),
        [UnitKind::Function]
    );
}

#[test]
fn comments_come_from_the_grammar_here_too() {
    let source = "// a note\nfun small() {}\n\n/* a block */\n";
    let parsed = parse_kotlin(source);
    let index = LineIndex::new(source, &metrics::comment_ranges(&parsed));

    assert_eq!(
        index.loc(units::units(&parsed).span),
        Loc {
            total: 4,
            code: 1,
            comment: 2,
            blank: 1
        }
    );
}

#[rstest]
#[case("fun a(): Int = 1", 1)]
#[case("fun a(v: Int): Int { if (v > 0) { return 1 } else { return 0 } }", 2)]
#[case(
    "fun a(v: Int): Int { if (v > 0) { return 1 } else if (v < 0) { return 2 } else { return 3 } }",
    3
)]
#[case(
    "fun a(x: Boolean, y: Boolean, z: Boolean): Boolean = if (x && y || z) true else false",
    4
)]
#[case(
    "fun a(v: Int): Int { for (i in 0..v) {}; while (v > 0) {}; do {} while (v < 0); return v }",
    4
)]
#[case(
    "fun a(v: Int): String = when (v) { 0 -> \"z\"; 1 -> \"o\"; else -> \"m\" }",
    3
)]
#[case(
    "fun a(): Int { try { return 1 } catch (e: Exception) { return 0 } }",
    2
)]
#[case("fun a(v: Int?): Int = v ?: 0", 2)]
fn cyclomatic_complexity_counts_kotlin_decisions(#[case] source: &str, #[case] expected: u32) {
    let parsed = parse_kotlin(source);
    let index = DecisionIndex::new(&metrics::decisions(&parsed));
    let file = units::units(&parsed);

    assert_eq!(index.cyclomatic(find_unit(&file, "a")), expected);
}

#[rstest]
#[case("fun a(): Int = 1", 0)]
#[case("fun a(v: Int): Int { if (v > 0) { return 1 } else { return 0 } }", 2)]
#[case(
    "fun a(v: Int): Int { if (v > 0) { return 1 } else if (v < 0) { return 2 } else { return 3 } }",
    3
)]
#[case(
    "fun a(x: Boolean, y: Boolean, z: Boolean): Boolean { if (x) { if (y) { if (z) { return true } } }; return false }",
    6
)]
#[case(
    "fun a(x: Boolean, y: Boolean, z: Boolean): Boolean { if (x) {}; if (y) {}; if (z) {}; return false }",
    3
)]
#[case(
    "fun a(v: Int): String = when (v) { 0 -> \"z\"; 1 -> \"o\"; 2 -> \"t\"; else -> \"m\" }",
    1
)]
#[case(
    "fun a(x: Boolean, y: Boolean, z: Boolean): Boolean = if (x && y && z) true else false",
    3
)]
#[case(
    "fun a(x: Boolean, y: Boolean, z: Boolean): Boolean = if (x && y || z) true else false",
    4
)]
fn cognitive_complexity_charges_kotlin_nesting(#[case] source: &str, #[case] expected: u32) {
    let parsed = parse_kotlin(source);
    let index = CognitiveIndex::new(&metrics::increments(&parsed));
    let file = units::units(&parsed);

    assert_eq!(index.cognitive(find_unit(&file, "a")), expected);
}

#[rstest]
#[case("fun a() {}", 0)]
#[case("fun a(first: Int, second: Int) {}", 2)]
#[case("class H {\n    fun a(first: Int) {}\n}\n", 1)]
fn parameters_are_counted_the_same_way(#[case] source: &str, #[case] expected: u32) {
    let file = units::units(&parse_kotlin(source));

    assert_eq!(find_unit(&file, "a").parameters, expected);
}
