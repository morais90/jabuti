mod common;

use common::{commit, jabuti, repository, write};
use predicates::str::contains;

const LAYERS: &str = "[layers]\n\
    domain = { paths = [\"src/domain/**\"], depends_on = [] }\n\
    infrastructure = { paths = [\"src/infrastructure/**\"], depends_on = [\"domain\"] }\n";

fn layered(book: &str) -> tempfile::TempDir {
    repository(&[
        ("jabuti.toml", LAYERS),
        (
            "src/main.rs",
            "mod domain;\nmod infrastructure;\n\nfn main() {}\n",
        ),
        ("src/domain/mod.rs", "pub mod book;\n"),
        ("src/domain/book.rs", book),
        ("src/infrastructure/mod.rs", "pub mod db;\n"),
        (
            "src/infrastructure/db.rs",
            "use crate::domain::book::Book;\n\npub fn save(_: Book) {}\n",
        ),
    ])
}

const CLEAN_BOOK: &str = "pub struct Book;\n";
const LEAKING_BOOK: &str =
    "pub struct Book;\n\npub fn persist() {\n    crate::infrastructure::db::save(Book);\n}\n";

#[test]
fn a_dependency_in_the_allowed_direction_is_not_reported() {
    let directory = layered(CLEAN_BOOK);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_dependency_a_layer_may_not_have_is_reported_at_the_line_that_makes_it() {
    let directory = layered(LEAKING_BOOK);

    jabuti(&directory).assert().success().stdout(contains(
        "src/domain/book.rs:4  warning  layer-violation  domain may not depend on infrastructure (src/infrastructure/db.rs)",
    ));
}

#[test]
fn a_violation_that_already_existed_stays_quiet_under_since_until_its_line_is_touched() {
    let directory = layered(LEAKING_BOOK);
    write(
        &directory,
        "src/main.rs",
        "mod domain;\nmod infrastructure;\n\nfn main() {\n    println!(\"hi\");\n}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_violation_this_change_wrote_is_reported_under_since() {
    let directory = layered(CLEAN_BOOK);
    write(&directory, "src/domain/book.rs", LEAKING_BOOK);

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("layer-violation"));
}

#[test]
fn a_layer_that_matches_no_file_is_announced_rather_than_silently_empty() {
    let directory = layered(CLEAN_BOOK);
    write(
        &directory,
        "jabuti.toml",
        "[layers]\ndomain = { paths = [\"src/domain\"], depends_on = [] }\n",
    );

    jabuti(&directory).assert().success().stderr(contains(
        "layer domain matches no file, so nothing is checked against it",
    ));
}

#[test]
fn a_layer_that_depends_on_an_undeclared_one_is_a_configuration_error() {
    let directory = layered(CLEAN_BOOK);
    write(
        &directory,
        "jabuti.toml",
        "[layers]\ndomain = { paths = [\"src/domain/**\"], depends_on = [\"storage\"] }\n",
    );

    jabuti(&directory).assert().code(2).stderr(contains(
        "layer domain depends on storage, which is not a declared layer",
    ));
}

#[test]
fn switching_the_rule_off_keeps_the_layers_but_reports_nothing() {
    let directory = layered(LEAKING_BOOK);
    write(
        &directory,
        "jabuti.toml",
        &format!("{LAYERS}\n[rules]\nlayer-violation = {{ severity = \"off\" }}\n"),
    );
    commit(&directory, "switch off");

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_violation_on_an_untouched_line_of_a_changed_file_stays_quiet_under_since() {
    let directory = layered(LEAKING_BOOK);
    write(
        &directory,
        "src/domain/book.rs",
        &format!("{LEAKING_BOOK}\npub fn title() -> &'static str {{\n    \"untitled\"\n}}\n"),
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_file_matched_by_two_layers_is_a_configuration_error_naming_both() {
    let directory = layered(CLEAN_BOOK);
    write(
        &directory,
        "jabuti.toml",
        "[layers]\n\
         domain = { paths = [\"src/domain/**\"], depends_on = [] }\n\
         everything = { paths = [\"src/**\"], depends_on = [] }\n",
    );

    jabuti(&directory).assert().code(2).stderr(contains(
        "src/domain/book.rs is in both the domain and the everything layer",
    ));
}

#[test]
fn layer_paths_are_relative_to_the_project_whatever_root_the_command_names() {
    let directory = layered(LEAKING_BOOK);
    let absolute = directory.path().canonicalize().expect("the project exists");

    common::binary(&directory)
        .arg("check")
        .arg(&absolute)
        .assert()
        .success()
        .stdout(contains("layer-violation"))
        .stderr("");
}

#[test]
fn an_unchanged_file_the_index_could_not_read_is_named_so_its_missing_edges_are_visible() {
    let directory = repository(&[
        (
            "jabuti.toml",
            "[layers]\n\
             domain = { paths = [\"src/domain/**\"], depends_on = [] }\n\
             infrastructure = { paths = [\"src/infrastructure/**\"], depends_on = [] }\n",
        ),
        (
            "src/domain/Book.kt",
            "package org.example.domain\n\nclass Book\n",
        ),
        (
            "src/infrastructure/Db.kt",
            "package org.example.infrastructure\n\nclass Db(val name: String = [])\n",
        ),
    ]);
    write(
        &directory,
        "src/domain/Book.kt",
        "package org.example.domain\n\nimport org.example.infrastructure.Db\n\nclass Book(val db: Db)\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("src/infrastructure/Db.kt  unreadable syntax"));
}

#[test]
fn a_changed_file_both_the_scan_and_the_index_failed_on_is_named_once() {
    let directory = repository(&[
        (
            "jabuti.toml",
            "[layers]\ndomain = { paths = [\"src/domain/**\"], depends_on = [] }\n",
        ),
        (
            "src/domain/Book.kt",
            "package org.example.domain\n\nclass Book\n",
        ),
    ]);
    write(
        &directory,
        "src/domain/Book.kt",
        "package org.example.domain\n\nclass Book(val tags: Array<String> = [])\n",
    );

    let output = jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let rendered = String::from_utf8(output).expect("utf8");

    assert_eq!(
        rendered.matches("src/domain/Book.kt  unreadable").count(),
        1,
        "{rendered}"
    );
    assert!(rendered.contains("1 file was not measured"), "{rendered}");
}
