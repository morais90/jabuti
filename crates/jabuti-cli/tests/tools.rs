mod common;

use common::{jabuti, project};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

fn tools(directory: &tempfile::TempDir) -> assert_cmd::Command {
    let mut command = assert_cmd::Command::cargo_bin("jabuti").expect("the binary is built");
    command.current_dir(directory.path()).arg("tools");
    command
}

#[test]
fn a_tool_says_it_does_not_apply_when_the_project_has_no_marker_for_it() {
    let directory = project(&[("src/lib.rs", "fn small() {}\n")]);

    tools(&directory)
        .assert()
        .success()
        .stdout(contains("clippy").and(contains("not applicable here")));
}

#[test]
fn an_applicable_tool_that_is_off_says_how_to_turn_it_on() {
    let directory = project(&[
        (
            "Cargo.toml",
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
        ),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    tools(&directory)
        .assert()
        .success()
        .stdout(contains("enable with [tools.clippy] enabled = true"));
}

#[test]
fn a_tool_nobody_has_heard_of_stops_the_run() {
    let directory = project(&[
        ("jabuti.toml", "[tools.spline]\nenabled = true\n"),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .code(2)
        .stderr(contains("unknown tool spline"));
}

#[test]
fn install_rejects_an_unknown_configured_tool_before_running_commands() {
    let directory = project(&[
        ("jabuti.toml", "[tools.spline]\nenabled = true\n"),
        (
            "Cargo.toml",
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
        ),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    tools(&directory)
        .arg("install")
        .env("PATH", "")
        .assert()
        .code(2)
        .stderr(contains("unknown tool spline"));
}

#[test]
fn a_disabled_tool_never_runs_even_where_it_applies() {
    let directory = project(&[
        (
            "Cargo.toml",
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
        ),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

fn rust_project(extra: &[(&str, &str)]) -> tempfile::TempDir {
    let mut files = vec![
        (
            "Cargo.toml",
            "[package]\nname = \"lintme\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        ),
        (
            "src/lib.rs",
            "pub fn sum(values: &[i32]) -> i32 {\n    let mut total = 0;\n    for index in 0..values.len() {\n        total += values[index];\n    }\n    total\n}\n",
        ),
    ];
    files.extend_from_slice(extra);

    project(&files)
}

const CLIPPY_ON: &str = "[tools.clippy]\nenabled = true\n";

#[test]
fn an_enabled_and_available_tool_says_it_will_run() {
    let directory = rust_project(&[("jabuti.toml", CLIPPY_ON)]);

    tools(&directory)
        .assert()
        .success()
        .stdout(contains("clippy").and(contains("will run")));
}

#[test]
fn a_lint_from_the_tool_is_reported_like_any_other_finding() {
    let directory = rust_project(&[("jabuti.toml", CLIPPY_ON)]);

    jabuti(&directory)
        .assert()
        .stdout(contains("src/lib.rs:3").and(contains("clippy/needless_range_loop")));
}

#[test]
fn a_lint_the_configuration_switches_off_is_not_reported() {
    let directory = rust_project(&[(
        "jabuti.toml",
        "[tools.clippy]\nenabled = true\n\n[rules]\n\"clippy/needless_range_loop\" = { severity = \"off\" }\n",
    )]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_lint_switched_off_for_its_language_is_not_reported() {
    let directory = rust_project(&[(
        "jabuti.toml",
        "[tools.clippy]\nenabled = true\n\n[languages.rust.rules]\n\"clippy/needless_range_loop\" = { severity = \"off\" }\n",
    )]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_tool_that_cannot_be_found_says_how_to_install_it() {
    let directory = rust_project(&[("jabuti.toml", CLIPPY_ON)]);

    tools(&directory)
        .env("PATH", "")
        .assert()
        .success()
        .stdout(contains("install with `jabuti tools install`"));
}

#[test]
fn a_tool_that_fails_without_reporting_anything_says_so_and_does_not_pass_silently() {
    let directory = project(&[
        ("jabuti.toml", CLIPPY_ON),
        ("Cargo.toml", "[package]\nthis is not valid toml at all\n"),
        ("src/lib.rs", "fn small() {}\n"),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stderr(contains("clippy exited without reporting anything"));
}

#[test]
fn a_lint_outside_the_changed_lines_is_left_out_when_scoping_to_a_diff() {
    let directory = rust_project(&[("jabuti.toml", CLIPPY_ON)]);
    common::init_repository(&directory);
    common::write(&directory, "src/other.rs", "fn added() {}\n");

    jabuti(&directory)
        .arg("--since")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("No findings"));
}

#[test]
fn a_tool_that_finds_nothing_is_not_mistaken_for_a_tool_that_failed() {
    let directory = project(&[
        ("jabuti.toml", CLIPPY_ON),
        (
            "Cargo.toml",
            "[package]\nname = \"clean\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        ),
        (
            "src/lib.rs",
            "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
        ),
    ]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("No findings"))
        .stderr(contains("exited without reporting anything").not());
}

#[test]
fn every_language_reports_its_extensions_and_grammar_version() {
    let directory = project(&[("src/lib.rs", "fn small() {}\n")]);
    let mut command = assert_cmd::Command::cargo_bin("jabuti").expect("the binary is built");

    command
        .current_dir(directory.path())
        .arg("languages")
        .assert()
        .success()
        .stdout(
            contains("rust")
                .and(contains(".rs"))
                .and(contains("grammar"))
                .and(contains("15/15 native rules")),
        )
        .stdout(contains("kotlin").and(contains(".kt")))
        .stdout(contains("typescript").and(contains(".ts")));
}

#[test]
fn a_file_that_cannot_be_parsed_says_where_the_trouble_starts() {
    let directory = project(&[(
        "src/broken.rs",
        "fn first() -> usize {\n    1\n}\n\nfn second() -> usize {\n    let value = = 2;\n    value\n}\n",
    )]);

    jabuti(&directory)
        .assert()
        .success()
        .stdout(contains("src/broken.rs  unreadable syntax from line 6"));
}

#[cfg(unix)]
mod provisioning {
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::{Path, PathBuf};

    use predicates::prelude::PredicateBooleanExt as _;
    use predicates::str::contains;
    use tempfile::TempDir;

    use super::{jabuti, project, tools};

    const CARGO: &str = r#"#!/bin/sh
printf 'cargo' >> "$JABUTI_FAKE_LOG"
for argument in "$@"; do
    printf '\t%s' "$argument" >> "$JABUTI_FAKE_LOG"
done
printf '\n' >> "$JABUTI_FAKE_LOG"
printf 'cargo child stdout\n'
printf 'cargo child stderr\n' >&2

if [ "$#" -eq 2 ] && [ "$1" = "clippy" ] && [ "$2" = "--version" ]; then
    if [ -f "$JABUTI_FAKE_STATE/clippy" ]; then
        exit 0
    fi
    exit 1
fi

if [ "$#" -eq 2 ] && [ "$1" = "llvm-cov" ] && [ "$2" = "--version" ]; then
    if [ -f "$JABUTI_FAKE_STATE/cargo-llvm-cov" ]; then
        exit 0
    fi
    exit 1
fi

if [ "$#" -eq 5 ] && [ "$1" = "install" ] && [ "$2" = "cargo-llvm-cov" ] && [ "$3" = "--version" ] && [ "$4" = "0.9.0" ] && [ "$5" = "--locked" ]; then
    : > "$JABUTI_FAKE_STATE/cargo-llvm-cov"
    exit 0
fi

exit 91
"#;

    const RUSTUP: &str = r#"#!/bin/sh
printf 'rustup' >> "$JABUTI_FAKE_LOG"
for argument in "$@"; do
    printf '\t%s' "$argument" >> "$JABUTI_FAKE_LOG"
done
printf '\n' >> "$JABUTI_FAKE_LOG"

if [ "$JABUTI_FAKE_MODE" = "silent-command-failure" ]; then
    exit 42
fi

printf 'rustup child stdout\n'
printf 'rustup child stderr\n' >&2

if [ "$JABUTI_FAKE_MODE" = "command-failure" ]; then
    exit 41
fi

if [ "$#" -eq 3 ] && [ "$1" = "component" ] && [ "$2" = "add" ] && [ "$3" = "clippy" ]; then
    if [ "$JABUTI_FAKE_MODE" != "post-probe-failure" ]; then
        : > "$JABUTI_FAKE_STATE/clippy"
    fi
    exit 0
fi

if [ "$#" -eq 3 ] && [ "$1" = "component" ] && [ "$2" = "add" ] && [ "$3" = "llvm-tools-preview" ]; then
    : > "$JABUTI_FAKE_STATE/llvm-tools-preview"
    exit 0
fi

exit 92
"#;

    const PROVISION_LOG: &str = "cargo\tclippy\t--version\n\
rustup\tcomponent\tadd\tclippy\n\
cargo\tclippy\t--version\n\
cargo\tllvm-cov\t--version\n\
cargo\tinstall\tcargo-llvm-cov\t--version\t0.9.0\t--locked\n\
rustup\tcomponent\tadd\tllvm-tools-preview\n\
cargo\tllvm-cov\t--version\n";

    struct FakeTools {
        project: TempDir,
        bin: PathBuf,
        state: PathBuf,
        log: PathBuf,
    }

    impl FakeTools {
        fn new(applicable: bool) -> Self {
            let mut files = vec![("src/lib.rs", "fn small() {}\n")];
            if applicable {
                files.push((
                    "Cargo.toml",
                    "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
                ));
            }
            let project = project(&files);
            let bin = project.path().join("fake-bin");
            let state = project.path().join("fake-state");
            let log = project.path().join("provision.log");
            fs::create_dir_all(&bin).expect("fake executable directory");
            fs::create_dir_all(&state).expect("fake state directory");
            executable(&bin.join("cargo"), CARGO);
            executable(&bin.join("rustup"), RUSTUP);

            Self {
                project,
                bin,
                state,
                log,
            }
        }

        fn install(&self, mode: &str) -> assert_cmd::Command {
            let mut command = tools(&self.project);
            command
                .arg("install")
                .env("PATH", &self.bin)
                .env("JABUTI_FAKE_LOG", &self.log)
                .env("JABUTI_FAKE_STATE", &self.state)
                .env("JABUTI_FAKE_MODE", mode);
            command
        }

        fn list(&self) -> assert_cmd::Command {
            let mut command = tools(&self.project);
            command
                .env("PATH", &self.bin)
                .env("JABUTI_FAKE_LOG", &self.log)
                .env("JABUTI_FAKE_STATE", &self.state)
                .env("JABUTI_FAKE_MODE", "normal");
            command
        }

        fn check(&self) -> assert_cmd::Command {
            let mut command = jabuti(&self.project);
            command
                .env("PATH", &self.bin)
                .env("JABUTI_FAKE_LOG", &self.log)
                .env("JABUTI_FAKE_STATE", &self.state)
                .env("JABUTI_FAKE_MODE", "normal");
            command
        }

        fn mark_available(&self, tool: &str) {
            fs::write(self.state.join(tool), "").expect("availability marker");
        }

        fn recorded(&self) -> String {
            if self.log.exists() {
                fs::read_to_string(&self.log).expect("provision log")
            } else {
                String::new()
            }
        }
    }

    fn executable(path: &Path, contents: &str) {
        fs::write(path, contents).expect("fake executable");
        let mut permissions = fs::metadata(path).expect("fake metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("fake executable permissions");
    }

    #[test]
    fn install_uses_the_curated_command_sequence_for_every_applicable_unavailable_tool() {
        let fake = FakeTools::new(true);

        fake.install("normal")
            .assert()
            .success()
            .stdout("Installed clippy.\nInstalled cargo-llvm-cov.\n")
            .stderr("");

        assert_eq!(fake.recorded(), PROVISION_LOG);
    }

    #[test]
    fn install_does_not_enable_tools_or_write_configuration() {
        let fake = FakeTools::new(true);
        fake.install("normal")
            .assert()
            .success()
            .stdout("Installed clippy.\nInstalled cargo-llvm-cov.\n")
            .stderr("");
        fs::write(&fake.log, "").expect("log reset");

        fake.list().assert().success().stdout(
            contains("enable with [tools.clippy] enabled = true")
                .and(contains(
                    "enable with [tools.cargo-llvm-cov] enabled = true",
                ))
                .and(contains("will run").not()),
        );
        assert!(!fake.project.path().join("jabuti.toml").exists());
    }

    #[test]
    fn install_runs_no_commands_when_no_registered_tool_applies() {
        let fake = FakeTools::new(false);

        fake.install("normal")
            .assert()
            .success()
            .stdout("No tools need installation.\n")
            .stderr("");

        assert_eq!(fake.recorded(), "");
    }

    #[test]
    fn install_only_probes_tools_that_are_already_available() {
        let fake = FakeTools::new(true);
        fake.mark_available("clippy");
        fake.mark_available("cargo-llvm-cov");

        fake.install("normal")
            .assert()
            .success()
            .stdout("No tools need installation.\n")
            .stderr("");

        assert_eq!(
            fake.recorded(),
            "cargo\tclippy\t--version\ncargo\tllvm-cov\t--version\n"
        );
    }

    #[test]
    fn a_failed_install_command_exits_two_and_stops_provisioning() {
        let fake = FakeTools::new(true);

        fake.install("command-failure")
            .assert()
            .code(2)
            .stdout("")
            .stderr(
                contains("installing clippy with rustup exited with")
                    .and(contains("rustup child stderr")),
            );

        assert_eq!(
            fake.recorded(),
            "cargo\tclippy\t--version\nrustup\tcomponent\tadd\tclippy\n"
        );
    }

    #[test]
    fn an_install_command_with_no_stderr_still_exits_two_with_its_status() {
        let fake = FakeTools::new(true);

        fake.install("silent-command-failure")
            .assert()
            .code(2)
            .stdout("")
            .stderr("jabuti: installing clippy with rustup exited with exit status: 42\n");

        assert_eq!(
            fake.recorded(),
            "cargo\tclippy\t--version\nrustup\tcomponent\tadd\tclippy\n"
        );
    }

    #[test]
    fn a_tool_still_unavailable_after_installation_exits_two() {
        let fake = FakeTools::new(true);

        fake.install("post-probe-failure")
            .assert()
            .code(2)
            .stdout("")
            .stderr(contains("clippy is still unavailable after installation"));

        assert_eq!(
            fake.recorded(),
            "cargo\tclippy\t--version\nrustup\tcomponent\tadd\tclippy\ncargo\tclippy\t--version\n"
        );
    }

    #[test]
    fn check_probes_but_never_provisions_missing_tools() {
        let fake = FakeTools::new(true);
        fs::write(
            fake.project.path().join("jabuti.toml"),
            "[rules]\nhotspot = { severity = \"off\" }\n\n[tools.clippy]\nenabled = true\n\n[tools.cargo-llvm-cov]\nenabled = true\n",
        )
        .expect("configuration written");

        fake.check()
            .assert()
            .success()
            .stdout(contains("No findings").and(contains("child stdout").not()))
            .stderr("");

        assert_eq!(
            fake.recorded(),
            "cargo\tclippy\t--version\ncargo\tllvm-cov\t--version\n"
        );
    }
}
