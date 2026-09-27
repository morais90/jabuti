use jabuti_core::catalog::UnitKind;
use jabuti_core::code::metrics::{self, CognitiveIndex, DecisionIndex, LineIndex, Loc};
use jabuti_core::code::units::{self, Unit};
use jabuti_core::{lang, syntax};
use rstest::rstest;

use super::common::{self, find_unit, kinds, outline};

fn parse_typescript(source: &str) -> syntax::Parsed<'_> {
    syntax::parse(source, &lang::TYPESCRIPT).expect("the fixture parses cleanly")
}

fn typescript_units() -> Unit {
    units::units(&parse_typescript(&common::read_fixture(
        "typescript/units.ts",
    )))
}

#[derive(Debug, PartialEq, Eq)]
struct FunctionMetrics {
    cyclomatic: u32,
    cognitive: u32,
    parameters: u32,
}

#[test]
fn a_typescript_file_is_detected_by_its_extension() {
    let spec = lang::detect(std::path::Path::new("src/catalog.ts")).expect("typescript is known");

    assert_eq!(spec.id, lang::LanguageId::TypeScript);
    assert_eq!(spec.extensions, ["ts"]);
}

#[test]
fn the_unit_tree_mirrors_the_structure_of_the_source() {
    insta::assert_snapshot!(outline(&typescript_units()));
}

#[test]
fn nested_functions_and_arrows_belong_to_their_enclosing_function() {
    let file = typescript_units();

    assert_eq!(
        kinds(&find_unit(&file, "display").children),
        [UnitKind::Function, UnitKind::Function]
    );
    assert_eq!(
        kinds(&find_unit(&file, "outer").children),
        [UnitKind::Function]
    );
}

#[test]
fn comments_come_from_the_typescript_grammar() {
    let source = "// a note\nfunction small(): void {}\n\n/* a block */\n";
    let parsed = parse_typescript(source);
    let index = LineIndex::new(source, &metrics::comment_ranges(&parsed));

    assert_eq!(
        index.loc(units::units(&parsed).span),
        Loc {
            total: 4,
            code: 1,
            comment: 2,
            blank: 1,
        }
    );
}

#[rstest]
#[case("straight", FunctionMetrics { cyclomatic: 1, cognitive: 0, parameters: 1 })]
#[case("branches", FunctionMetrics { cyclomatic: 3, cognitive: 3, parameters: 1 })]
#[case("nested", FunctionMetrics { cyclomatic: 3, cognitive: 3, parameters: 2 })]
#[case("logical", FunctionMetrics { cyclomatic: 4, cognitive: 3, parameters: 3 })]
#[case("loops", FunctionMetrics { cyclomatic: 6, cognitive: 6, parameters: 1 })]
#[case("choose", FunctionMetrics { cyclomatic: 2, cognitive: 1, parameters: 3 })]
#[case("dispatch", FunctionMetrics { cyclomatic: 3, cognitive: 1, parameters: 1 })]
#[case("coalesce", FunctionMetrics { cyclomatic: 2, cognitive: 1, parameters: 1 })]
#[case("compare", FunctionMetrics { cyclomatic: 1, cognitive: 0, parameters: 1 })]
fn typescript_constructs_have_hand_derived_metrics(
    #[case] name: &str,
    #[case] expected: FunctionMetrics,
) {
    let source = common::read_fixture("typescript/metrics.ts");
    let parsed = parse_typescript(&source);
    let file = units::units(&parsed);
    let decisions = DecisionIndex::new(&metrics::decisions(&parsed));
    let cognitive = CognitiveIndex::new(&metrics::increments(&parsed));
    let unit = find_unit(&file, name);

    assert_eq!(
        FunctionMetrics {
            cyclomatic: decisions.cyclomatic(unit),
            cognitive: cognitive.cognitive(unit),
            parameters: unit.parameters,
        },
        expected
    );
}

#[test]
fn named_arrows_reset_nesting_but_callbacks_inherit_it() {
    let source = "const named = (first: boolean, second: boolean): boolean => {\n    if (first) {\n        if (second) { return true; }\n    }\n    return false;\n};\n\nfunction host(values: number[]): number[] {\n    return values.map((value) => {\n        if (value > 0) { return value; }\n        return 0;\n    });\n}\n";
    let parsed = parse_typescript(source);
    let file = units::units(&parsed);
    let cognitive = CognitiveIndex::new(&metrics::increments(&parsed));

    assert_eq!(cognitive.cognitive(find_unit(&file, "named")), 3);
    assert_eq!(cognitive.cognitive(find_unit(&file, "host")), 2);
}

#[test]
fn an_unparenthesized_arrow_declares_one_parameter() {
    let source = "const identity = value => {\n    const copy = value;\n    return copy;\n};\n";
    let file = units::units(&parse_typescript(source));

    assert_eq!(find_unit(&file, "identity").parameters, 1);
}

#[test]
fn a_class_field_arrow_resets_cognitive_nesting() {
    let source = "class Classifier {\n    classify = (first: boolean, second: boolean): boolean => {\n        if (first) {\n            if (second) { return true; }\n        }\n        return false;\n    };\n}\n";
    let parsed = parse_typescript(source);
    let file = units::units(&parsed);
    let cognitive = CognitiveIndex::new(&metrics::increments(&parsed));

    assert_eq!(cognitive.cognitive(find_unit(&file, "classify")), 3);
}

#[test]
fn a_deep_expression_has_zero_cognitive_complexity_without_consuming_the_thread_stack() {
    let depth = 4_096;
    let source = format!(
        "function deep(value: number): number {{ return {}value{}; }}",
        "(".repeat(depth),
        ")".repeat(depth)
    );
    let parsed = parse_typescript(&source);

    assert_eq!(metrics::increments(&parsed), []);
}
