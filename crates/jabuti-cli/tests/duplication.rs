mod common;

use common::{append, function_of, jabuti, repository, shaped_like, write};
use predicates::str::contains;

#[test]
fn a_block_copied_into_another_file_is_reported_on_both_sides() {
    let directory = repository(&[
        (
            "src/parser.rs",
            &shaped_like("parse_header", "parts", "name"),
        ),
        ("src/reader.rs", &shaped_like("read_pair", "pieces", "key")),
    ]);

    jabuti(&directory)
        .arg("--all")
        .assert()
        .success()
        .stdout(contains("src/parser.rs:1  duplicate-block"))
        .stdout(contains("src/reader.rs:1  duplicate-block"));
}

#[test]
fn duplication_stays_quiet_until_a_block_is_large_enough() {
    let directory = repository(&[
        (
            "src/parser.rs",
            "fn parse(input: &str) -> usize {\n    input.trim().len() + 1\n}\n",
        ),
        (
            "src/reader.rs",
            "fn read(text: &str) -> usize {\n    text.trim().len() + 1\n}\n",
        ),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_copy_landing_in_the_change_is_reported() {
    let directory = repository(&[(
        "src/parser.rs",
        &shaped_like("parse_header", "parts", "name"),
    )]);

    write(
        &directory,
        "src/reader.rs",
        &shaped_like("read_pair", "pieces", "key"),
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .code(1)
        .stdout(contains("src/reader.rs:1  duplicate-block"));
}

#[test]
fn a_copy_that_predates_the_change_is_left_out() {
    let directory = repository(&[
        (
            "src/parser.rs",
            &shaped_like("parse_header", "parts", "name"),
        ),
        ("src/reader.rs", &shaped_like("read_pair", "pieces", "key")),
        ("src/live.rs", "fn small() {}\n"),
    ]);

    append(&directory, "src/live.rs", "\nfn added() {}\n");

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn measures_stay_scoped_to_the_change_even_though_duplication_reads_everything() {
    let directory = repository(&[
        ("src/legacy.rs", &function_of("legacy", 70)),
        ("src/live.rs", "fn small() {}\n"),
    ]);

    append(&directory, "src/live.rs", "\nfn added() {}\n");

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout("No findings across 1 file and 2 units.\n");
}

#[test]
fn the_same_file_reached_through_two_roots_is_not_a_copy_of_itself() {
    let directory = repository(&[
        (
            "src/parser.rs",
            &shaped_like("parse_header", "parts", "name"),
        ),
        ("src/small.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .arg("--all")
        .arg("src")
        .assert()
        .success()
        .stdout("No findings across 2 files and 2 units.\n");
}
