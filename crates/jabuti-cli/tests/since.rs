mod common;

use common::{append, error_on_long_functions, function_of, jabuti, repository, write};
use predicates::str::contains;

#[test]
fn a_finding_in_a_file_nobody_touched_is_left_out() {
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
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
fn a_unit_overlapping_a_changed_line_is_reported() {
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        ("src/live.rs", "fn small() {}\n"),
    ]);

    append(
        &directory,
        "src/live.rs",
        &format!("\n{}", function_of("added", 70)),
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .code(1)
        .stdout(contains("src/live.rs:3  error  function-lines  added"));
}

#[test]
fn a_unit_in_a_touched_file_but_away_from_the_change_is_left_out() {
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        (
            "src/live.rs",
            &format!("{}\nfn edited() {{}}\n", function_of("untouched", 70)),
        ),
    ]);

    write(
        &directory,
        "src/live.rs",
        &format!(
            "{}\nfn edited() {{\n    let value = 1;\n}}\n",
            function_of("untouched", 70)
        ),
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn every_unit_of_a_brand_new_file_counts_as_changed() {
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        ("src/live.rs", "fn small() {}\n"),
    ]);

    write(&directory, "src/fresh.rs", &function_of("fresh", 70));

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .code(1)
        .stdout(contains("src/fresh.rs:1  error  function-lines  fresh"));
}

#[test]
fn one_changed_line_inside_a_unit_is_enough_to_report_it() {
    let base = function_of("long", 70);
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        ("src/live.rs", &base),
    ]);

    write(
        &directory,
        "src/live.rs",
        &base.replacen("    let value = 1;\n", "    let value = 2;\n", 1),
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .code(1)
        .stdout(contains("src/live.rs:1  error  function-lines  long"));
}

#[test]
fn a_change_on_the_opening_line_of_a_unit_reports_it() {
    let base = format!("fn header() {{}}\n{}", function_of("long", 70));
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        ("src/live.rs", &base),
    ]);

    write(
        &directory,
        "src/live.rs",
        &base.replacen("fn long() {", "fn long(/* touched */) {", 1),
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .code(1)
        .stdout(contains("src/live.rs:2  error  function-lines  long"));
}

#[test]
fn a_change_on_the_line_before_a_unit_does_not_reach_into_it() {
    let base = format!("fn header() {{}}\n{}", function_of("long", 70));
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        ("src/live.rs", &base),
    ]);

    write(
        &directory,
        "src/live.rs",
        &base.replacen("fn header() {}", "fn header() { }", 1),
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn an_unknown_reference_stops_the_run_rather_than_passing_the_gate() {
    let directory = repository(&[("src/live.rs", "fn small() {}\n")]);

    jabuti(&directory)
        .arg("--since")
        .arg("no-such-branch")
        .assert()
        .code(2)
        .stderr(contains("git diff"));
}
