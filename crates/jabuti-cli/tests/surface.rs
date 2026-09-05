mod common;

use common::{commit, jabuti, repository, stage, write};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

fn binary_crate() -> tempfile::TempDir {
    repository(&[
        (
            "src/main.rs",
            "mod tasks;\n\nfn main() {\n    tasks::run();\n}\n",
        ),
        ("src/tasks.rs", "pub fn run() {}\n"),
    ])
}

fn library_crate() -> tempfile::TempDir {
    repository(&[
        ("src/lib.rs", "pub mod api;\nmod internal;\n"),
        ("src/api.rs", "pub fn serve() -> u32 {\n    1\n}\n"),
        ("src/internal.rs", "pub fn helper() -> u32 {\n    2\n}\n"),
    ])
}

#[test]
fn a_new_public_function_nothing_references_is_reported_where_it_was_declared() {
    let directory = binary_crate();
    write(
        &directory,
        "src/tasks.rs",
        "pub fn run() {}\n\npub fn pause() {}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains(
            "src/tasks.rs:3  warning  speculative-api  pause  public, and nothing references it",
        ));
}

#[test]
fn a_new_public_function_another_file_calls_is_not_speculative() {
    let directory = binary_crate();
    write(
        &directory,
        "src/tasks.rs",
        "pub fn run() {\n    pause();\n}\n\npub fn pause() {}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_public_function_that_already_existed_is_not_reported_when_its_file_changes() {
    let directory = binary_crate();
    write(
        &directory,
        "src/tasks.rs",
        "pub fn run() {}\n\npub fn pause() {}\n",
    );
    commit(&directory, "add pause");
    write(
        &directory,
        "src/tasks.rs",
        "pub fn run() {\n    println!(\"running\");\n}\n\npub fn pause() {}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_new_item_in_a_public_module_of_a_library_is_the_library_surface_and_not_speculative() {
    let directory = library_crate();
    write(
        &directory,
        "src/api.rs",
        "pub fn serve() -> u32 {\n    1\n}\n\npub fn spare() -> u32 {\n    0\n}\n",
    );
    write(
        &directory,
        "src/internal.rs",
        "pub fn helper() -> u32 {\n    2\n}\n\npub fn stray() -> u32 {\n    3\n}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains(
            "src/internal.rs:5  warning  speculative-api  stray  public, and nothing references it",
        ))
        .stdout(contains("spare").not());
}

#[test]
fn an_item_carrying_an_attribute_we_cannot_follow_is_unknown_rather_than_speculative() {
    let directory = binary_crate();
    write(
        &directory,
        "src/tasks.rs",
        "pub fn run() {}\n\n#[no_mangle]\npub extern \"C\" fn entry() {}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_new_kotlin_class_nothing_references_is_reported_and_an_annotated_one_is_not() {
    let directory = repository(&[(
        "src/main/kotlin/shop/Shelf.kt",
        "package shop\n\nclass Shelf\n\nfun main() {\n    Shelf()\n}\n",
    )]);
    write(
        &directory,
        "src/main/kotlin/shop/Shelf.kt",
        "package shop\n\n@Component\nclass Catalog\n\nclass Basket\n\nclass Shelf\n\nfun main() {\n    Shelf()\n}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains(
            "src/main/kotlin/shop/Shelf.kt:6  warning  speculative-api  Basket  public, and nothing references it",
        ))
        .stdout(contains("Catalog").not());
}

#[test]
fn the_rule_can_be_switched_off() {
    let directory = binary_crate();
    write(
        &directory,
        "jabuti.toml",
        "[rules]\nspeculative-api = { severity = \"off\" }\n",
    );
    write(
        &directory,
        "src/tasks.rs",
        "pub fn run() {}\n\npub fn pause() {}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn without_since_the_rule_has_nothing_to_compare_against_and_says_so_when_it_gates() {
    let directory = binary_crate();
    write(
        &directory,
        "jabuti.toml",
        "[rules]\nspeculative-api = { severity = \"error\" }\n",
    );
    write(
        &directory,
        "src/tasks.rs",
        "pub fn run() {}\n\npub fn pause() {}\n",
    );

    jabuti(&directory).assert().success().stderr(contains(
        "speculative-api compares against an earlier revision, so it needs --since",
    ));
}

#[test]
fn a_name_mentioned_only_in_a_file_we_could_not_read_is_unknown_rather_than_speculative() {
    let directory = binary_crate();
    write(
        &directory,
        "src/broken.rs",
        "fn truncated() {\n    pause();\n",
    );
    write(
        &directory,
        "src/tasks.rs",
        "pub fn run() {}\n\npub fn pause() {}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("speculative-api").not())
        .stdout(contains("src/broken.rs  unreadable syntax from line 1"));
}

#[test]
fn a_public_item_under_a_test_directory_belongs_to_the_test_runner_and_is_not_reported() {
    let directory = binary_crate();
    write(
        &directory,
        "tests/smoke.rs",
        "pub fn fixture() -> u32 {\n    1\n}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_caller_outside_the_paths_given_on_the_command_line_still_counts() {
    let directory = binary_crate();
    write(
        &directory,
        "src/main.rs",
        "mod tasks;\n\nfn main() {\n    tasks::run();\n    tasks::pause();\n}\n",
    );
    write(
        &directory,
        "src/tasks.rs",
        "pub fn run() {}\n\npub fn pause() {}\n",
    );

    jabuti(&directory)
        .arg("src/tasks.rs")
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_language_can_switch_the_rule_on_while_the_shared_setting_is_off() {
    let directory = binary_crate();
    write(
        &directory,
        "jabuti.toml",
        "[rules]\nspeculative-api = { severity = \"off\" }\n\n[languages.rust.rules]\nspeculative-api = { severity = \"warning\" }\n",
    );
    write(
        &directory,
        "src/tasks.rs",
        "pub fn run() {}\n\npub fn pause() {}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("speculative-api  pause"));
}

#[test]
fn a_file_whose_earlier_revision_does_not_parse_is_skipped_rather_than_read_as_new() {
    let directory = binary_crate();
    write(
        &directory,
        "src/tasks.rs",
        "pub fn run() {}\n\npub fn old_orphan() {}\n\nfn broken( {\n",
    );
    commit(&directory, "broken base");
    write(
        &directory,
        "src/tasks.rs",
        "pub fn run() {}\n\npub fn old_orphan() {}\n\nfn fixed() {}\n",
    );

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("speculative-api").not());
}

#[test]
fn a_change_that_is_already_staged_is_still_measured_against_the_revision() {
    let directory = binary_crate();
    write(
        &directory,
        "src/tasks.rs",
        "pub fn run() {}\n\npub fn pause() {}\n",
    );
    stage(&directory);

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("speculative-api  pause"));
}
