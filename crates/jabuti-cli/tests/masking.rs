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

#[test]
fn typescript_empty_catches_are_reported_as_error_masking() {
    let source = "export async function live(): Promise<void> {\n    const pending = read();\n    try {\n        await pending;\n    } catch {\n    }\n}\n";
    let directory = project(&[("jabuti.toml", &only_masking()), ("src/live.ts", source)]);

    jabuti(&directory).assert().success().stdout(contains(
        "src/live.ts:5  warning  error-masking  catch  the failure is caught and nothing happens",
    ));
}

#[test]
fn typescript_spec_files_are_left_out_of_error_masking() {
    let source = "export async function checks(): Promise<void> {\n    try {\n        await read();\n    } catch {\n    }\n}\n";
    let directory = project(&[
        ("jabuti.toml", &only_masking()),
        ("src/live.spec.ts", source),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_configured_typescript_api_is_resolved_through_its_import_alias() {
    let settings =
        "[languages.typescript.concepts]\nerror-discard = [\"@mycorp/errors.discard\"]\n";
    let source = "import { discard as ignore } from \"@mycorp/errors\";\nexport function live(ready: boolean): void {\n    if (ready) {\n        ignore();\n    }\n}\n";
    let directory = project(&[("jabuti.toml", settings), ("src/live.ts", source)]);

    jabuti(&directory).assert().success().stdout(contains(
        "src/live.ts:4  warning  error-masking  ignore  the failure is dropped without being read",
    ));
}

#[test]
fn an_unknown_concept_stops_configuration_loading() {
    let settings = "[languages.typescript.concepts]\nunknown = [\"ignore\"]\n";
    let directory = project(&[("jabuti.toml", settings), ("src/live.ts", "export {};\n")]);

    jabuti(&directory)
        .assert()
        .code(2)
        .stderr(contains("unknown concept unknown"));
}

#[test]
fn a_concept_binding_without_paths_is_rejected() {
    let settings = "[languages.typescript.concepts]\nerror-discard = []\n";
    let directory = project(&[("jabuti.toml", settings), ("src/live.ts", "export {};\n")]);

    jabuti(&directory).assert().code(2).stderr(contains(
        "concept error-discard must name at least one non-empty API path",
    ));
}
