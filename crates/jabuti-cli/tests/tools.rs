mod common;

use common::{jabuti, project};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

fn tools(directory: &tempfile::TempDir) -> assert_cmd::Command {
    let mut command = assert_cmd::Command::cargo_bin("jabuti").expect("the binary is built");
    command.current_dir(directory.path()).arg("tools");
    command
}

#[test]
fn a_tool_says_it_does_not_apply_when_the_project_has_no_marker_for_it() {
    let directory = project(&[("src/lib.rs", "fn small() {}\n")]);

    tools(&directory)
        .assert()
        .success()
        .stdout(contains("clippy").and(contains("not applicable here")));
}

#[test]
fn an_applicable_tool_that_is_off_says_how_to_turn_it_on() {
    let directory = project(&[
        (
            "Cargo.toml",
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
        ),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    tools(&directory)
        .assert()
        .success()
        .stdout(contains("enable with [tools.clippy] enabled = true"));
}

#[test]
fn a_tool_nobody_has_heard_of_stops_the_run() {
    let directory = project(&[
        ("jabuti.toml", "[tools.spline]\nenabled = true\n"),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .code(2)
        .stderr(contains("unknown tool spline"));
}

#[test]
fn a_disabled_tool_never_runs_even_where_it_applies() {
    let directory = project(&[
        (
            "Cargo.toml",
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
        ),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

fn rust_project(extra: &[(&str, &str)]) -> tempfile::TempDir {
    let mut files = vec![
        (
            "Cargo.toml",
            "[package]\nname = \"lintme\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        ),
        (
            "src/lib.rs",
            "pub fn sum(values: &[i32]) -> i32 {\n    let mut total = 0;\n    for index in 0..values.len() {\n        total += values[index];\n    }\n    total\n}\n",
        ),
    ];
    files.extend_from_slice(extra);

    project(&files)
}

const CLIPPY_ON: &str = "[tools.clippy]\nenabled = true\n";

#[test]
fn an_enabled_and_available_tool_says_it_will_run() {
    let directory = rust_project(&[("jabuti.toml", CLIPPY_ON)]);

    tools(&directory)
        .assert()
        .success()
        .stdout(contains("clippy").and(contains("will run")));
}

#[test]
fn a_lint_from_the_tool_is_reported_like_any_other_finding() {
    let directory = rust_project(&[("jabuti.toml", CLIPPY_ON)]);

    jabuti(&directory)
        .assert()
        .stdout(contains("src/lib.rs:3").and(contains("clippy/needless_range_loop")));
}

#[test]
fn a_lint_the_configuration_switches_off_is_not_reported() {
    let directory = rust_project(&[(
        "jabuti.toml",
        "[tools.clippy]\nenabled = true\n\n[rules]\n\"clippy/needless_range_loop\" = { severity = \"off\" }\n",
    )]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_tool_that_cannot_be_found_says_how_to_install_it() {
    let directory = rust_project(&[("jabuti.toml", CLIPPY_ON)]);

    tools(&directory)
        .env("PATH", "")
        .assert()
        .success()
        .stdout(contains("install with `rustup component add clippy`"));
}

#[test]
fn a_tool_that_fails_without_reporting_anything_says_so_and_does_not_pass_silently() {
    let directory = project(&[
        ("jabuti.toml", CLIPPY_ON),
        ("Cargo.toml", "[package]\nthis is not valid toml at all\n"),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stderr(contains("clippy exited without reporting anything"));
}

#[test]
fn a_lint_outside_the_changed_lines_is_left_out_when_scoping_to_a_diff() {
    let directory = rust_project(&[("jabuti.toml", CLIPPY_ON)]);
    common::init_repository(&directory);
    common::write(&directory, "src/other.rs", "pub fn added() {}\n");

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_tool_that_finds_nothing_is_not_mistaken_for_a_tool_that_failed() {
    let directory = project(&[
        ("jabuti.toml", CLIPPY_ON),
        (
            "Cargo.toml",
            "[package]\nname = \"clean\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        ),
        (
            "src/lib.rs",
            "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
        ),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"))
        .stderr(contains("exited without reporting anything").not());
}
