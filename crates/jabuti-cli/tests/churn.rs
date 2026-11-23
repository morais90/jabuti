mod common;

use common::{commit, jabuti, project, repository, write};
use predicates::str::contains;
use serde_json::Value;

fn churn_of(directory: &tempfile::TempDir) -> Vec<(String, String, Option<u64>)> {
    let output = jabuti(directory)
        .args(["--all", "--format", "measures"])
        .output()
        .expect("jabuti runs");
    let document: Value = serde_json::from_slice(&output.stdout).expect("measures are JSON");

    document["measures"]
        .as_array()
        .expect("a list of measures")
        .iter()
        .map(|reading| {
            (
                reading["path"].as_str().unwrap_or_default().to_owned(),
                reading["kind"].as_str().unwrap_or_default().to_owned(),
                reading["values"]["churn"].as_u64(),
            )
        })
        .collect()
}

fn every_commit_is_counted_against(name: &str) {
    let directory = repository(&[(name, "fn busy() {}\n")]);

    write(&directory, name, "fn busy() { }\n");
    commit(&directory, "second");
    write(&directory, name, "fn busy() {  }\n");
    commit(&directory, "third");

    assert_eq!(
        churn_of(&directory),
        [
            (name.to_owned(), "file".to_owned(), Some(3)),
            (name.to_owned(), "function".to_owned(), None),
        ]
    );
}

#[test]
fn churn_counts_every_commit_that_touched_the_file() {
    every_commit_is_counted_against("src/busy.rs");
}

#[test]
fn a_file_named_with_an_accent_has_every_commit_counted() {
    every_commit_is_counted_against("src/ação.rs");
}

#[test]
fn the_commit_that_renamed_a_file_counts_against_its_new_name() {
    let directory = repository(&[("src/old.rs", "fn busy() {}\n")]);

    common::git(&directory, &["mv", "src/old.rs", "src/new.rs"]);
    commit(&directory, "rename");
    write(&directory, "src/new.rs", "fn busy() { }\n");
    commit(&directory, "edit");

    assert_eq!(
        churn_of(&directory),
        [
            ("src/new.rs".to_owned(), "file".to_owned(), Some(2)),
            ("src/new.rs".to_owned(), "function".to_owned(), None),
        ]
    );
}

#[test]
fn history_outside_a_repository_is_said_to_be_missing_rather_than_failing_the_map() {
    let directory = project(&[("src/lib.rs", "fn small() {}\n")]);

    jabuti(&directory)
        .arg("--all")
        .assert()
        .success()
        .stdout(contains("No findings"))
        .stderr(contains(
            "jabuti: hotspot needs a git repository, so it was not evaluated (",
        ));
}

fn tangled(name: &str) -> String {
    format!(
        "fn {name}(a: bool, b: bool) {{\n    if a {{\n        if b {{\n            if a && b {{\n                return;\n            }}\n        }}\n    }}\n}}\n"
    )
}

#[test]
fn a_file_both_busier_and_more_tangled_than_nine_in_ten_is_a_hotspot() {
    let calm: Vec<(String, String)> = (0..11)
        .map(|index| {
            (
                format!("src/calm_{index:02}.rs"),
                "fn calm() {}\n".to_owned(),
            )
        })
        .collect();
    let mut files: Vec<(&str, &str)> = calm
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    let first = tangled("tangled");
    files.push(("src/tangled.rs", &first));
    let directory = repository(&files);

    write(&directory, "src/tangled.rs", &format!("{first}\n"));
    commit(&directory, "second");
    write(&directory, "src/tangled.rs", &format!("{first}\n\n"));
    commit(&directory, "third");

    jabuti(&directory)
        .arg("--all")
        .assert()
        .success()
        .stdout(
            "No findings across 12 files and 12 units.\n\n\
             Context, which does not fail the run:\n\
             src/tangled.rs:1  hotspot  measured 91, limit 90\n",
        )
        .stderr("");

    write(&directory, "src/tangled.rs", &format!("{first}\n\n\n"));
    jabuti(&directory).assert().success().stdout(contains(
        "Context, which does not fail the run:\n\
             src/tangled.rs:1  hotspot  measured 91, limit 90\n",
    ));
}

#[test]
fn the_measures_of_a_change_still_carry_how_often_each_file_changed() {
    let directory = repository(&[("src/busy.rs", "fn busy() {}\n")]);
    write(&directory, "src/busy.rs", "fn busy() { }\n");
    commit(&directory, "second");
    write(&directory, "src/busy.rs", "fn busy() {  }\n");

    jabuti(&directory)
        .args(["--format", "measures"])
        .assert()
        .success()
        .stdout(contains("\"churn\": 2"));
}
