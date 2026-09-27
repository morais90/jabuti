mod common;

use common::{
    append, binary, commit, error_on_long_functions, function_of, git, jabuti, repository, write,
};
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
        .stdout(contains("No findings"));
}

#[test]
fn unchanged_files_are_skipped_entirely_when_nothing_needs_the_whole_repository() {
    let directory = repository(&[
        (
            "jabuti.toml",
            "[rules]\nduplicate-block = { severity = \"off\" }\nhotspot = { severity = \"off\" }\n",
        ),
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

fn a_function_appended_to_is_reported(name: &str) {
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        (name, "fn small() {}\n"),
    ]);

    append(&directory, name, &format!("\n{}", function_of("added", 70)));

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .code(1)
        .stdout(contains(format!("{name}:3  error  function-lines  added")));
}

#[test]
fn a_unit_overlapping_a_changed_line_is_reported() {
    a_function_appended_to_is_reported("src/live.rs");
}

#[test]
fn a_changed_file_named_with_an_accent_stays_in_the_change() {
    a_function_appended_to_is_reported("src/ação.rs");
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

fn a_new_file_is_reported_whole(name: &str) {
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        ("src/live.rs", "fn small() {}\n"),
    ]);

    write(&directory, name, &function_of("fresh", 70));

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .code(1)
        .stdout(contains(format!("{name}:1  error  function-lines  fresh")));
}

#[test]
fn every_unit_of_a_brand_new_file_counts_as_changed() {
    a_new_file_is_reported_whole("src/fresh.rs");
}

#[test]
fn a_new_file_named_with_an_accent_stays_in_the_change() {
    a_new_file_is_reported_whole("src/ação.rs");
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
fn a_changed_file_outside_the_paths_given_on_the_command_line_is_not_reviewed() {
    let directory = repository(&[
        ("src/inside/lib.rs", "fn small() {}\n"),
        ("src/outside/lib.rs", "fn tiny() {}\n"),
    ]);
    write(&directory, "src/inside/lib.rs", "fn small() { }\n");
    write(&directory, "src/outside/lib.rs", "fn tiny() { }\n");

    binary(&directory)
        .arg("check")
        .arg("src/inside")
        .arg("--since")
        .arg("HEAD")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(contains("\"files\": 1,"));
}

#[test]
fn a_reference_with_more_than_one_merge_base_stops_the_run_rather_than_picking_one() {
    let directory = repository(&[("src/lib.rs", "fn small() {}\n")]);
    git(&directory, &["checkout", "-q", "-b", "left"]);
    write(&directory, "src/left.rs", "fn left() {}\n");
    commit(&directory, "left");
    git(&directory, &["checkout", "-q", "main"]);
    git(&directory, &["checkout", "-q", "-b", "right"]);
    write(&directory, "src/right.rs", "fn right() {}\n");
    commit(&directory, "right");
    git(&directory, &["checkout", "-q", "left"]);
    git(
        &directory,
        &["merge", "-q", "-m", "left takes right", "right"],
    );
    git(&directory, &["checkout", "-q", "right"]);
    git(
        &directory,
        &["merge", "-q", "-m", "right takes left", "left~1"],
    );

    jabuti(&directory)
        .arg("--since")
        .arg("left")
        .assert()
        .code(2)
        .stderr(contains("more than one merge base"));
}

#[test]
fn an_unknown_reference_stops_the_run_rather_than_passing_the_gate() {
    let directory = repository(&[("src/live.rs", "fn small() {}\n")]);

    jabuti(&directory)
        .arg("--since")
        .arg("no-such-branch")
        .assert()
        .code(2)
        .stderr(contains("git merge-base --all HEAD no-such-branch failed"));
}
