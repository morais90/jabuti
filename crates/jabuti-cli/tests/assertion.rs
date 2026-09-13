mod common;

use common::{jabuti, project};
use predicates::str::contains;

fn only_assertion() -> String {
    "[rules]\nhotspot = { severity = \"off\" }\nduplicate-block = { severity = \"off\" }\n"
        .to_owned()
}

#[test]
fn an_empty_rust_test_names_the_test_that_asserts_nothing() {
    let source = "#[test]\nfn checks() {}\n";
    let directory = project(&[("jabuti.toml", &only_assertion()), ("src/live.rs", source)]);

    jabuti(&directory).assert().success().stdout(contains(
        "src/live.rs:2  warning  assertion  checks  the test runs and asserts nothing",
    ));
}

#[test]
fn a_rust_test_with_a_direct_assertion_is_left_alone() {
    let source = "#[test]\nfn checks() {\n    assert_eq!(1, 1);\n}\n";
    let directory = project(&[("jabuti.toml", &only_assertion()), ("src/live.rs", source)]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_rust_test_reaching_a_same_file_helper_that_asserts_is_left_alone() {
    let source = "fn responds_with(status: u16) {\n    assert_eq!(status, 201);\n}\n\n#[test]\nfn checks() {\n    responds_with(201);\n}\n";
    let directory = project(&[("jabuti.toml", &only_assertion()), ("src/live.rs", source)]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_rust_test_marked_should_panic_needs_no_body_assertion() {
    let source = "#[test]\n#[should_panic]\nfn checks() {\n    do_it();\n}\n";
    let directory = project(&[("jabuti.toml", &only_assertion()), ("src/live.rs", source)]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_kotlin_test_using_assert_that_is_left_alone() {
    let source = "class LiveTest {\n    @Test\n    fun checks() {\n        assertThat(1).isEqualTo(1)\n    }\n}\n";
    let directory = project(&[("jabuti.toml", &only_assertion()), ("src/Live.kt", source)]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_kotlin_test_with_no_assertion_names_the_test() {
    let source =
        "class LiveTest {\n    @Test\n    fun checks() {\n        println(\"hi\")\n    }\n}\n";
    let directory = project(&[("jabuti.toml", &only_assertion()), ("src/Live.kt", source)]);

    jabuti(&directory).assert().success().stdout(contains(
        "src/Live.kt:2  warning  assertion  checks  the test runs and asserts nothing",
    ));
}

#[test]
fn a_typescript_it_callback_with_no_assertion_is_reported_with_no_subject() {
    let source = "it('does something', () => {\n});\n";
    let directory = project(&[("jabuti.toml", &only_assertion()), ("src/live.ts", source)]);

    jabuti(&directory).assert().success().stdout(contains(
        "src/live.ts:1  warning  assertion  the test runs and asserts nothing",
    ));
}

#[test]
fn a_typescript_it_callback_with_expect_is_left_alone() {
    let source = "it('does something', () => {\n    expect(1).toBe(1);\n});\n";
    let directory = project(&[("jabuti.toml", &only_assertion()), ("src/live.ts", source)]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_typescript_describe_callback_is_never_reported_on_its_own() {
    let source = "describe('group', () => {\n    it('does something', () => {\n        expect(1).toBe(1);\n    });\n});\n";
    let directory = project(&[("jabuti.toml", &only_assertion()), ("src/live.ts", source)]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn assertion_can_be_promoted_to_a_failing_gate() {
    let source = "#[test]\nfn checks() {}\n";
    let directory = project(&[
        (
            "jabuti.toml",
            "[rules]\nassertion = { severity = \"error\" }\n",
        ),
        ("src/live.rs", source),
    ]);

    jabuti(&directory)
        .assert()
        .code(1)
        .stdout(contains("error  assertion"));
}

#[test]
fn assertion_can_be_switched_off_for_one_language() {
    let source = "#[test]\nfn checks() {}\n";
    let directory = project(&[
        (
            "jabuti.toml",
            "[languages.rust.rules]\nassertion = { severity = \"off\" }\n",
        ),
        ("src/live.rs", source),
    ]);

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
