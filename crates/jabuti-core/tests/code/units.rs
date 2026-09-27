use jabuti_core::catalog::UnitKind;
use jabuti_core::code::units;
use jabuti_core::model::Span;
use jabuti_core::{lang, syntax};
use rstest::rstest;

use super::common::{find_unit, kinds, outline, parse_fixture, units_of};

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
    let file = units::units(&parse_fixture("\n\nfn measured() {}\n"));

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
    let file = units::units(&parse_fixture("   \n   \n"));

    assert_eq!(
        file.span,
        Span {
            start_line: 1,
            end_line: 2
        }
    );
}

#[test]
fn a_rust_test_attribute_marks_its_unit_as_a_test() {
    let file = units::units(&parse_fixture(
        "#[test]\nfn checks() {}\n\nfn helper() {}\n",
    ));

    assert!(find_unit(&file, "checks").is_test);
    assert!(!find_unit(&file, "helper").is_test);
}

#[test]
fn a_cfg_test_only_helper_is_test_scoped_code_and_not_itself_a_test() {
    let file = units::units(&parse_fixture(
        "#[cfg(test)]\nfn helper() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn checks() {}\n}\n",
    ));

    assert!(!find_unit(&file, "helper").is_test);
    assert!(find_unit(&file, "checks").is_test);
}

#[test]
fn should_panic_is_recognised_only_beside_a_rust_test() {
    let file = units::units(&parse_fixture(
        "#[test]\n#[should_panic]\nfn panics() {}\n\n#[test]\nfn checks() {}\n",
    ));

    assert!(find_unit(&file, "panics").should_panic);
    assert!(!find_unit(&file, "checks").should_panic);
}

#[test]
fn an_allow_attribute_naming_should_panic_in_its_argument_is_not_should_panic() {
    let file = units::units(&parse_fixture(
        "#[test]\n#[allow(clippy::should_panic_without_expect)]\nfn checks() {}\n",
    ));

    assert!(!find_unit(&file, "checks").should_panic);
}

#[test]
fn an_attribute_whose_name_only_contains_test_is_not_a_test_entry() {
    let file = units::units(&parse_fixture("#[legacy_test]\nfn checks() {}\n"));

    assert!(!find_unit(&file, "checks").is_test);
}

#[test]
fn a_qualified_test_attribute_is_still_a_test_entry() {
    let file = units::units(&parse_fixture("#[tokio::test]\nasync fn checks() {}\n"));

    assert!(find_unit(&file, "checks").is_test);
}

#[test]
fn a_kotlin_test_annotation_marks_its_unit_as_a_test() {
    let source = "fun helper() {}\n\nclass LiveTest {\n    @Test\n    fun checks() {}\n}\n";
    let parsed = syntax::parse(source, &lang::KOTLIN).expect("source parses cleanly");
    let file = units::units(&parsed);

    assert!(find_unit(&file, "checks").is_test);
    assert!(!find_unit(&file, "helper").is_test);
}

#[rstest]
#[case("it")]
#[case("test")]
#[case("it.only")]
#[case("it.skip")]
#[case("test.only")]
#[case("test.skip")]
fn a_typescript_callback_passed_to_a_test_runner_is_a_test(#[case] runner: &str) {
    let source = format!("{runner}('does something', () => {{}});\n");
    let parsed = syntax::parse(&source, &lang::TYPESCRIPT).expect("source parses cleanly");
    let file = units::units(&parsed);

    assert_eq!(kinds(&file.children), [UnitKind::Closure]);
    assert!(file.children[0].is_test);
}

#[rstest]
#[case("suite.only")]
#[case("it.deepEqual")]
fn a_typescript_callback_needs_both_the_runner_and_the_right_suffix(#[case] runner: &str) {
    let source = format!("{runner}('does something', () => {{}});\n");
    let parsed = syntax::parse(&source, &lang::TYPESCRIPT).expect("source parses cleanly");
    let file = units::units(&parsed);

    assert!(!file.children[0].is_test);
}

#[test]
fn a_typescript_describe_callback_is_not_itself_a_test() {
    let source = "describe('group', () => {\n    it('nested', () => {});\n});\n";
    let parsed = syntax::parse(source, &lang::TYPESCRIPT).expect("source parses cleanly");
    let file = units::units(&parsed);

    let describe_callback = &file.children[0];
    assert!(!describe_callback.is_test);

    let nested = &describe_callback.children[0];
    assert!(nested.is_test);
}

#[test]
fn a_typescript_closure_assigned_to_a_variable_is_not_a_test() {
    let source = "const helper = () => {};\n";
    let parsed = syntax::parse(source, &lang::TYPESCRIPT).expect("source parses cleanly");
    let file = units::units(&parsed);

    assert!(!find_unit(&file, "helper").is_test);
}
