mod common;

use common::{binary, repository};
use predicates::str::contains;

#[test]
fn the_configuration_is_found_above_the_directory_the_command_runs_from() {
    let directory = repository(&[
        (
            "jabuti.toml",
            "[rules]\nfunction-lines = { limit = 3, severity = \"error\" }\n",
        ),
        (
            "src/lib.rs",
            "pub fn wide() -> usize {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n    a + b + c\n}\n",
        ),
    ]);

    binary(&directory)
        .current_dir(directory.path().join("src"))
        .arg("check")
        .arg(".")
        .assert()
        .code(1)
        .stdout(contains("src/lib.rs:1  error  function-lines"));
}

#[test]
fn paths_are_shown_relative_to_the_project_wherever_the_command_runs_from() {
    let directory = repository(&[
        ("jabuti.toml", "[rules]\n"),
        (
            "src/deep/inner.rs",
            "pub fn read() -> usize {\n    let value: Option<usize> = None;\n    value.unwrap()\n}\n",
        ),
    ]);

    binary(&directory)
        .current_dir(directory.path().join("src/deep"))
        .arg("check")
        .arg(".")
        .assert()
        .success()
        .stdout(contains("src/deep/inner.rs:3  warning  error-masking"));
}

#[test]
fn a_directory_named_tests_above_the_project_does_not_make_the_project_test_code() {
    let directory = repository(&[
        ("tests/project/jabuti.toml", "[rules]\n"),
        (
            "tests/project/src/lib.rs",
            "pub fn read() -> usize {\n    let value: Option<usize> = None;\n    value.unwrap()\n}\n",
        ),
    ]);

    binary(&directory)
        .current_dir(directory.path().join("tests/project"))
        .arg("check")
        .arg(".")
        .assert()
        .success()
        .stdout(contains("src/lib.rs:3  warning  error-masking"));
}

#[test]
fn a_new_file_is_still_part_of_the_change_when_the_command_runs_from_below_it() {
    let directory = repository(&[
        ("jabuti.toml", "[rules]\n"),
        ("src/lib.rs", "pub fn a() {}\n"),
    ]);
    common::write(
        &directory,
        "src/deep/inner.rs",
        "pub fn read() -> usize {\n    let value: Option<usize> = None;\n    value.unwrap()\n}\n",
    );

    binary(&directory)
        .current_dir(directory.path().join("src/deep"))
        .arg("check")
        .arg(".")
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("src/deep/inner.rs:3  warning  error-masking"));
}

#[test]
fn a_root_outside_the_project_is_refused_rather_than_shown_half_anchored() {
    let directory = repository(&[
        ("jabuti.toml", "[rules]\n"),
        ("src/lib.rs", "pub fn a() {}\n"),
    ]);
    let sibling = tempfile::TempDir::new().expect("a sibling directory");
    std::fs::write(sibling.path().join("other.rs"), "pub fn b() {}\n").expect("written");

    binary(&directory)
        .arg("check")
        .arg(sibling.path())
        .assert()
        .code(2)
        .stderr(contains("is outside the project at"));
}

#[test]
fn a_configuration_above_the_repository_is_not_picked_up() {
    let outer = tempfile::TempDir::new().expect("an outer directory");
    std::fs::write(
        outer.path().join("jabuti.toml"),
        "[rules]\nfunction-lines = { limit = 1, severity = \"error\" }\n",
    )
    .expect("written");
    let inner = outer.path().join("repo");
    std::fs::create_dir_all(inner.join("src")).expect("created");
    std::fs::write(
        inner.join("src/lib.rs"),
        "pub fn a() {\n    let x = 1;\n    x\n}\n",
    )
    .expect("written");
    let init = std::process::Command::new("git")
        .args(["init", "-q", "-b", "main"])
        .current_dir(&inner)
        .status()
        .expect("git runs");
    assert!(init.success());

    assert_cmd::Command::cargo_bin("jabuti")
        .expect("the binary is built")
        .current_dir(&inner)
        .arg("check")
        .arg(".")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_root_that_does_not_exist_is_an_error_rather_than_an_empty_report() {
    let directory = repository(&[("src/lib.rs", "pub fn a() {}\n")]);

    binary(&directory)
        .arg("check")
        .arg("nope")
        .assert()
        .code(2)
        .stderr(contains("resolving nope"));
}
