use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

#[derive(Debug)]
pub(crate) struct Churn {
    root: PathBuf,
    commits: BTreeMap<PathBuf, u32>,
}

impl Churn {
    pub(crate) fn of_repository() -> Result<Self> {
        let root = PathBuf::from(git(&["rev-parse", "--show-toplevel"])?.trim());
        let log = git(&["log", "--numstat", "--format="])?;

        Ok(Self {
            root: root.canonicalize().unwrap_or(root),
            commits: tally(&log),
        })
    }

    pub(crate) fn commits(&self, path: &Path) -> u32 {
        self.relative(path)
            .and_then(|relative| self.commits.get(&relative).copied())
            .unwrap_or(0)
    }

    fn relative(&self, path: &Path) -> Option<PathBuf> {
        let absolute = path.canonicalize().ok()?;

        absolute
            .strip_prefix(&self.root)
            .ok()
            .map(Path::to_path_buf)
    }
}

fn git(arguments: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(arguments)
        .output()
        .context("running git")?;

    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    String::from_utf8(output.stdout).context("git produced output that is not utf8")
}

fn tally(log: &str) -> BTreeMap<PathBuf, u32> {
    let mut commits: BTreeMap<PathBuf, u32> = BTreeMap::new();

    for line in log.lines() {
        if let Some(path) = touched_path(line) {
            *commits.entry(path).or_default() += 1;
        }
    }

    commits
}

fn touched_path(line: &str) -> Option<PathBuf> {
    let mut columns = line.split('\t');
    let _added = columns.next()?;
    let _removed = columns.next()?;

    columns.next().map(PathBuf::from)
}
