use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ignore::WalkBuilder;
use ignore::overrides::{Override, OverrideBuilder};
use jabuti_core::lang;

use crate::config::{self, Settings};

pub(crate) fn sources(
    roots: &[PathBuf],
    exclude: &[String],
    project: &Path,
) -> Result<Vec<PathBuf>> {
    let mut builder = walker(roots, project)?;
    builder.overrides(overrides(exclude, project)?);
    let mut paths = readable(&builder);

    paths.sort_by_key(|path| (path.is_symlink(), path.clone()));

    let mut seen = BTreeSet::new();
    paths.retain(|path| seen.insert(canonical(path)));

    Ok(paths)
}

pub(crate) fn excluded(
    roots: &[PathBuf],
    exclude: &[String],
    project: &Path,
) -> Result<Vec<PathBuf>> {
    let matcher = overrides(exclude, project)?;
    let mut excluded: Vec<PathBuf> = readable(&walker(roots, project)?)
        .into_iter()
        .filter(|path| {
            path.ancestors()
                .take_while(|ancestor| *ancestor != project)
                .any(|ancestor| matcher.matched(ancestor, ancestor != path).is_ignore())
        })
        .collect();
    excluded.sort();
    excluded.dedup();

    Ok(excluded)
}

pub(crate) fn admits(
    path: &Path,
    roots: &[PathBuf],
    exclude: &[String],
    project: &Path,
) -> Result<bool> {
    let within = roots
        .iter()
        .map(|root| canonical(root))
        .any(|root| path.starts_with(root));
    let matcher = overrides(exclude, project)?;
    let excluded = path
        .ancestors()
        .take_while(|ancestor| *ancestor != project)
        .any(|ancestor| matcher.matched(ancestor, ancestor != path).is_ignore());

    Ok(within && !excluded)
}

fn walker(roots: &[PathBuf], project: &Path) -> Result<WalkBuilder> {
    let absolute: Vec<PathBuf> = roots
        .iter()
        .map(|root| inside(root, project))
        .collect::<Result<_>>()?;
    let first = absolute.first().map_or(project, PathBuf::as_path);

    let mut builder = WalkBuilder::new(first);
    for root in absolute.iter().skip(1) {
        builder.add(root);
    }

    Ok(builder)
}

fn overrides(exclude: &[String], project: &Path) -> Result<Override> {
    let mut overrides = OverrideBuilder::new(project);
    for pattern in exclude {
        overrides
            .add(&format!("!{pattern}"))
            .with_context(|| format!("invalid exclude pattern {pattern}"))?;
    }

    Ok(overrides.build()?)
}

fn readable(builder: &WalkBuilder) -> Vec<PathBuf> {
    builder
        .build()
        .flatten()
        .map(ignore::DirEntry::into_path)
        .filter(|path| path.is_file() && lang::detect(path).is_some())
        .collect()
}

pub(crate) fn extended(
    paths: &[PathBuf],
    exclude: &[String],
    project: &Path,
) -> Result<Vec<PathBuf>> {
    let given: BTreeSet<PathBuf> = paths.iter().map(|path| canonical(path)).collect();
    let mut extended = paths.to_vec();
    for path in sources(&[project.to_path_buf()], exclude, project)? {
        if !given.contains(&canonical(&path)) {
            extended.push(path);
        }
    }
    extended.sort_by_key(|path| (path.is_symlink(), path.clone()));

    Ok(extended)
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
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

pub(crate) fn discover() -> Result<(PathBuf, Settings)> {
    let here = std::env::current_dir()
        .context("reading the current directory")?
        .canonicalize()
        .context("resolving the current directory")?;

    let boundary = repository_root().unwrap_or_else(|| here.clone());
    let root = here
        .ancestors()
        .take_while(|directory| directory.starts_with(&boundary))
        .find(|directory| directory.join(config::FILE_NAME).is_file())
        .map_or(here.clone(), Path::to_path_buf);
    let settings = config::load(&root)?;

    Ok((root, settings))
}

fn repository_root() -> Option<PathBuf> {
    let top = super::git::run(&["rev-parse", "--show-toplevel"]).ok()?;

    PathBuf::from(top.trim()).canonicalize().ok()
}
