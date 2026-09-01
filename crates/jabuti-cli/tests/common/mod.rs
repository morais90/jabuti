#![allow(dead_code)]

use std::fs;
use std::process::Command as Process;

use assert_cmd::Command;
use tempfile::TempDir;

pub(crate) fn project(files: &[(&str, &str)]) -> TempDir {
    let directory = TempDir::new().expect("temporary directory");

    for (name, contents) in files {
        write(&directory, name, contents);
    }

    directory
}

pub(crate) fn repository(files: &[(&str, &str)]) -> TempDir {
    let directory = project(files);
    init_repository(&directory);

    directory
}

pub(crate) fn init_repository(directory: &TempDir) {
    git(directory, &["init", "-q", "-b", "main"]);
    git(directory, &["config", "user.email", "test@example.com"]);
    git(directory, &["config", "user.name", "test"]);
    git(directory, &["add", "-A"]);
    git(directory, &["commit", "-qm", "base"]);
}

pub(crate) fn commit(directory: &TempDir, message: &str) {
    git(directory, &["add", "-A"]);
    git(directory, &["commit", "-qm", message]);
}

pub(crate) fn write(directory: &TempDir, name: &str, contents: &str) {
    let path = directory.path().join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directory");
    }
    fs::write(path, contents).expect("fixture written");
}

pub(crate) fn append(directory: &TempDir, name: &str, contents: &str) {
    let path = directory.path().join(name);
    let existing = fs::read_to_string(&path).expect("file exists");
    fs::write(path, existing + contents).expect("fixture extended");
}

pub(crate) fn binary(directory: &TempDir) -> Command {
    let mut command = Command::cargo_bin("jabuti").expect("the binary is built");
    command.current_dir(directory.path());
    command
}

pub(crate) fn jabuti(directory: &TempDir) -> Command {
    let mut command = binary(directory);
    command.arg("check").arg(".");
    command
}

pub(crate) fn function_of(name: &str, body_lines: usize) -> String {
    let body = "    let value = 1;\n".repeat(body_lines);

    format!("fn {name}() {{\n{body}}}\n")
}

pub(crate) fn shaped_like(name: &str, first: &str, second: &str) -> String {
    format!(
        "fn {name}(input: &str) -> Option<String> {{
    let mut {first} = input.splitn(2, ':');
    let {second} = {first}.next()?.trim().to_lowercase();
    if {second}.is_empty() {{
        return None;
    }}
    Some({second})
}}
"
    )
}

pub(crate) fn report_duplication_above(limit: usize) -> String {
    format!(
        "[rules]
duplicate-block = {{ limit = {limit}, severity = \"warning\" }}
hotspot = {{ severity = \"off\" }}
"
    )
}

pub(crate) fn error_on_long_functions(limit: usize) -> String {
    format!("[rules]\nfunction-lines = {{ limit = {limit}, severity = \"error\" }}\n")
}

fn git(directory: &TempDir, arguments: &[&str]) {
    let status = Process::new("git")
        .args(arguments)
        .current_dir(directory.path())
        .status()
        .expect("git runs");

    assert!(status.success(), "git {arguments:?}");
}
