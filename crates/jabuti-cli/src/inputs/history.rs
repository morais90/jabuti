use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use jabuti_core::history::{churn, rhythm};
use jabuti_core::report::Rhythm;

#[derive(Debug)]
pub(crate) struct Churn {
    root: PathBuf,
    commits: BTreeMap<PathBuf, u32>,
}

impl Churn {
    fn of_repository() -> Result<Self> {
        let root =
            PathBuf::from(crate::inputs::git::run(&["rev-parse", "--show-toplevel"])?.trim());
        let log = crate::inputs::git::run(&["log", "--name-only", "--format="])?;

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

pub(crate) fn load(needed: bool, notices: &mut Vec<String>) -> Option<Churn> {
    match Churn::of_repository() {
        Ok(history) => Some(history),
        Err(_) if !needed => None,
        Err(reason) => {
            notices.push(format!(
                "hotspot needs a git repository, so it was not evaluated ({reason})"
            ));
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

pub(crate) fn rhythm(root: &Path, paths: &[&str]) -> Result<Option<BTreeMap<String, Rhythm>>> {
    let git = |arguments: &[&str]| crate::inputs::git::run_at(root, arguments);
    if paths.is_empty() {
        return Ok(None);
    }
    if git(&["rev-parse", "--verify", "--quiet", "HEAD"]).is_err() {
        return Ok(None);
    }
    let head = git(&["log", "-1", "--no-show-signature", "--format=%ct"])?;
    let head: i64 = head.trim().parse()?;
    let since = format!("--since=@{}", rhythm::window(head));
    let mut arguments = vec![
        "--literal-pathspecs",
        "log",
        "-z",
        "--no-show-signature",
        "--relative",
        rhythm::FORMAT,
        "--name-only",
        &since,
        "--",
    ];
    arguments.extend_from_slice(paths);
    let log = git(&arguments)?;

    Ok(Some(rhythm::read(&log, head)?))
}
