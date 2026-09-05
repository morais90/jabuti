use std::path::Path;
use std::process::Command;

use anyhow::{Result, bail};
use jabuti_core::model::Finding;
use jabuti_core::tools::cargo_diagnostics;

use crate::config::Settings;
use crate::git::since::Changes;

pub(crate) struct Tool {
    pub(crate) name: &'static str,
    pub(crate) applies_when: &'static [&'static str],
    pub(crate) install_hint: &'static str,
    probe: &'static [&'static str],
    invoke: &'static [&'static str],
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
};

pub(crate) static ALL: &[&Tool] = &[&CLIPPY];

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

    pub(crate) fn run(&self, root: &Path) -> Result<Vec<Finding>, String> {
        let (program, arguments) = self.invoke.split_first().ok_or("no command configured")?;

        let output = Command::new(program)
            .args(arguments)
            .current_dir(root)
            .output()
            .map_err(|error| format!("running {}: {error}", self.name))?;

        let text = String::from_utf8_lossy(&output.stdout);
        let findings = cargo_diagnostics(self.name, &text);

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

    fn responds(&self, root: &Path) -> bool {
        let Some((program, arguments)) = self.probe.split_first() else {
            return false;
        };

        Command::new(program)
            .args(arguments)
            .current_dir(root)
            .output()
            .is_ok_and(|output| output.status.success())
    }
}

pub(crate) fn known(settings: &Settings) -> Result<()> {
    let known: Vec<&str> = ALL.iter().map(|tool| tool.name).collect();
    for name in settings.tools.keys() {
        if !known.contains(&name.as_str()) {
            bail!("unknown tool {name}, jabuti knows {}", known.join(", "));
        }
    }

    Ok(())
}

pub(crate) fn enabled(settings: &Settings, name: &str) -> bool {
    settings.tools.get(name).copied().unwrap_or(false)
}

pub(crate) fn findings(
    root: &Path,
    settings: &Settings,
    changes: Option<&Changes>,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for tool in ALL {
        if !tool.status(root, enabled(settings, tool.name)).runnable() {
            continue;
        }

        match tool.run(root) {
            Ok(reported) => findings.extend(admitted(reported, root, settings, changes)),
            Err(reason) => eprintln!("jabuti: {reason}"),
        }
    }

    findings
}

fn admitted(
    reported: Vec<Finding>,
    root: &Path,
    settings: &Settings,
    changes: Option<&Changes>,
) -> Vec<Finding> {
    reported
        .into_iter()
        .filter_map(|finding| settings.policy.admit(finding))
        .filter(|finding| in_scope(finding, root, changes))
        .collect()
}

fn in_scope(finding: &Finding, root: &Path, changes: Option<&Changes>) -> bool {
    changes.is_none_or(|changes| changes.touches(&root.join(&finding.path), finding.span))
}
