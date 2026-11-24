mod common;

use common::{function_of, git, jabuti, project, repository, write};
use predicates::str::contains;

#[test]
fn a_tree_within_the_limits_reports_nothing_and_passes() {
    let directory = project(&[("src/lib.rs", "fn small() {}\n")]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout("No findings across 1 file and 1 unit.\n");
}

#[test]
fn a_finding_configured_as_an_error_fails_the_gate() {
    let directory = project(&[("src/lib.rs", &function_of("wide", 70))]);

    jabuti(&directory)
        .assert()
        .code(1)
        .stdout(contains(
            "src/lib.rs:1  function-lines  wide  measured 72, limit 60",
        ))
        .stdout(contains("1 finding to fix across 1 file and 1 unit."));
}

#[test]
fn every_unit_in_the_tree_is_counted_not_only_the_top_level_ones() {
    let directory = project(&[(
        "src/lib.rs",
        "struct Holder;\n\nimpl Holder {\n    fn method() {\n        let closure = || 1;\n    }\n}\n",
    )]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("1 file and 4 units"));
}

#[test]
fn a_file_that_does_not_parse_is_named_in_the_output_and_left_out_of_the_count() {
    let directory = repository(&[
        ("src/lib.rs", "fn small() {}\n"),
        ("src/broken.rs", "fn truncated() {\n"),
    ]);

    jabuti(&directory)
        .arg("--all")
        .assert()
        .success()
        .stdout(contains("1 file and 1 unit"))
        .stdout(contains(
            "1 file was not measured, so nothing above accounts for it.",
        ))
        .stdout(contains("src/broken.rs  unreadable syntax from line 1"))
        .stderr("");
}

#[test]
fn an_excluded_path_is_not_analysed_at_all() {
    let directory = project(&[
        ("jabuti.toml", "exclude = [\"generated/**\"]\n"),
        ("src/lib.rs", "fn small() {}\n"),
        ("generated/tables.rs", &function_of("huge", 400)),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout("No findings across 1 file and 1 unit.\n");
}

#[test]
fn a_file_of_another_language_is_not_analysed() {
    let directory = project(&[
        ("src/lib.rs", "fn small() {}\n"),
        ("README.md", "fn this_is_prose_not_code() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout("No findings across 1 file and 1 unit.\n");
}

#[test]
fn a_rules_table_stops_the_run_and_says_why() {
    let directory = project(&[
        ("jabuti.toml", "[rules]\nfunction-lines = { limit = 80 }\n"),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory).assert().code(2).stderr(
        "jabuti: [rules] is not read: jabuti fixes which rules run and their limits, and every finding it reports is one to fix, so there is nothing to set there. Remove the table\n",
    );
}

#[test]
fn a_rules_table_for_one_language_stops_the_run_too() {
    let directory = project(&[
        (
            "jabuti.toml",
            "[languages.rust.rules]\nduplicate-block = { severity = \"off\" }\n",
        ),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .code(2)
        .stderr(contains("[languages.rust.rules] is not read: jabuti fixes"));
}

#[test]
fn findings_are_reported_in_path_then_line_order() {
    let directory = project(&[
        ("src/b.rs", &function_of("second", 70)),
        ("src/a.rs", &function_of("first", 70)),
    ]);

    let output = jabuti(&directory)
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let rendered = String::from_utf8(output).expect("utf8");
    let first = rendered.find("src/a.rs").expect("a is reported");
    let second = rendered.find("src/b.rs").expect("b is reported");

    assert!(first < second, "{rendered}");
}

#[test]
fn a_kotlin_file_is_measured_against_kotlins_own_limit() {
    let body = "    val value = 1\n".repeat(50);
    let directory = project(&[("src/Main.kt", &format!("fun wide() {{\n{body}}}\n"))]);

    jabuti(&directory).assert().code(1).stdout(contains(
        "src/Main.kt:1  function-lines  wide  measured 52, limit 47",
    ));
}

#[test]
fn a_typescript_file_is_measured_against_typescripts_own_limit() {
    let body = "    consume(value);\n".repeat(71);
    let source = format!("export function wide(value: number): void {{\n{body}}}\n");
    let directory = project(&[("src/live.ts", &source)]);

    jabuti(&directory).assert().code(1).stdout(contains(
        "src/live.ts:1  function-lines  wide  measured 73, limit 71",
    ));
}

#[test]
fn a_language_nobody_supports_stops_the_run() {
    let directory = project(&[
        ("jabuti.toml", "[languages.cobol]\n"),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .code(2)
        .stderr(contains("unknown language cobol"));
}

#[test]
fn the_json_format_names_the_rule_the_way_configuration_does() {
    let directory = project(&[(
        "src/lib.rs",
        "fn wide(a: i32, b: i32, c: i32, d: i32, e: i32) -> i32 {\n    a + b + c + d + e\n}\n",
    )]);

    jabuti(&directory)
        .arg("--format")
        .arg("json")
        .assert()
        .code(1)
        .stdout(contains("\"schema\": 5"))
        .stdout(contains("\"rule\": \"parameters\""))
        .stdout(contains("\"findings\": 1,"));
}

#[test]
fn the_sarif_format_keeps_the_gate_result_when_the_agent_limit_is_zero() {
    let directory = project(&[("src/lib.rs", &function_of("wide", 70))]);

    let output = jabuti(&directory)
        .arg("--format")
        .arg("sarif")
        .arg("--limit")
        .arg("0")
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let document: serde_json::Value = serde_json::from_slice(&output).expect("SARIF is valid JSON");

    assert_eq!(document["version"], "2.1.0");
    assert_eq!(
        document["runs"][0]["results"],
        serde_json::json!([
            {
                "ruleId": "function-lines",
                "ruleIndex": 0,
                "level": "error",
                "message": { "text": "wide measured 72, limit 60" },
                "locations": [
                    {
                        "physicalLocation": {
                            "artifactLocation": { "uri": "src/lib.rs" },
                            "region": { "startLine": 1, "endLine": 72 }
                        }
                    }
                ]
            }
        ])
    );
    assert_eq!(
        document["runs"][0]["invocations"][0]["executionSuccessful"],
        true
    );
}

#[test]
fn the_measures_format_reports_a_rule_that_is_switched_off() {
    let directory = project(&[(
        "src/lib.rs",
        "fn a(v: i32) -> i32 {\n    if v > 0 { 1 } else { 0 }\n}\n",
    )]);

    jabuti(&directory)
        .arg("--format")
        .arg("measures")
        .assert()
        .success()
        .stdout(contains("\"cyclomatic-complexity\": 2"));
}

#[test]
fn source_the_grammar_is_too_old_to_know_is_reported_rather_than_skipped() {
    let directory = repository(&[
        ("src/small.kt", "fun small() {}\n"),
        (
            "src/guard.kt",
            "fun describe(value: Any, active: Boolean): String =\n\
             \x20   when (value) {\n\
             \x20       is String if active -> \"active string\"\n\
             \x20       else -> \"other\"\n\
             \x20   }\n",
        ),
    ]);

    jabuti(&directory)
        .arg("--all")
        .assert()
        .success()
        .stdout(
            "No findings across 1 file and 1 unit.\n\
             \n\
             1 file was not measured, so nothing above accounts for it.\n\
             src/guard.kt  unreadable syntax from line 1\n",
        )
        .stderr("");
}

#[test]
fn a_symlink_to_a_source_file_does_not_replace_the_file_it_points_at() {
    let directory = repository(&[(
        "src/lib.rs",
        "pub fn read() -> usize {\n    let value: Option<usize> = None;\n    value.unwrap()\n}\n",
    )]);
    std::os::unix::fs::symlink("src/lib.rs", directory.path().join("alias.rs"))
        .expect("the alias can be created");

    jabuti(&directory)
        .arg("--all")
        .assert()
        .success()
        .stdout(contains("src/lib.rs:3  error-masking"))
        .stdout(contains("1 file and 1 unit"));
}

#[test]
fn every_rule_has_a_page_explaining_it_in_the_repository() {
    let repository = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    if !repository.join("justfile").is_file() {
        return;
    }

    let pages = repository.join("docs/rules");
    let missing: Vec<&str> = jabuti_core::catalog::Rule::ALL
        .into_iter()
        .map(jabuti_core::catalog::Rule::id)
        .filter(|id| !pages.join(format!("{id}.md")).is_file())
        .collect();

    assert!(
        missing.is_empty(),
        "rules without a page under docs/rules: {missing:?}"
    );
}

#[test]
fn findings_are_listed_by_path_and_line_whichever_rule_produced_them() {
    let directory = repository(&[
        (
            "src/main.rs",
            "mod aa;\nmod git;\nmod zz;\n\nfn main() {}\n",
        ),
        (
            "src/git.rs",
            "pub fn run() -> String {\n    String::new()\n}\n",
        ),
        ("src/aa.rs", "pub fn aa() {}\n"),
        ("src/zz.rs", "fn zz() {}\n"),
    ]);
    write(
        &directory,
        "src/aa.rs",
        "pub fn aa() -> usize {\n    let value: Option<usize> = None;\n    value.unwrap()\n}\n",
    );
    write(&directory, "src/zz.rs", &function_of("zz", 70));

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .code(1)
        .stdout(contains(
            "src/aa.rs:3  error-masking  unwrap  the failure becomes a panic\nsrc/zz.rs:1  function-lines  zz  measured 72, limit 60\n",
        ));
}

#[test]
fn the_human_format_asked_for_through_a_pipe_is_written_without_color() {
    let directory = project(&[("src/lib.rs", &function_of("wide", 70))]);

    jabuti(&directory)
        .args(["--format", "human"])
        .env_remove("NO_COLOR")
        .assert()
        .code(1)
        .stdout(
            "⬢ jabuti  1 file · 1 unit\n\
             \n\
             src/lib.rs\n  \
             :1  wide  function-lines  ━━━━━━━━┃━━╌╌  72/60\n\
             \n\
             1 to fix · seal 980a·7004\n",
        );
}

fn commit_dated(directory: &tempfile::TempDir, date: &str) {
    git(directory, &["add", "-A"]);
    let status = std::process::Command::new("git")
        .args(["-c", "commit.gpgsign=false", "commit", "-qm", date])
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .current_dir(directory.path())
        .status()
        .expect("git runs");
    assert!(status.success(), "git commit at {date}");
}

#[test]
fn the_human_format_draws_each_files_weekly_commits_counted_back_from_head() {
    let directory = project(&[("src/lib.rs", &function_of("wide", 70))]);
    git(&directory, &["init", "-q", "-b", "main"]);
    git(&directory, &["config", "user.email", "test@example.com"]);
    git(&directory, &["config", "user.name", "test"]);
    commit_dated(&directory, "2026-01-01T12:00:00+00:00");
    write(&directory, "src/lib.rs", &function_of("wide", 71));
    write(&directory, "src/a\"[b].rs", &function_of("odd", 70));
    commit_dated(&directory, "2026-01-22T12:00:00+00:00");

    jabuti(&directory)
        .arg("--all")
        .args(["--format", "human"])
        .assert()
        .success()
        .stdout(contains(
            "▁▁▁▁▁▁▁▁█▁▁█  src/lib.rs  2 commits in 12 weeks\n              :1  wide",
        ))
        .stdout(contains(
            "▁▁▁▁▁▁▁▁▁▁▁█  src/a\"[b].rs  1 commit in 12 weeks\n              :1  odd",
        ));
}

#[test]
fn verbose_opens_with_the_commits_of_the_change_and_marks_the_lines_it_touched() {
    let directory = project(&[("src/lib.rs", &function_of("wide", 70))]);
    git(&directory, &["init", "-q", "-b", "main"]);
    git(&directory, &["config", "user.email", "test@example.com"]);
    git(&directory, &["config", "user.name", "test"]);
    commit_dated(&directory, "2026-01-01T12:00:00+00:00");
    git(&directory, &["switch", "-q", "-c", "feature"]);
    let grown = function_of("wide", 70).replacen("{\n", "{\n    let grown = 2;\n", 1);
    write(&directory, "src/lib.rs", &grown);
    git(&directory, &["add", "-A"]);
    git(
        &directory,
        &["-c", "commit.gpgsign=false", "commit", "-qm", "grow wide"],
    );

    let verbose = jabuti(&directory)
        .args(["--since", "main", "--format", "human", "-v"])
        .assert()
        .code(1);
    let output = String::from_utf8_lossy(&verbose.get_output().stdout).into_owned();
    let quiet = jabuti(&directory)
        .args(["--since", "main", "--format", "human"])
        .assert()
        .code(1);
    let terse = String::from_utf8_lossy(&quiet.get_output().stdout).into_owned();

    assert!(output.contains("  grow wide\n  ┴\n"), "{output}");
    let excerpt = [
        "   1 │ fn wide() {",
        "   2 ┃     let grown = 2;",
        "   3 │     let value = 1;",
    ]
    .map(|line| format!("{:17}{line}\n", ""))
    .concat();
    assert!(output.contains(&excerpt), "{output}");
    assert!(
        !terse.contains("grow wide") && !terse.contains("let grown"),
        "{terse}"
    );
}

#[test]
fn verbose_lists_only_the_commits_that_touched_the_project_checked() {
    let directory = project(&[
        ("app/src/lib.rs", &function_of("wide", 70)),
        ("web/index.js", "export const a = 1;\n"),
    ]);
    git(&directory, &["init", "-q", "-b", "main"]);
    git(&directory, &["config", "user.email", "test@example.com"]);
    git(&directory, &["config", "user.name", "test"]);
    commit_dated(&directory, "2026-01-01T12:00:00+00:00");
    git(&directory, &["switch", "-q", "-c", "feature"]);
    write(&directory, "web/index.js", "export const a = 2;\n");
    git(&directory, &["add", "-A"]);
    git(
        &directory,
        &["-c", "commit.gpgsign=false", "commit", "-qm", "web only"],
    );
    write(&directory, "app/src/lib.rs", &function_of("wide", 71));
    git(&directory, &["add", "-A"]);
    git(
        &directory,
        &["-c", "commit.gpgsign=false", "commit", "-qm", "app grows"],
    );

    let assert = jabuti(&directory)
        .current_dir(directory.path().join("app"))
        .args(["--since", "main", "--format", "human", "-v"])
        .assert()
        .code(1);
    let output = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();

    assert!(output.contains("  app grows\n  ┴\n"), "{output}");
    assert!(!output.contains("web only"), "{output}");
}

#[test]
fn a_misspelled_table_in_the_configuration_stops_the_run_instead_of_being_ignored() {
    let directory = project(&[
        (
            "jabuti.toml",
            "[layer.domain]\npaths = [\"src/domain/**\"]\n",
        ),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .code(2)
        .stderr(contains("unknown field `layer`"));
}

#[cfg(target_os = "linux")]
#[test]
fn a_terminal_is_shown_the_human_format_without_asking() {
    let directory = project(&[("src/lib.rs", &function_of("wide", 70))]);
    let command = format!("'{}' check .", env!("CARGO_BIN_EXE_jabuti"));

    let output = std::process::Command::new("script")
        .args(["--quiet", "--command", &command, "/dev/null"])
        .current_dir(directory.path())
        .env("TERM", "xterm-256color")
        .env("NO_COLOR", "1")
        .output()
        .expect("script from util-linux runs the check in a terminal");
    let shown = String::from_utf8_lossy(&output.stdout);

    assert!(shown.contains("⬢ jabuti  1 file · 1 unit"), "{shown}");
}

#[test]
fn a_clean_change_in_a_repository_is_one_line_in_the_human_format() {
    let directory = repository(&[("src/lib.rs", "fn small() {}\n")]);
    write(&directory, "src/lib.rs", "fn small() {}\n\nfn other() {}\n");

    jabuti(&directory)
        .args(["--format", "human"])
        .assert()
        .success()
        .stdout(contains("since HEAD · nothing to fix · 1 file · 2 units"));
}
