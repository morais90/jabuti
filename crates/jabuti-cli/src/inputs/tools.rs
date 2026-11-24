use std::cell::OnceCell;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use anyhow::{Context, Result, bail};
use jabuti_core::lang::{self, LanguageId};
use jabuti_core::model::Finding;
use jabuti_core::tools::coverage::Format;
use jabuti_core::tools::{cargo_diagnostics, enabled_by_default};

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
    language: LanguageId,
    levels: &'static [&'static [&'static str]],
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
    language: LanguageId::Rust,
    levels: &[
        &["clippy-driver", "--edition", "2015", "-W", "help"],
        &["clippy-driver", "--edition", "2018", "-W", "help"],
        &["clippy-driver", "--edition", "2021", "-W", "help"],
        &["clippy-driver", "--edition", "2024", "-W", "help"],
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
    language: LanguageId::Rust,
    levels: &[],
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

    pub(crate) fn run(
        &self,
        root: &Path,
        project: &Path,
        notices: &mut Vec<String>,
    ) -> Result<Vec<Finding>, String> {
        let (program, arguments) = self.invoke.split_first().ok_or("no command configured")?;
        let arguments = self.arguments(arguments, root)?;

        let output = command(program, root)
            .args(arguments)
            .output()
            .map_err(|error| format!("running {}: {error}", self.name))?;

        let defaults = OnceCell::new();
        let asked = |lint: &str| {
            defaults
                .get_or_init(|| self.enabled_by_default(root))
                .as_ref()
                .map_or(true, |enabled| enabled.contains(lint))
        };
        let findings = match self.output {
            Output::Diagnostics => {
                cargo_diagnostics(self.name, &String::from_utf8_lossy(&output.stdout), asked)
            }
            Output::Coverage { file: _ } => Vec::new(),
        };
        if let Some(Err(reason)) = defaults.get() {
            notices.push(format!(
                "{} did not list the lints it enables by default, so every warning it reported is kept: {reason}",
                self.name
            ));
        }

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

        Ok(located(findings, root, project))
    }

    fn arguments(&self, fixed: &[&str], root: &Path) -> Result<Vec<OsString>, String> {
        let mut arguments: Vec<OsString> = fixed.iter().map(OsString::from).collect();

        if let Some(report) = self.produces(root) {
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

    fn produces(&self, root: &Path) -> Option<PathBuf> {
        match self.output {
            Output::Diagnostics => None,
            Output::Coverage { file } => Some(root.join(file)),
        }
    }

    fn packages(&self, paths: &[&PathBuf], project: &Path) -> BTreeSet<PathBuf> {
        paths
            .iter()
            .filter(|path| lang::detect(path).is_some_and(|spec| spec.id == self.language))
            .filter_map(|path| {
                path.ancestors()
                    .skip(1)
                    .take_while(|directory| directory.starts_with(project))
                    .find(|directory| self.applies(directory))
            })
            .map(Path::to_path_buf)
            .collect()
    }

    fn workspaces(
        &self,
        packages: BTreeSet<PathBuf>,
        project: &Path,
        notices: &mut Vec<String>,
    ) -> BTreeSet<PathBuf> {
        let mut roots = BTreeSet::new();
        for package in packages {
            let shown = place(&package, project);
            if !self.responds(&package) {
                notices.push(format!(
                    "{} is enabled but does not answer in {shown}; `jabuti tools install` run there provisions it",
                    self.name
                ));
                continue;
            }
            match workspace_root(&package) {
                Ok(root) => {
                    roots.insert(root);
                }
                Err(reason) => notices.push(format!("{} skipped {shown}: {reason}", self.name)),
            }
        }

        roots
    }

    fn enabled_by_default(&self, root: &Path) -> Result<BTreeSet<String>, String> {
        let mut enabled = BTreeSet::new();
        for listing in self.levels {
            enabled.extend(listed(listing, root)?);
        }

        Ok(enabled)
    }

    fn run_in(
        &self,
        roots: BTreeSet<PathBuf>,
        scan: &Scan<'_>,
        notices: &mut Vec<String>,
    ) -> (Vec<Finding>, Vec<Produced>) {
        let mut findings = Vec::new();
        let mut produced = Vec::new();
        for root in roots {
            match self.run(&root, scan.project, notices) {
                Ok(reported) => {
                    findings.extend(admitted(reported, scan));
                    produced.extend(self.produces(&root).map(|report| Produced {
                        workspace: root,
                        report,
                    }));
                }
                Err(reason) => notices.push(reason),
            }
        }

        (findings, produced)
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

fn place(directory: &Path, project: &Path) -> String {
    let shown = workspace::display(directory, project);
    if shown.is_empty() {
        return ".".to_owned();
    }

    shown
}

fn listed(listing: &[&str], root: &Path) -> Result<BTreeSet<String>, String> {
    let (program, arguments) = listing.split_first().ok_or("no listing configured")?;
    let output = command(program, root)
        .args(arguments)
        .output()
        .map_err(|error| format!("running {program}: {error}"))?;
    let enabled = enabled_by_default(&String::from_utf8_lossy(&output.stdout));
    if !output.status.success() {
        return Err(format!("{program} exited with {}", output.status));
    }
    if enabled.is_empty() {
        return Err(format!("{program} listed no lint levels"));
    }

    Ok(enabled)
}

fn workspace_root(package: &Path) -> Result<PathBuf, String> {
    let output = command("cargo", package)
        .args(["locate-project", "--workspace", "--message-format", "plain"])
        .output()
        .map_err(|error| format!("locating the cargo workspace: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "locating the cargo workspace exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .last()
                .unwrap_or_default()
        ));
    }

    let manifest = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());

    Ok(manifest.parent().unwrap_or(package).to_path_buf())
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

pub(crate) fn produces_coverage(settings: &Settings) -> bool {
    ALL.iter()
        .any(|tool| matches!(tool.output, Output::Coverage { .. }) && enabled(settings, tool.name))
}

pub(crate) fn enabled(settings: &Settings, name: &str) -> bool {
    settings.tools.get(name).copied().unwrap_or(false)
}

pub(crate) struct Scan<'a> {
    pub(crate) project: &'a Path,
    pub(crate) paths: &'a [PathBuf],
    pub(crate) touched: Option<&'a BTreeSet<PathBuf>>,
    pub(crate) settings: &'a Settings,
}

pub(crate) struct Produced {
    pub(crate) workspace: PathBuf,
    pub(crate) report: PathBuf,
}

pub(crate) fn findings(
    scan: &Scan<'_>,
    notices: &mut Vec<String>,
) -> (Vec<Finding>, Vec<Produced>) {
    let checked: Vec<&PathBuf> = scan
        .paths
        .iter()
        .filter(|path| scan.touched.is_none_or(|touched| touched.contains(*path)))
        .collect();
    let mut findings = Vec::new();
    let mut produced = Vec::new();

    for tool in ALL.iter().filter(|tool| enabled(scan.settings, tool.name)) {
        let measures_a_change = matches!(tool.output, Output::Coverage { .. });
        if measures_a_change && scan.touched.is_none() {
            continue;
        }
        let packages = tool.packages(&checked, scan.project);
        if packages.is_empty() && scan.touched.is_none() {
            notices.push(format!(
                "{} is enabled, but no {} file checked sits under a {}",
                tool.name,
                tool.language.name(),
                tool.applies_when.join(" or ")
            ));
        }
        let roots = tool.workspaces(packages, scan.project, notices);
        let (reported, made) = tool.run_in(roots, scan, notices);
        findings.extend(reported);
        produced.extend(made);
    }

    (findings, produced)
}

fn admitted(reported: Vec<Finding>, scan: &Scan<'_>) -> Vec<Finding> {
    reported
        .into_iter()
        .filter_map(|finding| scan.settings.policy.admit(finding))
        .collect()
}
