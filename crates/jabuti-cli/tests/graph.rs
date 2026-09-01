mod common;

use common::{binary, commit, repository, write};
use predicates::str::contains;

fn helper_project() -> tempfile::TempDir {
    repository(&[
        (
            "src/main.rs",
            "mod git;\nmod since;\n\nfn main() {\n    println!(\"{}\", since::latest());\n}\n",
        ),
        (
            "src/git.rs",
            "pub fn run(arguments: &[&str]) -> String {\n    arguments.join(\" \")\n}\n",
        ),
        (
            "src/since.rs",
            "pub fn latest() -> String {\n    crate::git::run(&[\"log\"])\n}\n",
        ),
    ])
}

#[test]
fn a_file_reached_only_through_an_inline_path_is_still_reported_as_affected() {
    let directory = helper_project();
    write(
        &directory,
        "src/git.rs",
        "pub fn run(arguments: &[&str], quiet: bool) -> String {\n    let _ = quiet;\n    arguments.join(\" \")\n}\n",
    );

    binary(&directory)
        .arg("graph")
        .arg("impact")
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(
            "1 file changed, 2 files reached.\n\
             \n\
             src/git.rs\n\
             \x20 src/main.rs\n\
             \x20 src/since.rs\n",
        );
}

#[test]
fn a_change_nothing_depends_on_says_so_in_one_line() {
    let directory = helper_project();
    write(
        &directory,
        "src/main.rs",
        "mod git;\nmod since;\n\nfn main() {\n    println!(\"{} done\", since::latest());\n}\n",
    );
    commit(&directory, "edit the entry point");

    binary(&directory)
        .arg("graph")
        .arg("impact")
        .arg("--since")
        .arg("HEAD~1")
        .assert()
        .success()
        .stdout("1 file changed, 0 files reached.\n");
}

#[test]
fn a_file_the_graph_could_not_read_is_named_so_a_missing_edge_is_visible() {
    let directory = repository(&[
        (
            "src/main.rs",
            "mod git;\n\nfn main() {\n    let latest = git::run(&[\"log\"]);\n    println!(\"{latest}\");\n}\n",
        ),
        (
            "src/git.rs",
            "pub fn run(arguments: &[&str]) -> String {\n    arguments.join(\" \")\n}\n",
        ),
        ("src/broken.rs", "fn truncated() {\n"),
    ]);
    write(
        &directory,
        "src/git.rs",
        "pub fn run(arguments: &[&str], quiet: bool) -> String {\n    let _ = quiet;\n    arguments.join(\" \")\n}\n",
    );

    binary(&directory)
        .arg("graph")
        .arg("impact")
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("1 file changed, 1 file reached."))
        .stdout(contains(
            "1 file was not measured, so nothing above accounts for it.",
        ))
        .stdout(contains("src/broken.rs  unreadable syntax from line 1"));
}

#[test]
fn a_change_with_more_dependents_than_the_limit_says_how_many_it_withheld() {
    let directory = repository(&[
        (
            "src/main.rs",
            "mod git;\nmod one;\nmod two;\nmod three;\n\nfn main() {\n    let _ = (one::go(), two::go(), three::go());\n}\n",
        ),
        (
            "src/git.rs",
            "pub fn run(arguments: &[&str]) -> String {\n    arguments.join(\" \")\n}\n",
        ),
        (
            "src/one.rs",
            "pub fn go() -> String {\n    crate::git::run(&[\"one\"])\n}\n",
        ),
        (
            "src/two.rs",
            "pub fn go() -> String {\n    crate::git::run(&[\"two\"])\n}\n",
        ),
        (
            "src/three.rs",
            "pub fn go() -> String {\n    crate::git::run(&[\"three\"])\n}\n",
        ),
    ]);
    write(
        &directory,
        "src/git.rs",
        "pub fn run(arguments: &[&str], quiet: bool) -> String {\n    let _ = quiet;\n    arguments.join(\" \")\n}\n",
    );

    binary(&directory)
        .arg("graph")
        .arg("impact")
        .arg("--since")
        .arg("HEAD")
        .arg("--limit")
        .arg("2")
        .assert()
        .success()
        .stdout(contains("1 file changed, 4 files reached."))
        .stdout(contains("2 further files not shown."));
}

#[test]
fn the_answer_is_byte_identical_whatever_the_thread_count() {
    let directory = helper_project();
    write(
        &directory,
        "src/git.rs",
        "pub fn run(arguments: &[&str], quiet: bool) -> String {\n    let _ = quiet;\n    arguments.join(\" \")\n}\n",
    );

    let single = binary(&directory)
        .env("RAYON_NUM_THREADS", "1")
        .arg("graph")
        .arg("impact")
        .arg("--since")
        .arg("HEAD")
        .output()
        .expect("the binary runs");

    let many = binary(&directory)
        .env("RAYON_NUM_THREADS", "8")
        .arg("graph")
        .arg("impact")
        .arg("--since")
        .arg("HEAD")
        .output()
        .expect("the binary runs");

    assert_eq!(single.stdout, many.stdout);
    assert!(!single.stdout.is_empty());
}
