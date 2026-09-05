use std::path::Path;
use std::process::Command;

use jabuti_core::model::Finding;
use jabuti_core::tools::cargo_diagnostics;

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
