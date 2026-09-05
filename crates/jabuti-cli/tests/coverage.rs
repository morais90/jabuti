mod common;

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use common::{jabuti, project, repository, write};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use tempfile::TempDir;

const CARGO_TOML: &str = "[package]\nname = \"covme\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";

const REPORTED: &str = "[coverage]\nreport = \"coverage.lcov\"\n\n[rules]\nhotspot = { severity = \"off\" }\nuncovered-new-code = { severity = \"warning\" }\n";

const BEFORE: &str = "fn existing() -> i32 {\n    1\n}\n";

const AFTER: &str = "fn existing() -> i32 {\n    1\n}\n\nfn added() -> i32 {\n    let first = 2;\n    let second = 3;\n    first + second\n}\n";

fn rust_repository(configuration: &str) -> TempDir {
    repository(&[
        ("jabuti.toml", configuration),
        ("Cargo.toml", CARGO_TOML),
        ("src/lib.rs", BEFORE),
    ])
}

fn lcov(directory: &TempDir, file: &str, hits: &[(u32, u32)]) -> String {
    let absolute = directory
        .path()
        .canonicalize()
        .expect("temporary directory resolves")
        .join(file);
    let mut text = format!("SF:{}\n", absolute.display());
    for (line, count) in hits {
        writeln!(text, "DA:{line},{count}").expect("writing to a string never fails");
    }
    text.push_str("end_of_record\n");

    text
}

fn make_old(directory: &TempDir, name: &str) {
    let file = fs::File::options()
        .write(true)
        .open(directory.path().join(name))
        .expect("report exists");
    file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_hours(24))
        .expect("modification time set");
}

fn since_head(directory: &TempDir) -> assert_cmd::Command {
    let mut command = jabuti(directory);
    command.arg("--since").arg("HEAD");
    command
}

#[test]
fn a_stretch_of_new_lines_no_test_ran_is_reported_once_at_its_first_line() {
    let directory = rust_repository(REPORTED);
    write(&directory, "src/lib.rs", AFTER);
    let report = lcov(
        &directory,
        "src/lib.rs",
        &[(1, 1), (2, 1), (3, 1), (5, 0), (6, 0), (7, 0), (8, 0)],
    );
    write(&directory, "coverage.lcov", &report);

    since_head(&directory)
        .assert()
        .success()
        .stdout(
            "0 errors and 1 warning across 1 file and 2 units.\n\nsrc/lib.rs:5  warning  uncovered-new-code  4 lines run by no test\n",
        )
        .stderr("");
}

#[test]
fn an_old_line_no_test_runs_is_not_reported_when_the_change_did_not_touch_it() {
    let directory = rust_repository(REPORTED);
    write(&directory, "src/lib.rs", AFTER);
    let report = lcov(
        &directory,
        "src/lib.rs",
        &[(1, 1), (2, 0), (3, 1), (5, 1), (6, 1), (7, 1), (8, 1)],
    );
    write(&directory, "coverage.lcov", &report);

    since_head(&directory)
        .assert()
        .success()
        .stdout("No findings across 1 file and 2 units.\n");
}

#[test]
fn a_new_line_a_test_ran_is_not_reported_and_the_stretch_starts_where_coverage_stops() {
    let directory = rust_repository(REPORTED);
    write(&directory, "src/lib.rs", AFTER);
    let report = lcov(
        &directory,
        "src/lib.rs",
        &[(1, 1), (2, 1), (3, 1), (5, 1), (6, 1), (7, 0), (8, 0)],
    );
    write(&directory, "coverage.lcov", &report);

    since_head(&directory).assert().success().stdout(contains(
        "src/lib.rs:7  warning  uncovered-new-code  2 lines run by no test",
    ));
}

#[test]
fn a_file_the_report_does_not_mention_is_left_alone() {
    let directory = rust_repository(REPORTED);
    write(&directory, "src/lib.rs", AFTER);
    let report = lcov(&directory, "src/other.rs", &[(1, 0), (2, 0)]);
    write(&directory, "coverage.lcov", &report);

    since_head(&directory)
        .assert()
        .success()
        .stdout("No findings across 1 file and 2 units.\n")
        .stderr("");
}

#[test]
fn a_test_file_is_never_reported_as_uncovered() {
    let directory = rust_repository(REPORTED);
    write(
        &directory,
        "tests/added.rs",
        "#[test]\nfn adds() {\n    assert_eq!(1 + 1, 2);\n}\n",
    );
    let report = lcov(&directory, "tests/added.rs", &[(2, 0), (3, 0), (4, 0)]);
    write(&directory, "coverage.lcov", &report);

    since_head(&directory)
        .assert()
        .success()
        .stdout("No findings across 1 file and 1 unit.\n");
}

