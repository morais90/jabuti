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
        let unified = super::run(&["diff", "--unified=0", &base])?;
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
                requested.insert(relative, PathBuf::from(workspace::display(path, project)));
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
