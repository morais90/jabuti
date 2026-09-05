use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use jabuti_core::history::churn;

#[derive(Debug)]
pub(crate) struct Churn {
    root: PathBuf,
    commits: BTreeMap<PathBuf, u32>,
}

impl Churn {
    pub(crate) fn of_repository() -> Result<Self> {
        let root = PathBuf::from(crate::git::run(&["rev-parse", "--show-toplevel"])?.trim());
        let log = crate::git::run(&["log", "--numstat", "--format="])?;

        Ok(Self {
            root: root.canonicalize().unwrap_or(root),
            commits: churn::tally(&log),
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
