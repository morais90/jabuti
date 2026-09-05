use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use jabuti_core::history::churn;
use jabuti_core::model::Rule;

use crate::config::Settings;

#[derive(Debug)]
pub(crate) struct Churn {
    root: PathBuf,
    commits: BTreeMap<PathBuf, u32>,
}

impl Churn {
    fn of_repository() -> Result<Self> {
        let root = PathBuf::from(crate::git::run(&["rev-parse", "--show-toplevel"])?.trim());
        let log = crate::git::run(&["log", "--numstat", "--format="])?;

        Ok(Self {
            root: root.canonicalize().unwrap_or(root),
            commits: churn::tally(&log),
        })
    }

    fn commits(&self, path: &Path) -> u32 {
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

pub(crate) fn load(settings: &Settings) -> Option<Churn> {
    if !settings.enabled(Rule::Churn) && !settings.enabled(Rule::Hotspot) {
        return None;
    }

    match Churn::of_repository() {
        Ok(history) => Some(history),
        Err(reason) => {
            eprintln!(
                "jabuti: churn and hotspot need a git repository, so they were not evaluated ({reason})"
            );
            None
        }
    }
}

pub(crate) fn commits(history: Option<&Churn>, paths: &[PathBuf]) -> BTreeMap<PathBuf, u32> {
    let Some(history) = history else {
        return BTreeMap::new();
    };

    paths
        .iter()
        .map(|path| (path.clone(), history.commits(path)))
        .collect()
}
