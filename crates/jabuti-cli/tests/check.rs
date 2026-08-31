mod common;

use common::{error_on_long_functions, function_of, jabuti, project};
use predicates::str::contains;

#[test]
fn a_tree_within_the_limits_reports_nothing_and_passes() {
    let directory = project(&[("src/lib.rs", "fn small() {}\n")]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout("No findings across 1 file and 1 unit.\n");
}

#[test]
fn a_finding_configured_as_an_error_fails_the_gate() {
    let directory = project(&[
        ("jabuti.toml", &error_on_long_functions(5)),
        ("src/lib.rs", &function_of("wide", 20)),
    ]);

    jabuti(&directory)
        .assert()
        .code(1)
        .stdout(contains(
            "src/lib.rs:1  error  function-lines  wide  measured 22, limit 5",
        ))
        .stdout(contains("1 error and 0 warnings across 1 file and 1 unit."));
}

#[test]
fn a_finding_left_as_a_warning_is_reported_without_failing_the_gate() {
    let directory = project(&[
        (
            "jabuti.toml",
            "[rules]\nfunction-lines = { limit = 5, severity = \"warning\" }\n",
        ),
        ("src/lib.rs", &function_of("wide", 20)),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("0 errors and 1 warning"));
}

#[test]
fn every_unit_in_the_tree_is_counted_not_only_the_top_level_ones() {
    let directory = project(&[(
        "src/lib.rs",
        "struct Holder;\n\nimpl Holder {\n    fn method() {\n        let closure = || 1;\n    }\n}\n",
    )]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("1 file and 4 units"));
}

#[test]
fn a_file_that_does_not_parse_is_named_on_stderr_and_left_out_of_the_count() {
    let directory = project(&[
        ("src/lib.rs", "fn small() {}\n"),
        ("src/broken.rs", "fn truncated() {\n"),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("1 file and 1 unit"))
        .stderr(contains("could not analyse src/broken.rs"));
}

#[test]
fn an_excluded_path_is_not_analysed_at_all() {
    let directory = project(&[
        ("jabuti.toml", "exclude = [\"generated/**\"]\n"),
        ("src/lib.rs", "fn small() {}\n"),
        ("generated/tables.rs", &function_of("huge", 400)),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout("No findings across 1 file and 1 unit.\n");
}

#[test]
fn a_file_of_another_language_is_not_analysed() {
    let directory = project(&[
        ("src/lib.rs", "fn small() {}\n"),
        ("README.md", "fn this_is_prose_not_code() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout("No findings across 1 file and 1 unit.\n");
}

#[test]
fn an_unknown_rule_in_the_configuration_stops_the_run() {
    let directory = project(&[
        (
            "jabuti.toml",
            "[rules]\nspline-reticulation = { limit = 3 }\n",
        ),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .code(2)
        .stderr(contains("unknown rule spline-reticulation"));
}

#[test]
fn an_unknown_severity_in_the_configuration_stops_the_run() {
    let directory = project(&[
        (
            "jabuti.toml",
            "[rules]\nfunction-lines = { severity = \"catastrophic\" }\n",
        ),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .code(2)
        .stderr(contains("unknown severity catastrophic"));
}

#[test]
fn findings_are_reported_in_path_then_line_order() {
    let directory = project(&[
        ("jabuti.toml", &error_on_long_functions(5)),
        ("src/b.rs", &function_of("second", 20)),
        ("src/a.rs", &function_of("first", 20)),
    ]);

    let output = jabuti(&directory)
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let rendered = String::from_utf8(output).expect("utf8");
    let first = rendered.find("src/a.rs").expect("a is reported");
    let second = rendered.find("src/b.rs").expect("b is reported");

    assert!(first < second, "{rendered}");
}

#[test]
fn a_kotlin_file_is_measured_against_kotlins_own_limit() {
    let body = "    val value = 1\n".repeat(50);
    let directory = project(&[("src/Main.kt", &format!("fun wide() {{\n{body}}}\n"))]);

    jabuti(&directory).assert().success().stdout(contains(
        "src/Main.kt:1  warning  function-lines  wide  measured 52, limit 47",
    ));
}

#[test]
fn a_language_limit_written_in_the_configuration_wins() {
    let body = "    val value = 1\n".repeat(50);
    let directory = project(&[
        (
            "jabuti.toml",
            "[languages.kotlin.rules]\nfunction-lines = { limit = 80 }\n",
        ),
        ("src/Main.kt", &format!("fun wide() {{\n{body}}}\n")),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_language_nobody_supports_stops_the_run() {
    let directory = project(&[
        (
            "jabuti.toml",
            "[languages.cobol.rules]\nfunction-lines = { limit = 80 }\n",
        ),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .code(2)
        .stderr(contains("unknown language cobol"));
}
