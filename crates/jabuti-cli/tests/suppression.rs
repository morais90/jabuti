mod common;

use common::{jabuti, project};
use predicates::str::contains;

#[test]
fn a_rust_allow_names_the_lint_it_waives() {
    let source = "#[allow(dead_code)]\nfn unused() {}\n";
    let directory = project(&[("src/live.rs", source)]);

    jabuti(&directory).assert().code(1).stdout(contains(
        "src/live.rs:1  suppression  dead_code  a linter diagnostic is suppressed instead of satisfied",
    ));
}

#[test]
fn a_rust_crate_level_allow_is_reported_too() {
    let source = "#![allow(clippy::all)]\n\nfn live() {}\n";
    let directory = project(&[("src/live.rs", source)]);

    jabuti(&directory)
        .assert()
        .code(1)
        .stdout(contains("src/live.rs:1  suppression  clippy::all"));
}

#[test]
fn the_same_construct_in_a_test_marked_function_is_still_reported() {
    let source = "#[test]\nfn checks() {\n    #[allow(dead_code)]\n    fn unused() {}\n}\n";
    let directory = project(&[("src/live.rs", source)]);

    jabuti(&directory)
        .assert()
        .code(1)
        .stdout(contains("src/live.rs:3  suppression  dead_code"));
}

#[test]
fn a_kotlin_suppress_names_the_argument_it_carries() {
    let source = "@Suppress(\"UNCHECKED_CAST\")\nfun unused() {}\n";
    let directory = project(&[("src/Live.kt", source)]);

    jabuti(&directory).assert().code(1).stdout(contains(
        "src/Live.kt:1  suppression  \"UNCHECKED_CAST\"  a linter diagnostic is suppressed instead of satisfied",
    ));
}

#[test]
fn a_typescript_ts_ignore_comment_is_reported_at_its_own_line() {
    let source = "// @ts-ignore\nconst legacy = read();\n";
    let directory = project(&[("src/live.ts", source)]);

    jabuti(&directory)
        .assert()
        .code(1)
        .stdout(contains("src/live.ts:1  suppression  ts-ignore"));
}

#[test]
fn a_typescript_eslint_disable_comment_names_the_rule_it_silences() {
    let source = "// eslint-disable-next-line no-console\nconsole.log(1);\n";
    let directory = project(&[("src/live.ts", source)]);

    jabuti(&directory)
        .assert()
        .code(1)
        .stdout(contains("src/live.ts:1  suppression  no-console"));
}

#[test]
fn a_typescript_any_assertion_names_the_type_checker_rather_than_the_linter() {
    let source = "function widen(value: unknown): unknown {\n    return value as any;\n}\n";
    let directory = project(&[("src/live.ts", source)]);

    jabuti(&directory).assert().code(1).stdout(contains(
        "src/live.ts:2  suppression  any  a type-checker diagnostic is suppressed instead of satisfied",
    ));
}

#[test]
fn a_configured_typescript_helper_extends_the_built_in_vocabulary() {
    let settings = "[languages.typescript.concepts]\nsuppression = [\"@mycorp/types.castAny\"]\n";
    let source = "import { castAny } from \"@mycorp/types\";\nexport function live(value: unknown): unknown {\n    return castAny(value);\n}\n";
    let directory = project(&[("jabuti.toml", settings), ("src/live.ts", source)]);

    jabuti(&directory).assert().code(1).stdout(contains(
        "src/live.ts:3  suppression  castAny  a linter diagnostic is suppressed instead of satisfied",
    ));
}
