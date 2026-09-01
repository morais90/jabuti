mod common;

use common::{jabuti, project};
use predicates::str::contains;

const MASKED: &str = "fn live() {\n    let value = read().unwrap();\n}\n";

fn only_masking() -> String {
    "[rules]\nhotspot = { severity = \"off\" }\nduplicate-block = { severity = \"off\" }\n"
        .to_owned()
}

#[test]
fn a_masked_failure_names_the_construct_and_what_it_costs() {
    let directory = project(&[("jabuti.toml", &only_masking()), ("src/live.rs", MASKED)]);

    jabuti(&directory).assert().success().stdout(contains(
        "src/live.rs:2  warning  error-masking  unwrap  the failure becomes a panic",
    ));
}

#[test]
fn the_same_construct_in_a_test_file_is_left_alone() {
    let directory = project(&[
        ("jabuti.toml", &only_masking()),
        ("tests/behaviour.rs", MASKED),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn masking_can_be_promoted_to_a_failing_gate() {
    let directory = project(&[
        (
            "jabuti.toml",
            "[rules]\nerror-masking = { severity = \"error\" }\n",
        ),
        ("src/live.rs", MASKED),
    ]);

    jabuti(&directory)
        .assert()
        .code(1)
        .stdout(contains("error  error-masking"));
}

#[test]
fn masking_can_be_switched_off_for_one_language() {
    let directory = project(&[
        (
            "jabuti.toml",
            "[languages.rust.rules]\nerror-masking = { severity = \"off\" }\n",
        ),
        ("src/live.rs", MASKED),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_test_directory_given_as_the_path_argument_is_still_a_test_directory() {
    let directory = project(&[
        ("jabuti.toml", &only_masking()),
        ("tests/behaviour.rs", MASKED),
    ]);

    jabuti(&directory)
        .arg("tests")
        .assert()
        .success()
        .stdout(contains("No findings"));
}
