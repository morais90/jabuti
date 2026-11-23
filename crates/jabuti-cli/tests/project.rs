mod common;

use common::{binary, function_of, repository, write};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

#[test]
fn the_configuration_is_found_above_the_directory_the_command_runs_from() {
    let directory = repository(&[
        ("jabuti.toml", "[rules]\n"),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    binary(&directory)
        .current_dir(directory.path().join("src"))
        .arg("check")
        .arg("--all")
        .arg(".")
        .assert()
        .code(2)
        .stderr(contains("[rules] is not read"));
}

#[test]
fn an_unreadable_file_outside_the_paths_given_is_not_named_when_no_rule_reads_the_whole_project() {
    let directory = repository(&[
        ("src/inside/lib.rs", "fn small() {}\n"),
        ("src/outside/broken.rs", "fn broken( {\n"),
    ]);

    binary(&directory)
        .arg("check")
        .arg("--all")
        .arg("src/inside")
        .assert()
        .success()
        .stdout(contains("No findings"))
        .stdout(contains("src/outside/broken.rs").not());
}

fn linked_into_the_paths_given(committed: bool) -> tempfile::TempDir {
    let directory = repository(&[("src/outside/real.rs", "fn wide() {}\n")]);
    std::fs::create_dir_all(directory.path().join("src/inside")).expect("directory created");
    std::os::unix::fs::symlink(
        "../outside/real.rs",
        directory.path().join("src/inside/link.rs"),
    )
    .expect("symlink created");
    if committed {
        common::commit(&directory, "link the file");
    }
    write(&directory, "src/outside/real.rs", &function_of("wide", 70));

    directory
}

fn the_link_is_reported(directory: &tempfile::TempDir) {
    binary(directory)
        .arg("check")
        .arg("src/inside")
        .arg("--since")
        .arg("HEAD")
        .assert()
        .code(1)
        .stdout(contains(
            "src/inside/link.rs:1  function-lines  wide  measured 72, limit 60",
        ));
}

#[test]
fn a_symlink_under_the_paths_given_is_still_reviewed_when_the_whole_project_is_read() {
    the_link_is_reported(&linked_into_the_paths_given(false));
}

#[test]
fn a_committed_symlink_is_reported_when_the_file_it_points_at_changes() {
    the_link_is_reported(&linked_into_the_paths_given(true));
}

const MASKED_READ: &str =
    "pub fn read() -> usize {\n    let value: Option<usize> = None;\n    value.unwrap()\n}\n";

fn masking_reported(project: &str, source: &str, run_from: &str, expected: &str) {
    let marker = format!("{project}jabuti.toml");
    let file = format!("{project}{source}");
    let directory = repository(&[(&marker, ""), (&file, MASKED_READ)]);

    binary(&directory)
        .current_dir(directory.path().join(project).join(run_from))
        .arg("check")
        .arg("--all")
        .arg(".")
        .assert()
        .success()
        .stdout(contains(expected));
}

#[test]
fn paths_are_shown_relative_to_the_project_wherever_the_command_runs_from() {
    masking_reported(
        "",
        "src/deep/inner.rs",
        "src/deep",
        "src/deep/inner.rs:3  error-masking",
    );
}

#[test]
fn a_directory_named_tests_above_the_project_does_not_make_the_project_test_code() {
    masking_reported(
        "tests/project/",
        "src/lib.rs",
        "",
        "src/lib.rs:3  error-masking",
    );
}

#[test]
fn a_new_file_is_still_part_of_the_change_when_the_command_runs_from_below_it() {
    let directory = repository(&[("jabuti.toml", ""), ("src/lib.rs", "pub fn a() {}\n")]);
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
        .code(1)
        .stdout(contains("src/deep/inner.rs:3  error-masking"));
}

#[test]
fn a_root_outside_the_project_is_refused_rather_than_shown_half_anchored() {
    let directory = repository(&[("src/lib.rs", "pub fn a() {}\n")]);
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
    std::fs::write(outer.path().join("jabuti.toml"), "").expect("written");
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
        .arg("--all")
        .arg(".")
        .assert()
        .success()
        .stdout("No findings across 1 file and 1 unit.\n");
}