#[test]
fn a_report_older_than_a_changed_file_is_skipped_rather_than_read() {
    let directory = rust_repository(REPORTED);
    write(&directory, "src/lib.rs", AFTER);
    let report = lcov(
        &directory,
        "src/lib.rs",
        &[(1, 1), (2, 1), (3, 1), (5, 0), (6, 0), (7, 0), (8, 0)],
    );
    write(&directory, "coverage.lcov", &report);
    make_old(&directory, "coverage.lcov");

    since_head(&directory)
        .assert()
        .success()
        .stdout("No findings across 1 file and 2 units.\n")
        .stderr("jabuti: uncovered-new-code skipped: coverage.lcov is older than src/lib.rs\n");
}

#[test]
fn a_configured_report_that_does_not_exist_is_named_in_the_skip_notice() {
    let directory = rust_repository(REPORTED);
    write(&directory, "src/lib.rs", AFTER);

    since_head(&directory)
        .assert()
        .success()
        .stdout("No findings across 1 file and 2 units.\n")
        .stderr(contains(
            "jabuti: uncovered-new-code skipped: coverage.lcov ",
        ));
}

#[test]
fn a_configured_report_with_an_extension_nobody_reads_stops_the_run() {
    let directory = rust_repository(
        "[coverage]\nreport = \"coverage.txt\"\n\n[rules]\nhotspot = { severity = \"off\" }\nuncovered-new-code = { severity = \"warning\" }\n",
    );

    since_head(&directory).assert().code(2).stderr(contains(
        "coverage report coverage.txt has an extension jabuti cannot read; use .lcov, .info or .xml",
    ));
}

#[test]
fn the_rule_says_it_needs_a_report_when_nothing_provides_one() {
    let directory = rust_repository(
        "[rules]\nhotspot = { severity = \"off\" }\nuncovered-new-code = { severity = \"warning\" }\n",
    );
    write(&directory, "src/lib.rs", AFTER);

    since_head(&directory)
        .assert()
        .success()
        .stdout("No findings across 1 file and 2 units.\n")
        .stderr(
            "jabuti: uncovered-new-code needs a coverage report; set [coverage] report or enable a tool that produces one\n",
        );
}

#[test]
fn the_rule_stays_quiet_while_it_is_off_even_when_no_report_exists() {
    let directory = rust_repository("[rules]\nhotspot = { severity = \"off\" }\n");
    write(&directory, "src/lib.rs", AFTER);

    since_head(&directory).assert().success().stderr("");
}

#[test]
fn a_rule_set_to_gate_says_it_cannot_run_without_a_reference_instead_of_passing() {
    let directory = rust_repository(
        "[coverage]\nreport = \"coverage.lcov\"\n\n[rules]\nhotspot = { severity = \"off\" }\nuncovered-new-code = { severity = \"error\" }\n",
    );

    jabuti(&directory).assert().success().stderr(contains(
        "uncovered-new-code compares against an earlier revision, so it needs --since",
    ));
}

#[test]
fn a_jacoco_report_locates_a_kotlin_file_by_its_package_directory() {
    let directory = repository(&[
        (
            "jabuti.toml",
            "[coverage]\nreport = \"build/jacoco.xml\"\n\n[rules]\nhotspot = { severity = \"off\" }\nuncovered-new-code = { severity = \"warning\" }\n",
        ),
        (
            "src/main/kotlin/org/example/Catalog.kt",
            "package org.example\n\nclass Catalog {\n    fun size(): Int = 0\n}\n",
        ),
    ]);
    write(
        &directory,
        "src/main/kotlin/org/example/Catalog.kt",
        "package org.example\n\nclass Catalog {\n    fun size(): Int = 0\n}\n\nprivate fun added(): Int {\n    val value = 2\n    return value + 1\n}\n",
    );
    write(
        &directory,
        "build/jacoco.xml",
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<!DOCTYPE report PUBLIC \"-//JACOCO//DTD Report 1.1//EN\" \"report.dtd\">\n<report name=\"app\">\n  <package name=\"org/example\">\n    <sourcefile name=\"Catalog.kt\">\n      <line nr=\"4\" mi=\"0\" ci=\"2\" mb=\"0\" cb=\"0\"/>\n      <line nr=\"8\" mi=\"2\" ci=\"0\" mb=\"0\" cb=\"0\"/>\n      <line nr=\"9\" mi=\"3\" ci=\"0\" mb=\"0\" cb=\"0\"/>\n    </sourcefile>\n  </package>\n</report>\n",
    );

    since_head(&directory).assert().success().stdout(contains(
        "src/main/kotlin/org/example/Catalog.kt:8  warning  uncovered-new-code  2 lines run by no test",
    ));
}

