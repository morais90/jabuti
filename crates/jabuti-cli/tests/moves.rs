mod common;

use common::{binary, function_of, jabuti, repository, write};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

fn split_out(moved: &str) -> tempfile::TempDir {
    let directory = repository(&[(
        "src/lib.rs",
        &format!("{}\nfn small() {{}}\n", function_of("legacy", 70)),
    )]);

    write(&directory, "src/lib.rs", "fn small() {}\n");
    write(&directory, "src/legacy.rs", moved);
    directory
}

#[test]
fn a_long_function_moved_to_a_new_file_is_not_reported() {
    let directory = split_out(&function_of("legacy", 70));

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_long_function_that_moved_and_grew_is_reported_where_it_landed() {
    let directory = split_out(&function_of("legacy", 75));

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .code(1)
        .stdout(contains(
            "src/legacy.rs:1  function-lines  legacy  measured 77",
        ));
}

#[test]
fn a_public_function_moved_to_a_new_file_is_not_new_api_but_one_written_there_is() {
    let directory = repository(&[(
        "src/lib.rs",
        "pub fn exposed() -> u32 {\n    1\n}\n\npub fn kept() -> u32 {\n    exposed()\n}\n",
    )]);

    write(
        &directory,
        "src/lib.rs",
        "pub fn kept() -> u32 {\n    2\n}\n",
    );
    write(
        &directory,
        "src/api.rs",
        "pub fn exposed() -> u32 {\n    1\n}\n\npub fn added() -> u32 {\n    3\n}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .code(1)
        .stdout(contains("src/api.rs:5  speculative-api  added").and(contains("exposed").not()));
}

#[test]
fn a_long_function_moved_out_of_a_file_the_change_deleted_is_not_reported() {
    let directory = repository(&[
        ("src/lib.rs", "mod a;\n"),
        ("src/old.rs", &function_of("legacy", 70)),
    ]);

    std::fs::remove_file(directory.path().join("src/old.rs")).expect("old file is removed");
    write(
        &directory,
        "src/a.rs",
        &format!("fn first() {{}}\n\n{}", function_of("legacy", 70)),
    );
    write(
        &directory,
        "src/b.rs",
        "fn second() {}\n\nfn third() {}\n\nfn fourth() {}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_function_deleted_outside_the_checked_project_does_not_vouch_for_one_inside_it() {
    let directory = repository(&[
        ("app/src/lib.rs", "mod a;\n"),
        ("other/src/old.rs", &function_of("legacy", 70)),
    ]);

    std::fs::remove_file(directory.path().join("other/src/old.rs")).expect("old file is removed");
    write(&directory, "app/src/a.rs", &function_of("legacy", 70));

    binary(&directory)
        .current_dir(directory.path().join("app"))
        .args(["check", ".", "--since", "HEAD"])
        .assert()
        .code(1)
        .stdout(contains("src/a.rs:1  function-lines  legacy  measured 72"));
}

#[test]
fn a_function_in_a_file_that_no_longer_parses_does_not_vouch_for_a_new_namesake() {
    let directory = repository(&[("src/a.rs", &function_of("parse", 80))]);

    write(&directory, "src/a.rs", "fn parse( {\n");
    write(&directory, "src/c.rs", &function_of("parse", 70));

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .code(1)
        .stdout(contains("src/c.rs:1  function-lines  parse  measured 72"));
}

fn moved_from_deleted(old: &str, setup: &[(&str, &str)], check: &[&str]) -> String {
    let mut files = vec![("src/lib.rs", "mod a;\n")];
    files.extend_from_slice(setup);
    let legacy = function_of("legacy", 70);
    files.push((old, &legacy));
    let directory = repository(&files);

    std::fs::remove_file(directory.path().join(old)).expect("old file is removed");
    write(&directory, "src/a.rs", &legacy);

    let output = binary(&directory)
        .arg("check")
        .args(check)
        .args(["--since", "HEAD"])
        .output()
        .expect("jabuti runs");
    String::from_utf8(output.stdout).expect("output is text")
}

#[test]
fn a_function_deleted_where_the_check_does_not_look_does_not_vouch_for_one_written_by_hand() {
    let reported = "src/a.rs:1  function-lines  legacy  measured 72";

    assert_eq!(
        [
            moved_from_deleted(
                "samples/old.rs",
                &[("jabuti.toml", "exclude = [\"samples/\"]\n")],
                &["."]
            ),
            moved_from_deleted("vendor/old.rs", &[], &["."]),
            moved_from_deleted("other/old.rs", &[], &["src"]),
            moved_from_deleted("lib/old.rs", &[], &["."]),
        ]
        .map(|output| output.contains(reported)),
        [true, true, true, false]
    );
}

#[test]
fn a_deleted_file_that_did_not_parse_before_raises_no_notice_about_its_findings() {
    let directory = repository(&[
        ("src/lib.rs", "fn kept() {}\n"),
        ("src/broken.rs", "fn broken( {\n"),
    ]);

    std::fs::remove_file(directory.path().join("src/broken.rs")).expect("broken file is removed");
    write(&directory, "src/lib.rs", "fn kept() {}\n\nfn added() {}\n");

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("could not be read").not())
        .stderr(contains("could not be read").not());
}
