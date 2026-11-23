mod common;

use common::{jabuti, project};
use predicates::str::contains;

#[test]
fn an_empty_rust_test_names_the_test_that_asserts_nothing() {
    let source = "#[test]\nfn checks() {}\n";
    let directory = project(&[("src/live.rs", source)]);

    jabuti(&directory).assert().code(1).stdout(contains(
        "src/live.rs:2  assertion  checks  the test runs and asserts nothing",
    ));
}

#[test]
fn a_rust_test_with_a_direct_assertion_is_left_alone() {
    let source = "#[test]\nfn checks() {\n    assert_eq!(1, 1);\n}\n";
    let directory = project(&[("src/live.rs", source)]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_rust_test_reaching_a_same_file_helper_that_asserts_is_left_alone() {
    let source = "fn responds_with(status: u16) {\n    assert_eq!(status, 201);\n}\n\n#[test]\nfn checks() {\n    responds_with(201);\n}\n";
    let directory = project(&[("src/live.rs", source)]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_rust_test_marked_should_panic_needs_no_body_assertion() {
    let source = "#[test]\n#[should_panic]\nfn checks() {\n    do_it();\n}\n";
    let directory = project(&[("src/live.rs", source)]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_kotlin_test_using_assert_that_is_left_alone() {
    let source = "class LiveTest {\n    @Test\n    fun checks() {\n        assertThat(1).isEqualTo(1)\n    }\n}\n";
    let directory = project(&[("src/Live.kt", source)]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_kotlin_test_with_no_assertion_names_the_test() {
    let source =
        "class LiveTest {\n    @Test\n    fun checks() {\n        println(\"hi\")\n    }\n}\n";
    let directory = project(&[("src/Live.kt", source)]);

    jabuti(&directory).assert().code(1).stdout(contains(
        "src/Live.kt:2  assertion  checks  the test runs and asserts nothing",
    ));
}

#[test]
fn a_typescript_it_callback_with_no_assertion_is_reported_with_no_subject() {
    let source = "it('does something', () => {\n});\n";
    let directory = project(&[("src/live.ts", source)]);

    jabuti(&directory).assert().code(1).stdout(contains(
        "src/live.ts:1  assertion  the test runs and asserts nothing",
    ));
}

#[test]
fn a_typescript_it_callback_with_expect_is_left_alone() {
    let source = "it('does something', () => {\n    expect(1).toBe(1);\n});\n";
    let directory = project(&[("src/live.ts", source)]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_typescript_describe_callback_is_never_reported_on_its_own() {
    let source = "describe('group', () => {\n    it('does something', () => {\n        expect(1).toBe(1);\n    });\n});\n";
    let directory = project(&[("src/live.ts", source)]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_configured_typescript_helper_extends_the_built_in_vocabulary() {
    let settings = "[languages.typescript.concepts]\nassertion = [\"@mycorp/testing.checkThat\"]\n";
    let source = "import { checkThat } from \"@mycorp/testing\";\n\nit('does something', () => {\n    checkThat(1);\n});\n";
    let directory = project(&[("jabuti.toml", settings), ("src/live.ts", source)]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}