fn tools(directory: &TempDir) -> assert_cmd::Command {
    let mut command = common::binary(directory);
    command.arg("tools");
    command
}

#[test]
fn the_coverage_producer_says_it_does_not_apply_outside_a_cargo_project() {
    let directory = project(&[("src/lib.rs", "fn small() {}\n")]);

    tools(&directory)
        .assert()
        .success()
        .stdout(contains("cargo-llvm-cov").and(contains("not applicable here")));
}

#[test]
fn the_coverage_producer_says_how_to_install_itself_when_it_cannot_be_found() {
    let directory = project(&[("Cargo.toml", CARGO_TOML), ("src/lib.rs", BEFORE)]);

    tools(&directory)
        .env("PATH", "")
        .assert()
        .success()
        .stdout(contains(
            "install with `cargo install cargo-llvm-cov && rustup component add llvm-tools-preview`",
        ));
}

#[test]
fn the_coverage_producer_says_how_to_turn_itself_on() {
    let directory = project(&[("Cargo.toml", CARGO_TOML), ("src/lib.rs", BEFORE)]);

    tools(&directory).assert().success().stdout(contains(
        "enable with [tools.cargo-llvm-cov] enabled = true",
    ));
}

#[test]
fn the_coverage_producer_runs_the_tests_and_the_rule_reads_what_it_wrote() {
    let directory = repository(&[
        (
            "jabuti.toml",
            "[tools.cargo-llvm-cov]\nenabled = true\n\n[rules]\nhotspot = { severity = \"off\" }\nuncovered-new-code = { severity = \"warning\" }\n",
        ),
        ("Cargo.toml", CARGO_TOML),
        (
            "src/lib.rs",
            "pub fn tested(value: i32) -> i32 {\n    value + 1\n}\n",
        ),
        (
            "tests/tested.rs",
            "#[test]\nfn adds_one() {\n    assert_eq!(covme::tested(1), 2);\n}\n",
        ),
    ]);
    write(
        &directory,
        "src/lib.rs",
        "pub fn tested(value: i32) -> i32 {\n    value + 1\n}\n\npub fn untested(value: i32) -> i32 {\n    let doubled = value * 2;\n    doubled + 1\n}\n",
    );

    since_head(&directory)
        .assert()
        .success()
        .stdout(contains(
            "src/lib.rs:5  warning  uncovered-new-code  4 lines run by no test",
        ))
        .stderr("");

    assert!(
        directory
            .path()
            .join("target/jabuti/coverage.lcov")
            .is_file(),
        "the producer writes its report under target/jabuti"
    );
}

#[test]
fn the_coverage_producer_does_not_inherit_the_instrumentation_of_a_run_it_is_nested_in() {
    let directory = repository(&[
        (
            "jabuti.toml",
            "[tools.cargo-llvm-cov]\nenabled = true\n\n[rules]\nhotspot = { severity = \"off\" }\nuncovered-new-code = { severity = \"warning\" }\n",
        ),
        ("Cargo.toml", CARGO_TOML),
        ("src/lib.rs", BEFORE),
    ]);
    write(&directory, "src/lib.rs", AFTER);

    since_head(&directory)
        .env("__CARGO_LLVM_COV_RUSTC_WRAPPER", "1")
        .env("__CARGO_LLVM_COV_RUSTC_WRAPPER_CRATE_NAMES", "covme")
        .env("RUSTC_WRAPPER", "/nonexistent/enclosing-coverage-wrapper")
        .assert()
        .success()
        .stdout(contains(
            "src/lib.rs:5  warning  uncovered-new-code  5 lines run by no test",
        ))
        .stderr("");
}

#[test]
fn a_producer_whose_tests_fail_says_so_and_the_rule_does_not_guess() {
    let directory = repository(&[
        (
            "jabuti.toml",
            "[tools.cargo-llvm-cov]\nenabled = true\n\n[rules]\nhotspot = { severity = \"off\" }\nuncovered-new-code = { severity = \"warning\" }\n",
        ),
        ("Cargo.toml", CARGO_TOML),
        ("src/lib.rs", BEFORE),
        (
            "tests/failing.rs",
            "#[test]\nfn fails() {\n    assert_eq!(1, 2);\n}\n",
        ),
    ]);
    write(&directory, "src/lib.rs", AFTER);

    since_head(&directory)
        .assert()
        .success()
        .stdout("No findings across 1 file and 2 units.\n")
        .stderr(
            contains("jabuti: cargo-llvm-cov exited without reporting anything").and(contains(
                "jabuti: uncovered-new-code skipped: target/jabuti/coverage.lcov ",
            )),
        );

    assert!(!Path::new(&directory.path().join("target/jabuti/coverage.lcov")).exists());
}
