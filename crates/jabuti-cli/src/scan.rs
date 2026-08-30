use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use jabuti_core::metrics::{CognitiveIndex, DecisionIndex, LineIndex};
use jabuti_core::model::{Finding, Unit, UnitKind};
use jabuti_core::policy::{FileUnderReview, Policy};
use jabuti_core::report::Scanned;
use jabuti_core::{lang, syntax};
use rayon::prelude::*;

use crate::config::Settings;
use crate::since::Changes;

#[derive(Debug, Default)]
pub(crate) struct Outcome {
    pub(crate) findings: Vec<Finding>,
    pub(crate) scanned: Scanned,
    pub(crate) unreadable: Vec<String>,
}

#[derive(Debug, Default)]
struct Reviewed {
    findings: Vec<Finding>,
    units: usize,
    unreadable: Option<String>,
}

pub(crate) fn scan(
    roots: &[PathBuf],
    settings: &Settings,
    changes: Option<&Changes>,
) -> Result<Outcome> {
    let mut paths = sources(roots, &settings.exclude)?;
    if let Some(changes) = changes {
        paths.retain(|path| changes.covers(path));
    }

    let reviewed: Vec<Reviewed> = paths
        .par_iter()
        .map(|path| review(path, &settings.policy, changes))
        .collect();

    Ok(gather(reviewed))
}

fn gather(reviewed: Vec<Reviewed>) -> Outcome {
    let mut outcome = Outcome::default();

    for file in reviewed {
        match file.unreadable {
            Some(path) => outcome.unreadable.push(path),
            None => outcome.scanned.files += 1,
        }
        outcome.scanned.units += file.units;
        outcome.findings.extend(file.findings);
    }

    outcome
}

fn sources(roots: &[PathBuf], exclude: &[String]) -> Result<Vec<PathBuf>> {
    let Some((first, rest)) = roots.split_first() else {
        return Ok(Vec::new());
    };

    let mut overrides = OverrideBuilder::new(first);
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

    paths.sort();
    Ok(paths)
}

fn review(path: &Path, policy: &Policy, changes: Option<&Changes>) -> Reviewed {
    let Ok(source) = std::fs::read_to_string(path) else {
        return unreadable(path);
    };

    let Ok(parsed) = syntax::parse(&source, &lang::RUST) else {
        return unreadable(path);
    };

    let lines = LineIndex::new(&source, &parsed.comment_ranges());
    let decisions = DecisionIndex::new(&parsed.decisions());
    let cognitive = CognitiveIndex::new(&parsed.increments());
    let units = parsed.units();
    let counted = count_units(&units);

    let file = FileUnderReview {
        path: display(path),
        units,
        lines: &lines,
        decisions: &decisions,
        cognitive: &cognitive,
    };

    let mut findings = policy.evaluate(&file);
    if let Some(changes) = changes {
        findings.retain(|finding| changes.touches(path, finding.span));
    }

    Reviewed {
        findings,
        units: counted,
        unreadable: None,
    }
}

fn unreadable(path: &Path) -> Reviewed {
    Reviewed {
        unreadable: Some(display(path)),
        ..Reviewed::default()
    }
}

fn count_units(unit: &Unit) -> usize {
    let counted = usize::from(unit.kind != UnitKind::File);

    counted + unit.children.iter().map(count_units).sum::<usize>()
}

fn display(path: &Path) -> String {
    path.strip_prefix("./")
        .unwrap_or(path)
        .display()
        .to_string()
}
