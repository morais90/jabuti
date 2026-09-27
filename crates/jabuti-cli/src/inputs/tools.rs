use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use anyhow::{Context, Result, bail};
use jabuti_core::model::Finding;
use jabuti_core::tools::cargo_diagnostics;
use jabuti_core::tools::coverage::Format;

use crate::config::Settings;
use crate::inputs::workspace;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Output {
    Diagnostics,
    Coverage { file: &'static str },
}

struct Provision {
    program: &'static str,
    arguments: &'static [&'static str],
}

pub(crate) struct Tool {
    pub(crate) name: &'static str,
    pub(crate) applies_when: &'static [&'static str],
    provision: &'static [Provision],
    probe: &'static [&'static str],
    invoke: &'static [&'static str],
    output: Output,
}

pub(crate) static CLIPPY: Tool = Tool {
    name: "clippy",
    applies_when: &["Cargo.toml"],
    provision: &[Provision {
        program: "rustup",
        arguments: &["component", "add", "clippy"],
    }],
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
    provision: &[
        Provision {
            program: "cargo",
            arguments: &[
                "install",
                "cargo-llvm-cov",
                "--version",
                "0.9.0",
                "--locked",
            ],
        },
        Provision {
            program: "rustup",
            arguments: &["component", "add", "llvm-tools-preview"],
        },
    ],
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
    fn applies(&self, root: &Path) -> bool {
        self.applies_when
            .iter()
            .any(|marker| root.join(marker).exists())
    }

    pub(crate) fn status(&self, root: &Path, enabled: bool) -> Status {
        if !self.applies(root) {
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

    fn install(&self, root: &Path) -> Result<()> {
        for provision in self.provision {
            let output = command(provision.program, root)
                .args(provision.arguments)
                .output()
                .with_context(|| format!("installing {} with {}", self.name, provision.program))?;
            if !output.status.success() {
                let reason = String::from_utf8_lossy(&output.stderr)
                    .lines()
                    .last()
                    .unwrap_or_default()
                    .to_owned();
                if reason.is_empty() {
                    bail!(
                        "installing {} with {} exited with {}",
                        self.name,
                        provision.program,
                        output.status
                    );
                }
                bail!(
                    "installing {} with {} exited with {}: {reason}",
                    self.name,
                    provision.program,
                    output.status
                );
            }
        }

        if !self.responds(root) {
            bail!("{} is still unavailable after installation", self.name);
        }

        Ok(())
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
        if findings.is_empty() {
            return Ok(findings);
        }

        Ok(located(findings, &workspace_root(root)?, project))
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

fn workspace_root(root: &Path) -> Result<PathBuf, String> {
    let output = command("cargo", root)
        .args(["locate-project", "--workspace", "--message-format", "plain"])
        .output()
        .map_err(|error| format!("locating the cargo workspace: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "locating the cargo workspace exited with {}",
            output.status
        ));
    }

    let manifest = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    manifest
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("cargo located no workspace from {}", root.display()))
}

fn located(findings: Vec<Finding>, base: &Path, project: &Path) -> Vec<Finding> {
    findings
        .into_iter()
        .filter_map(|finding| {
            let absolute = base.join(&finding.path);
            let resolved = absolute.canonicalize().unwrap_or(absolute);

            resolved.starts_with(project).then(|| Finding {
                path: workspace::display(&resolved, project),
                ..finding
            })
        })
        .collect()
}

pub(crate) fn install(root: &Path) -> Result<Vec<&'static str>> {
    let mut installed = Vec::new();
    for tool in ALL {
        if !tool.applies(root) || tool.responds(root) {
            continue;
        }
        tool.install(root)?;
        installed.push(tool.name);
    }

    Ok(installed)
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
    pub(crate) project: &'a Path,
    pub(crate) paths: &'a [PathBuf],
    pub(crate) settings: &'a Settings,
}

pub(crate) fn findings(
    scan: &Scan<'_>,
    notices: &mut Vec<String>,
) -> Result<(Vec<Finding>, Option<PathBuf>)> {
    let here = env::current_dir()?;
    let mut findings = Vec::new();
    let mut produced = None;

    for tool in ALL.iter().filter(|tool| enabled(scan.settings, tool.name)) {
        if !tool.status(&here, true).runnable() {
            continue;
        }

        match tool.run(&here, scan.project) {
            Ok(reported) => findings.extend(admitted(reported, scan)),
            Err(reason) => notices.push(reason),
        }
        produced = produced.or_else(|| tool.produces(scan.project));
    }

    Ok((findings, produced))
}

fn admitted(reported: Vec<Finding>, scan: &Scan<'_>) -> Vec<Finding> {
    reported
        .into_iter()
        .filter_map(|finding| scan.settings.policy.admit(finding))
        .collect()
}
