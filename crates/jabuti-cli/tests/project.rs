mod common;

use common::{binary, repository, write};
use predicates::prelude::PredicateBooleanExt;
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
fn an_unreadable_file_outside_the_paths_given_is_not_named_when_no_rule_reads_the_whole_project() {
    let directory = repository(&[
        ("src/inside/lib.rs", "fn small() {}\n"),
        ("src/outside/broken.rs", "fn broken( {\n"),
    ]);

    binary(&directory)
        .arg("check")
        .arg("src/inside")
        .assert()
        .success()
        .stdout(contains("No findings"))
        .stdout(contains("src/outside/broken.rs").not());
}

#[test]
fn under_since_the_whole_project_is_read_only_when_a_rule_compares_against_the_revision() {
    let directory = repository(&[
        (
            "jabuti.toml",
            "[rules]\nnew-dependency = { severity = \"off\" }\nspeculative-api = { severity = \"off\" }\n",
        ),
        ("src/inside/lib.rs", "fn small() {}\n"),
        ("src/outside/broken.rs", "fn broken( {\n"),
    ]);
    write(&directory, "src/inside/lib.rs", "fn small() { }\n");

    binary(&directory)
        .arg("check")
        .arg("src/inside")
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"))
        .stdout(contains("src/outside/broken.rs").not());
}

#[test]
fn a_symlink_under_the_paths_given_is_still_reviewed_when_the_whole_project_is_read() {
    let directory = repository(&[
        (
            "jabuti.toml",
            "[rules]\nfunction-lines = { limit = 2, severity = \"error\" }\n",
        ),
        ("src/outside/real.rs", "fn wide() {}\n"),
    ]);
    std::fs::create_dir_all(directory.path().join("src/inside")).expect("directory created");
    std::os::unix::fs::symlink(
        "../outside/real.rs",
        directory.path().join("src/inside/link.rs"),
    )
    .expect("symlink created");
    write(
        &directory,
        "src/outside/real.rs",
        "fn wide() -> u32 {\n    let a = 1;\n    a\n}\n",
    );

    binary(&directory)
        .arg("check")
        .arg("src/inside")
        .arg("--since")
        .arg("HEAD")
        .assert()
        .code(1)
        .stdout(contains("src/inside/link.rs:1  error  function-lines"));
}

#[test]
fn a_committed_symlink_is_reported_when_the_file_it_points_at_changes() {
    let directory = repository(&[
        (
            "jabuti.toml",
            "[rules]\nfunction-lines = { limit = 2, severity = \"error\" }\n",
        ),
        ("src/outside/real.rs", "fn wide() {}\n"),
    ]);
    std::fs::create_dir_all(directory.path().join("src/inside")).expect("directory created");
    std::os::unix::fs::symlink(
        "../outside/real.rs",
        directory.path().join("src/inside/link.rs"),
    )
    .expect("symlink created");
    common::commit(&directory, "link the file");
    write(
        &directory,
        "src/outside/real.rs",
        "fn wide() -> u32 {\n    let a = 1;\n    a\n}\n",
    );

    binary(&directory)
        .arg("check")
        .arg("src/inside")
        .arg("--since")
        .arg("HEAD")
        .assert()
        .code(1)
        .stdout(contains("src/inside/link.rs:1  error  function-lines"));
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
    write(
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

#[test]
fn a_directory_named_like_a_source_file_is_not_read_as_one() {
    let directory = repository(&[
        ("src/lib.rs", "fn small() {}\n"),
        ("src/generated.rs/notes.txt", "not source\n"),
    ]);

    binary(&directory)
        .arg("check")
        .arg(".")
        .assert()
        .success()
        .stdout("No findings across 1 file and 1 unit.\n");
}
