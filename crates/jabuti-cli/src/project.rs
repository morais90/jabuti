use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use jabuti_core::lang;

pub(crate) fn sources(
    roots: &[PathBuf],
    exclude: &[String],
    project: &Path,
) -> Result<Vec<PathBuf>> {
    let absolute: Vec<PathBuf> = roots
        .iter()
        .map(|root| inside(root, project))
        .collect::<Result<_>>()?;
    let Some((first, rest)) = absolute.split_first() else {
        return Ok(Vec::new());
    };

    let mut overrides = OverrideBuilder::new(project);
    for pattern in exclude {
        overrides
            .add(&format!("!{pattern}"))
            .with_context(|| format!("invalid exclude pattern {pattern}"))?;
    }

    let mut builder = WalkBuilder::new(first);
    builder.overrides(overrides.build()?);
    for root in rest {
        builder.add(root);
    }

    let mut paths: Vec<PathBuf> = builder
        .build()
        .flatten()
        .map(ignore::DirEntry::into_path)
        .filter(|path| path.is_file() && lang::detect(path).is_some())
        .collect();

    paths.sort_by_key(|path| (path.is_symlink(), path.clone()));

    let mut seen = BTreeSet::new();
    paths.retain(|path| seen.insert(path.canonicalize().unwrap_or_else(|_| path.clone())));

    Ok(paths)
}

fn inside(root: &Path, project: &Path) -> Result<PathBuf> {
    let absolute = root
        .canonicalize()
        .with_context(|| format!("resolving {}", root.display()))?;
    if !absolute.starts_with(project) {
        bail!(
            "{} is outside the project at {}; run from a directory under the project or move jabuti.toml",
            root.display(),
            project.display()
        );
    }

    Ok(absolute)
}

pub(crate) fn display(path: &Path, project: &Path) -> String {
    path.strip_prefix(project)
        .unwrap_or(path)
        .display()
        .to_string()
}
