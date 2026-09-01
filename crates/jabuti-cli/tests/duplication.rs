mod common;

use common::{
    append, function_of, jabuti, report_duplication_above, repository, shaped_like, write,
};
use predicates::str::contains;

#[test]
fn a_block_copied_into_another_file_is_reported_on_both_sides() {
    let directory = repository(&[
        ("jabuti.toml", &report_duplication_above(40)),
        (
            "src/parser.rs",
            &shaped_like("parse_header", "parts", "name"),
        ),
        ("src/reader.rs", &shaped_like("read_pair", "pieces", "key")),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("src/parser.rs:1  warning  duplicate-block"))
        .stdout(contains("src/reader.rs:1  warning  duplicate-block"));
}

#[test]
fn duplication_stays_quiet_until_a_block_is_large_enough() {
    let directory = repository(&[
        ("jabuti.toml", &report_duplication_above(400)),
        (
            "src/parser.rs",
            &shaped_like("parse_header", "parts", "name"),
        ),
        ("src/reader.rs", &shaped_like("read_pair", "pieces", "key")),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_copy_landing_in_the_change_is_reported() {
    let directory = repository(&[
        ("jabuti.toml", &report_duplication_above(40)),
        (
            "src/parser.rs",
            &shaped_like("parse_header", "parts", "name"),
        ),
    ]);

    write(
        &directory,
        "src/reader.rs",
        &shaped_like("read_pair", "pieces", "key"),
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("src/reader.rs:1  warning  duplicate-block"));
}

#[test]
fn a_copy_that_predates_the_change_is_left_out() {
    let directory = repository(&[
        ("jabuti.toml", &report_duplication_above(40)),
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
        ("jabuti.toml", &report_duplication_above(40)),
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
fn a_repository_wide_rule_cannot_be_set_per_language() {
    let directory = repository(&[
        (
            "jabuti.toml",
            "[languages.rust.rules]\nduplicate-block = { severity = \"off\" }\n",
        ),
        ("src/live.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .code(2)
        .stderr(contains("cannot be set per language"));
}

#[test]
fn the_same_file_reached_through_two_roots_is_not_a_copy_of_itself() {
    let directory = repository(&[
        ("jabuti.toml", &report_duplication_above(40)),
        (
            "src/parser.rs",
            &shaped_like("parse_header", "parts", "name"),
        ),
        ("src/small.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .arg("src")
        .assert()
        .success()
        .stdout("No findings across 2 files and 2 units.\n");
}
