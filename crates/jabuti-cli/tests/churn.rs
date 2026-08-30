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
fn asking_for_churn_outside_a_repository_says_so_rather_than_failing() {
    let directory = project(&[
        ("jabuti.toml", ONLY_CHURN),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stderr(contains("need a git repository"));
}

#[test]
fn history_is_not_read_when_no_rule_asks_for_it() {
    let directory = project(&[
        (
            "jabuti.toml",
            "[rules]\nchurn = { severity = \"off\" }\nhotspot = { severity = \"off\" }\n",
        ),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"))
        .stderr("");
}

const ONLY_HOTSPOT: &str = "[rules]\n\
hotspot = { limit = 50, severity = \"warning\" }\n\
churn = { severity = \"off\" }\n\
function-lines = { severity = \"off\" }\n\
cognitive-complexity = { severity = \"off\" }\n\
parameters = { severity = \"off\" }\n";

fn tangled(name: &str) -> String {
    format!(
        "fn {name}(a: bool, b: bool) {{\n    if a {{\n        if b {{\n            if a && b {{\n                let _ = 1;\n            }}\n        }}\n    }}\n}}\n"
    )
}

fn repository_with_a_hotspot() -> tempfile::TempDir {
    let directory = repository(&[
        ("jabuti.toml", ONLY_HOTSPOT),
        ("src/calm.rs", "fn calm() {}\n"),
        (
            "src/middling.rs",
            "fn middling(a: bool) {\n    if a {}\n}\n",
        ),
        ("src/tangled.rs", &tangled("tangled")),
    ]);

    write(
        &directory,
        "src/middling.rs",
        "fn middling(a: bool) {\n    if a {}\n    \n}\n",
    );
    write(
        &directory,
        "src/tangled.rs",
        &format!("{}\n", tangled("tangled")),
    );
    commit(&directory, "second");
    write(
        &directory,
        "src/tangled.rs",
        &format!("{}\n\n", tangled("tangled")),
    );
    commit(&directory, "third");

    directory
}

#[test]
fn hotspot_is_evaluated_even_when_churn_reporting_is_off() {
    let directory = repository_with_a_hotspot();

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("src/tangled.rs:1  warning  hotspot"))
        .stderr("");
}

#[test]
fn hotspot_is_skipped_with_since_and_says_so() {
    let directory = repository_with_a_hotspot();

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"))
        .stderr(contains("not evaluated with --since"));
}
