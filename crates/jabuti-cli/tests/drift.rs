mod common;

use std::fs;

use common::{commit, jabuti, repository, write};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

fn project() -> tempfile::TempDir {
    repository(&[
        (
            "src/main.rs",
            "mod git;\nmod report;\n\nfn main() {\n    println!(\"{}\", report::render());\n}\n",
        ),
        (
            "src/git.rs",
            "pub fn run(arguments: &[&str]) -> String {\n    arguments.join(\" \")\n}\n",
        ),
        (
            "src/report.rs",
            "pub fn render() -> String {\n    String::from(\"nothing\")\n}\n",
        ),
    ])
}

#[test]
fn a_dependency_this_change_introduced_is_reported_where_it_was_written() {
    let directory = project();
    write(
        &directory,
        "src/report.rs",
        "pub fn render() -> String {\n    crate::git::run(&[\"status\"])\n}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains(
            "src/report.rs:2  warning  new-dependency  now depends on src/git.rs",
        ));
}

#[test]
fn a_dependency_that_was_already_there_is_not_reported_again() {
    let directory = project();
    write(
        &directory,
        "src/report.rs",
        "pub fn render() -> String {\n    crate::git::run(&[\"status\"])\n}\n",
    );
    commit(&directory, "reach for git");
    write(
        &directory,
        "src/report.rs",
        "pub fn render() -> String {\n    crate::git::run(&[\"status\", \"--short\"])\n}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_file_this_change_created_reports_no_dependency_because_all_of_them_are_new() {
    let directory = project();
    write(
        &directory,
        "src/extra.rs",
        "fn extra() -> String {\n    crate::git::run(&[\"log\"])\n}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn every_changed_file_is_compared_against_its_own_earlier_revision_even_around_a_created_one() {
    let directory = project();
    write(
        &directory,
        "src/git.rs",
        "pub fn run(arguments: &[&str]) -> String {\n    crate::report::render();\n    arguments.join(\" \")\n}\n",
    );
    write(
        &directory,
        "src/log.rs",
        "fn log() -> String {\n    crate::git::run(&[\"log\"])\n}\n",
    );
    write(
        &directory,
        "src/report.rs",
        "pub fn render() -> String {\n    crate::git::run(&[\"status\"])\n}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains(
            "src/git.rs:2  warning  new-dependency  now depends on src/report.rs",
        ))
        .stdout(contains(
            "src/report.rs:2  warning  new-dependency  now depends on src/git.rs",
        ))
        .stdout(contains("src/log.rs").not());
}

#[test]
fn a_path_that_was_a_directory_at_the_base_does_not_shift_the_next_file_off_its_own_revision() {
    let directory = repository(&[
        (
            "src/main.rs",
            "mod git;\nmod report;\nmod tally;\n\nfn main() {\n    println!(\"{}\", report::render());\n}\n",
        ),
        (
            "src/git.rs",
            "pub fn run(arguments: &[&str]) -> String {\n    arguments.join(\" \")\n}\n",
        ),
        (
            "src/report.rs/mod.rs",
            "pub fn render() -> String {\n    String::new()\n}\n",
        ),
        (
            "src/tally.rs",
            "pub fn tally() -> String {\n    crate::git::run(&[\"count\"])\n}\n",
        ),
    ]);
    fs::remove_dir_all(directory.path().join("src/report.rs")).expect("directory removed");
    write(
        &directory,
        "src/report.rs",
        "pub fn render() -> String {\n    crate::git::run(&[\"status\"])\n}\n",
    );
    commit(&directory, "flatten report");
    write(
        &directory,
        "src/tally.rs",
        "pub fn tally() -> String {\n    crate::report::render()\n}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD~1")
        .assert()
        .success()
        .stdout(contains(
            "src/tally.rs:2  warning  new-dependency  now depends on src/report.rs",
        ))
        .stdout(contains("src/report.rs:2").not());
}

#[test]
fn the_earlier_revision_is_still_read_when_only_this_rule_asks_for_it() {
    let directory = project();
    write(
        &directory,
        "jabuti.toml",
        "[rules]\nspeculative-api = { severity = \"off\" }\n",
    );
    write(
        &directory,
        "src/report.rs",
        "pub fn render() -> String {\n    crate::git::run(&[\"status\"])\n}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains(
            "src/report.rs:2  warning  new-dependency  now depends on src/git.rs",
        ));
}

#[test]
fn a_rule_set_to_gate_says_it_cannot_run_without_a_reference_instead_of_passing() {
    let directory = project();
    write(
        &directory,
        "jabuti.toml",
        "[rules]\nnew-dependency = { severity = \"error\" }\n",
    );

    jabuti(&directory).assert().success().stderr(contains(
        "new-dependency compares against an earlier revision, so it needs --since",
    ));
}

#[test]
fn the_default_warning_stays_quiet_without_a_reference_rather_than_nagging() {
    let directory = project();

    jabuti(&directory).assert().success().stderr("");
}

#[test]
fn the_comparison_finds_the_earlier_file_when_run_from_a_subdirectory() {
    let directory = repository(&[
        ("app/src/main.rs", "mod git;\nmod report;\n"),
        (
            "app/src/git.rs",
            "pub fn run() -> String {\n    String::new()\n}\n",
        ),
        (
            "app/src/report.rs",
            "pub fn render() -> String {\n    String::from(\"nothing\")\n}\n",
        ),
    ]);
    write(
        &directory,
        "app/src/report.rs",
        "pub fn render() -> String {\n    crate::git::run()\n}\n",
    );

    jabuti(&directory)
        .current_dir(directory.path().join("app"))
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains(
            "src/report.rs:2  warning  new-dependency  now depends on src/git.rs",
        ));
}

#[test]
fn a_file_that_starts_using_something_it_declares_itself_is_not_a_dependency() {
    let directory = repository(&[(
        "src/Catalog.kt",
        "package org.example\n\nclass Catalog {\n    fun size(): Int = 0\n}\n",
    )]);
    write(
        &directory,
        "src/Catalog.kt",
        "package org.example\n\nclass Helper\n\nclass Catalog {\n    fun size(): Int = Helper().hashCode()\n}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_language_that_switches_the_rule_off_stops_being_reported() {
    let directory = project();
    write(
        &directory,
        "jabuti.toml",
        "[languages.rust.rules]\nnew-dependency = { severity = \"off\" }\n",
    );
    write(
        &directory,
        "src/report.rs",
        "pub fn render() -> String {\n    crate::git::run(&[\"status\"])\n}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}
