mod common;

use common::{commit, jabuti, project, repository, write};
use predicates::str::contains;

const ONLY_CHURN: &str = "[rules]\n\
churn = { limit = 1, severity = \"error\" }\n\
function-lines = { severity = \"off\" }\n\
cognitive-complexity = { severity = \"off\" }\n\
parameters = { severity = \"off\" }\n";

#[test]
fn churn_counts_every_commit_that_touched_the_file() {
    let directory = repository(&[
        ("jabuti.toml", ONLY_CHURN),
        ("src/busy.rs", "fn busy() {}\n"),
    ]);

    write(&directory, "src/busy.rs", "fn busy() { }\n");
    commit(&directory, "second");
    write(&directory, "src/busy.rs", "fn busy() {  }\n");
    commit(&directory, "third");

    jabuti(&directory)
        .assert()
        .code(1)
        .stdout(contains("src/busy.rs:1  error  churn  measured 3, limit 1"));
}

#[test]
fn a_file_committed_once_stays_within_a_limit_of_one() {
    let directory = repository(&[
        ("jabuti.toml", ONLY_CHURN),
        ("src/stable.rs", "fn stable() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn churn_is_reported_against_the_file_and_not_its_functions() {
    let directory = repository(&[
        ("jabuti.toml", ONLY_CHURN),
        ("src/busy.rs", "fn one() {}\nfn two() {}\n"),
    ]);

    write(
        &directory,
        "src/busy.rs",
        "fn one() {}\nfn two() {}\nfn three() {}\n",
    );
    commit(&directory, "second");

    jabuti(&directory)
        .assert()
        .code(1)
        .stdout(contains("1 error and 0 warnings"));
}

#[test]
fn asking_for_churn_outside_a_repository_stops_the_run() {
    let directory = project(&[
        ("jabuti.toml", ONLY_CHURN),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory).assert().code(2).stderr(contains("git"));
}

#[test]
fn history_is_not_read_when_no_rule_asks_for_it() {
    let directory = project(&[("src/lib.rs", "fn small() {}\n")]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"))
        .stderr("");
}
