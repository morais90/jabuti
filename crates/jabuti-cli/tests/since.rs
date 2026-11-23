mod common;

use common::{
    append, binary, commit, error_on_long_functions, function_of, git, jabuti, project, repository,
    write,
};
use predicates::prelude::PredicateBooleanExt;
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
fn touching_a_unit_already_over_its_limit_without_growing_it_reports_nothing() {
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
        .success()
        .stdout(contains("function-lines").not());
}

#[test]
fn a_unit_over_its_limit_that_the_change_made_longer_is_reported() {
    let base = function_of("long", 70);
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        ("src/live.rs", &base),
    ]);

    write(&directory, "src/live.rs", &function_of("long", 71));

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
        &base.replacen(
            "fn long() {",
            "fn long(a: u8, b: u8, c: u8, d: u8, e: u8) {",
            1,
        ),
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .stdout(contains("src/live.rs:2  warning  parameters  long"))
        .stdout(contains("function-lines").not());
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

#[test]
fn a_renamed_file_is_compared_with_the_file_it_came_from() {
    let base = function_of("long", 70);
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        ("src/old.rs", &base),
    ]);
    git(&directory, &["mv", "src/old.rs", "src/new.rs"]);
    write(
        &directory,
        "src/new.rs",
        &base.replacen("    let value = 1;\n", "    let value = 2;\n", 1),
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("function-lines").not());
}

#[test]
fn how_often_a_file_changed_is_not_compared_with_itself() {
    let directory = repository(&[
        (
            "jabuti.toml",
            "[rules.churn]\nseverity = \"error\"\nlimit = 0\n",
        ),
        ("src/live.rs", "fn live() {}\n"),
    ]);
    write(&directory, "src/live.rs", "fn live() {}\nfn more() {}\n");

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .code(1)
        .stdout(contains("src/live.rs:1  error  churn"));
}

#[test]
fn on_the_principal_branch_the_change_is_what_is_not_committed_yet() {
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        ("src/legacy.rs", &function_of("legacy", 70)),
    ]);
    write(&directory, "src/fresh.rs", &function_of("fresh", 70));

    jabuti(&directory)
        .assert()
        .code(1)
        .stdout(contains("src/fresh.rs:1  error  function-lines  fresh"))
        .stdout(contains("function-lines  legacy").not());
}

#[test]
fn on_another_branch_the_change_is_everything_since_it_left_the_principal_one() {
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        ("src/legacy.rs", &function_of("legacy", 70)),
    ]);
    git(&directory, &["switch", "-q", "-c", "feature"]);
    write(&directory, "src/fresh.rs", &function_of("fresh", 70));
    commit(&directory, "fresh");

    jabuti(&directory)
        .assert()
        .code(1)
        .stdout(contains("src/fresh.rs:1  error  function-lines  fresh"))
        .stdout(contains("function-lines  legacy").not());
}

#[test]
fn the_principal_branch_is_the_one_the_remote_names() {
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        ("src/legacy.rs", &function_of("legacy", 70)),
    ]);
    git(&directory, &["branch", "-q", "-m", "main", "trunk"]);
    git(
        &directory,
        &["update-ref", "refs/remotes/origin/trunk", "HEAD"],
    );
    git(
        &directory,
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/trunk",
        ],
    );
    git(&directory, &["switch", "-q", "-c", "feature"]);
    write(&directory, "src/fresh.rs", &function_of("fresh", 70));
    commit(&directory, "fresh");

    jabuti(&directory)
        .args(["--format", "human"])
        .assert()
        .code(1)
        .stdout(contains("since origin/trunk"))
        .stdout(contains(":1  fresh  function-lines"))
        .stdout(contains("legacy.rs  ").not());
}

#[test]
fn a_repository_without_commits_judges_every_file() {
    let directory = project(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        ("src/legacy.rs", &function_of("legacy", 70)),
    ]);
    git(&directory, &["init", "-q", "-b", "main"]);

    jabuti(&directory)
        .assert()
        .code(1)
        .stdout(contains("src/legacy.rs:1  error  function-lines  legacy"));
}

#[test]
fn the_whole_repository_is_listed_with_all_and_fails_nothing() {
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        ("src/legacy.rs", &function_of("legacy", 70)),
    ]);

    jabuti(&directory)
        .arg("--all")
        .assert()
        .success()
        .stdout(contains("src/legacy.rs:1  error  function-lines  legacy"));
}

#[test]
fn all_and_since_cannot_be_asked_together() {
    let directory = repository(&[("src/lib.rs", "fn small() {}\n")]);

    jabuti(&directory)
        .args(["--all", "--since", "HEAD"])
        .assert()
        .code(2)
        .stderr(contains("cannot be used with"));
}

#[test]
fn on_the_principal_branch_a_commit_not_pushed_yet_is_no_longer_the_change() {
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        ("src/lib.rs", "fn small() {}\n"),
    ]);
    git(
        &directory,
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    );
    git(
        &directory,
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/main",
        ],
    );
    write(&directory, "src/fresh.rs", &function_of("fresh", 70));
    commit(&directory, "fresh");

    jabuti(&directory)
        .args(["--format", "human"])
        .assert()
        .success()
        .stdout(contains("since HEAD"))
        .stdout(contains("fresh").not());
}

#[test]
fn a_detached_head_with_no_principal_branch_asks_for_a_reference_instead_of_judging_nothing() {
    let directory = repository(&[("src/lib.rs", "fn small() {}\n")]);
    git(&directory, &["branch", "-q", "-m", "main", "work"]);
    git(&directory, &["switch", "-q", "--detach"]);

    jabuti(&directory)
        .assert()
        .code(2)
        .stderr(contains("HEAD is detached"))
        .stderr(contains("--since"));
}

#[test]
fn a_detached_head_is_judged_against_the_principal_branch() {
    let directory = repository(&[
        ("jabuti.toml", &error_on_long_functions(60)),
        ("src/lib.rs", "fn small() {}\n"),
    ]);
    git(&directory, &["switch", "-q", "-c", "feature"]);
    write(&directory, "src/fresh.rs", &function_of("fresh", 70));
    commit(&directory, "fresh");
    git(&directory, &["switch", "-q", "--detach"]);

    jabuti(&directory)
        .assert()
        .code(1)
        .stdout(contains("src/fresh.rs:1  error  function-lines  fresh"));
}

#[test]
fn a_principal_branch_sharing_no_history_says_where_the_change_was_worked_out_from() {
    let directory = repository(&[("src/lib.rs", "fn small() {}\n")]);
    git(&directory, &["switch", "-q", "--orphan", "elsewhere"]);
    write(&directory, "src/other.rs", "fn other() {}\n");
    commit(&directory, "unrelated");

    jabuti(&directory)
        .assert()
        .code(2)
        .stderr(contains("the change was worked out from main"));
}
