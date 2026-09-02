mod common;

use common::{commit, jabuti, repository, write};
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
        "pub fn extra() -> String {\n    crate::git::run(&[\"log\"])\n}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
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
