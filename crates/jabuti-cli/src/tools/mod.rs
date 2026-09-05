mod coverage;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use anyhow::{Result, bail};
use jabuti_core::model::Finding;
use jabuti_core::tools::cargo_diagnostics;
use jabuti_core::tools::coverage::Format;

use crate::config::Settings;
use crate::git::since::Changes;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Output {
    Diagnostics,
    Coverage { file: &'static str },
}

pub(crate) struct Tool {
    pub(crate) name: &'static str,
    pub(crate) applies_when: &'static [&'static str],
    pub(crate) install_hint: &'static str,
    probe: &'static [&'static str],
    invoke: &'static [&'static str],
    output: Output,
}

pub(crate) static CLIPPY: Tool = Tool {
    name: "clippy",
    applies_when: &["Cargo.toml"],
    install_hint: "rustup component add clippy",
    probe: &["cargo", "clippy", "--version"],
    invoke: &[
        "cargo",
        "clippy",
        "--workspace",
        "--all-targets",
        "--message-format=json",
        "--quiet",
    ],
    output: Output::Diagnostics,
};

pub(crate) static LLVM_COV: Tool = Tool {
    name: "cargo-llvm-cov",
    applies_when: &["Cargo.toml"],
    install_hint: "cargo install cargo-llvm-cov && rustup component add llvm-tools-preview",
    probe: &["cargo", "llvm-cov", "--version"],
    invoke: &[
        "cargo",
        "llvm-cov",
        "--workspace",
        "--lcov",
        "--output-path",
    ],
    output: Output::Coverage {
        file: "target/jabuti/coverage.lcov",
    },
};

pub(crate) static ALL: &[&Tool] = &[&CLIPPY, &LLVM_COV];

const COVERAGE_WRAPPER_MARKER: &str = "__CARGO_LLVM_COV_RUSTC_WRAPPER";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    NotApplicable,
    Unavailable,
    Disabled,
    Ready,
}

impl Status {
    pub(crate) fn runnable(self) -> bool {
        self == Self::Ready
    }
}

impl Tool {
    pub(crate) fn status(&self, root: &Path, enabled: bool) -> Status {
        if !self.applies_when.iter().any(|m| root.join(m).exists()) {
            return Status::NotApplicable;
        }
        if !self.responds(root) {
            return Status::Unavailable;
        }
        if !enabled {
            return Status::Disabled;
        }

        Status::Ready
    }

    pub(crate) fn run(&self, root: &Path, project: &Path) -> Result<Vec<Finding>, String> {
        let (program, arguments) = self.invoke.split_first().ok_or("no command configured")?;
        let arguments = self.arguments(arguments, project)?;

        let output = command(program, root)
            .args(arguments)
            .output()
            .map_err(|error| format!("running {}: {error}", self.name))?;

        let findings = match self.output {
            Output::Diagnostics => {
                cargo_diagnostics(self.name, &String::from_utf8_lossy(&output.stdout))
            }
            Output::Coverage { file: _ } => Vec::new(),
        };

        if findings.is_empty() && !output.status.success() {
            return Err(format!(
                "{} exited without reporting anything: {}",
                self.name,
                String::from_utf8_lossy(&output.stderr)
                    .lines()
                    .last()
                    .unwrap_or_default()
            ));
        }

        Ok(findings)
    }

    fn arguments(&self, fixed: &[&str], project: &Path) -> Result<Vec<OsString>, String> {
        let mut arguments: Vec<OsString> = fixed.iter().map(OsString::from).collect();

        if let Some(report) = self.produces(project) {
            if let Some(directory) = report.parent() {
                fs::create_dir_all(directory).map_err(|error| {
                    format!(
                        "creating {} for {}: {error}",
                        directory.display(),
                        self.name
                    )
                })?;
            }
            arguments.push(report.into_os_string());
        }

        Ok(arguments)
    }

    fn produces(&self, project: &Path) -> Option<PathBuf> {
        match self.output {
            Output::Diagnostics => None,
            Output::Coverage { file } => Some(project.join(file)),
        }
    }

    fn responds(&self, root: &Path) -> bool {
        let Some((program, arguments)) = self.probe.split_first() else {
            return false;
        };

        command(program, root)
            .args(arguments)
            .output()
            .is_ok_and(|output| output.status.success())
    }
}

fn command(program: &str, root: &Path) -> Command {
    let mut command = Command::new(program);
    command.current_dir(root);

    if env::var_os(COVERAGE_WRAPPER_MARKER).is_some() {
        command.env_remove("RUSTC_WRAPPER");
    }

    command
}

pub(crate) fn known(settings: &Settings) -> Result<()> {
    let known: Vec<&str> = ALL.iter().map(|tool| tool.name).collect();
    for name in settings.tools.keys() {
        if !known.contains(&name.as_str()) {
            bail!("unknown tool {name}, jabuti knows {}", known.join(", "));
        }
    }

    if let Some(report) = &settings.coverage
        && Format::of(report).is_none()
    {
        bail!(
            "coverage report {} has an extension jabuti cannot read; use .lcov, .info or .xml",
            report.display()
        );
    }

    Ok(())
}

pub(crate) fn enabled(settings: &Settings, name: &str) -> bool {
    settings.tools.get(name).copied().unwrap_or(false)
}

pub(crate) struct Scan<'a> {
    pub(crate) here: &'a Path,
    pub(crate) project: &'a Path,
    pub(crate) paths: &'a [PathBuf],
    pub(crate) settings: &'a Settings,
    pub(crate) changes: Option<&'a Changes>,
}

pub(crate) fn findings(scan: &Scan<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut produced = None;

    for tool in ALL {
        if !tool
            .status(scan.here, enabled(scan.settings, tool.name))
            .runnable()
        {
            continue;
        }

        match tool.run(scan.here, scan.project) {
            Ok(reported) => findings.extend(admitted(reported, scan)),
            Err(reason) => eprintln!("jabuti: {reason}"),
        }
        produced = produced.or_else(|| tool.produces(scan.project));
    }

    if let Some(changes) = scan.changes {
        let configured = scan
            .settings
            .coverage
            .as_ref()
            .map(|report| scan.project.join(report));
        let report = configured.or(produced);
        findings.extend(coverage::findings(scan, changes, report.as_deref()));
    }

    findings
}

fn admitted(reported: Vec<Finding>, scan: &Scan<'_>) -> Vec<Finding> {
    reported
        .into_iter()
        .filter_map(|finding| scan.settings.policy.admit(finding))
        .filter(|finding| in_scope(finding, scan))
        .collect()
}

fn in_scope(finding: &Finding, scan: &Scan<'_>) -> bool {
    scan.changes
        .is_none_or(|changes| changes.touches(&scan.here.join(&finding.path), finding.span))
}
