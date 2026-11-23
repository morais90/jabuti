use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use jabuti_core::diff::{Diff, Placement};
use jabuti_core::model::Span;

use crate::inputs::workspace;

#[derive(Debug)]
pub(crate) struct Changes {
    base: String,
    root: PathBuf,
    project: PathBuf,
    diff: Diff,
}

impl Changes {
    pub(crate) fn since(reference: &str, project: &Path) -> Result<Self> {
        let root = PathBuf::from(super::run(&["rev-parse", "--show-toplevel"])?.trim());
        let base = merge_base(reference)?;
        let unified = super::run(&["diff", "--unified=0", "--find-renames", &base])?;
        let untracked = super::run_at(&root, &["ls-files", "--others", "--exclude-standard"])?;

        let mut diff = Diff::parse(&unified);
        for path in untracked.lines().filter(|line| !line.is_empty()) {
            diff.add_whole(PathBuf::from(path));
        }

        Ok(Self {
            base,
            root: root.canonicalize().unwrap_or(root),
            project: project.to_path_buf(),
            diff,
        })
    }

    pub(crate) fn base_texts(
        &self,
        paths: &[PathBuf],
        project: &Path,
    ) -> Result<BTreeMap<PathBuf, String>> {
        let mut requested = BTreeMap::new();
        for path in paths {
            let Some(relative) = self.relative(path) else {
                continue;
            };
            if self.diff.covers(&relative) {
                let origin = self.diff.origin(&relative).to_path_buf();
                requested.insert(origin, PathBuf::from(workspace::display(path, project)));
            }
        }
        let inside: Vec<PathBuf> = requested.keys().cloned().collect();

        let blobs = super::blobs(&self.base, &inside)?;
        let texts = blobs
            .into_iter()
            .filter_map(|(relative, text)| Some((requested.remove(&relative)?, text)))
            .collect();

        Ok(texts)
    }

    pub(crate) fn commits(&self) -> Result<String> {
        let range = format!("{}..HEAD", self.base);
        let inside = self.relative(Path::new("")).unwrap_or_default();
        let scope = format!(":(literal){}", inside.display());
        let mut arguments = vec!["log", "--no-show-signature", "--format=%h%x00%s", &range];
        if !inside.as_os_str().is_empty() {
            arguments.extend(["--", &scope]);
        }
        super::run_at(&self.root, &arguments)
    }

    pub(crate) fn diff(&self) -> &Diff {
        &self.diff
    }

    pub(crate) fn placement(&self, paths: &[PathBuf]) -> Placement {
        let project = self
            .project
            .strip_prefix(&self.root)
            .map_or_else(|_| PathBuf::new(), Path::to_path_buf);
        let aliases = paths
            .iter()
            .filter_map(|path| {
                let shown = PathBuf::from(workspace::display(path, &self.project));
                let real = self.relative(path)?;

                (real != project.join(&shown)).then_some((shown, real))
            })
            .collect();

        Placement { project, aliases }
    }

    pub(crate) fn covers(&self, path: &Path) -> bool {
        self.relative(path)
            .is_some_and(|relative| self.diff.covers(&relative))
    }

    pub(crate) fn touches(&self, path: &Path, span: Span) -> bool {
        self.relative(path)
            .is_some_and(|relative| self.diff.touches(&relative, span))
    }

    pub(crate) fn relative(&self, path: &Path) -> Option<PathBuf> {
        let absolute = self.project.join(path).canonicalize().ok()?;

        absolute
            .strip_prefix(&self.root)
            .ok()
            .map(Path::to_path_buf)
    }
}

pub(crate) fn implied() -> Result<Option<String>> {
    if super::run(&["rev-parse", "--verify", "--quiet", "HEAD"]).is_err() {
        return Ok(None);
    }
    let current = super::run(&["symbolic-ref", "--quiet", "--short", "HEAD"]).ok();
    let current = current.as_deref().map(str::trim);

    match (principal(), current) {
        (Some(branch), Some(name)) if name == local(&branch) => Ok(Some("HEAD".to_owned())),
        (Some(branch), _) => Ok(Some(branch)),
        (None, Some(_)) => Ok(Some("HEAD".to_owned())),
        (None, None) => bail!(
            "HEAD is detached and no principal branch was found in origin/HEAD, main or master, so there is no telling what the change is. Name the reference it started from with --since, or list the whole repository with --all"
        ),
    }
}

fn principal() -> Option<String> {
    if let Ok(remote) = super::run(&[
        "symbolic-ref",
        "--quiet",
        "--short",
        "refs/remotes/origin/HEAD",
    ]) {
        return Some(remote.trim().to_owned());
    }

    ["main", "master"]
        .into_iter()
        .find(|branch| {
            super::run(&[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("refs/heads/{branch}"),
            ])
            .is_ok()
        })
        .map(str::to_owned)
}

fn local(branch: &str) -> &str {
    branch.strip_prefix("origin/").unwrap_or(branch)
}

fn merge_base(reference: &str) -> Result<String> {
    let listed = super::run(&["merge-base", "--all", "HEAD", reference])?;
    let mut bases = listed.lines();
    let base = bases
        .next()
        .with_context(|| format!("HEAD and {reference} share no history"))?;
    if bases.next().is_some() {
        bail!(
            "HEAD and {reference} have more than one merge base, so there is no single revision to compare against"
        );
    }

    Ok(base.to_owned())
}
