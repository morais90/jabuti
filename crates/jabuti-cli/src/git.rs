use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

pub(crate) fn run(arguments: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(arguments)
        .output()
        .context("running git")?;

    collected(arguments, &output)
}

pub(crate) fn run_at(root: &Path, arguments: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .context("running git")?;

    collected(arguments, &output)
}

fn collected(arguments: &[&str], output: &std::process::Output) -> Result<String> {
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    String::from_utf8(output.stdout.clone()).context("git produced output that is not utf8")
}
