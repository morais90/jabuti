use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use jabuti_core::duplication::{self, FileFragments};
use jabuti_core::hotspot::{self, FileSummary};
use jabuti_core::metrics::{CognitiveIndex, DecisionIndex, LineIndex};
use jabuti_core::model::{Finding, Reading, Unit, UnitKind, Unreadable};
use jabuti_core::policy::{FileUnderReview, Policy};
use jabuti_core::report::Scanned;
use jabuti_core::{lang, masking, syntax};
use rayon::prelude::*;

use crate::churn::Churn;
use crate::config::Settings;
use crate::since::Changes;

#[derive(Debug, Default)]
pub(crate) struct Outcome {
    pub(crate) findings: Vec<Finding>,
    pub(crate) readings: Vec<Reading>,
    pub(crate) scanned: Scanned,
    pub(crate) unreadable: Vec<Unreadable>,
}

#[derive(Debug, Default)]
struct Reviewed {
    findings: Vec<Finding>,
    readings: Vec<Reading>,
    fragments: Option<FileFragments>,
    units: usize,
    unreadable: Option<Unreadable>,
    summary: Option<FileSummary>,
}

pub(crate) fn scan(
    roots: &[PathBuf],
    project: &Path,
    settings: &Settings,
    changes: Option<&Changes>,
    churn: Option<&Churn>,
) -> Result<Outcome> {
    let minimum_nodes = duplication_limit(&settings.policy);
    let mut paths = sources(roots, &settings.exclude, project)?;
    if let (Some(changes), None) = (changes, minimum_nodes) {
        paths.retain(|path| changes.covers(path));
    }

    let reviewed: Vec<Reviewed> = paths
        .par_iter()
        .map(|path| {
            review(
                path,
                &Review {
                    policy: &settings.policy,
                    project,
                    changes,
                    churn,
                    minimum_nodes,
                },
            )
        })
        .collect();

    let summaries: Vec<FileSummary> = reviewed
        .iter()
        .filter_map(|file| file.summary.clone())
        .collect();

    let repeated: Vec<FileFragments> = reviewed
        .iter()
        .filter_map(|file| file.fragments.clone())
        .collect();

    let mut outcome = gather(covered(&paths, reviewed, changes));
    outcome.findings.extend(
        duplication::duplicates(&repeated, &settings.policy)
            .into_iter()
            .filter(|finding| in_diff(finding, changes)),
    );

    if changes.is_none() {
        outcome
            .findings
            .extend(hotspot::hotspots(&summaries, &settings.policy));
        outcome.findings.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then(left.span.start_line.cmp(&right.span.start_line))
        });
    }

    Ok(outcome)
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
        outcome.readings.extend(file.readings);
    }

    outcome
}

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

fn masked_errors(
    shown: &str,
    path: &Path,
    parsed: &syntax::Parsed<'_>,
    policy: &Policy,
) -> Vec<Finding> {
    let Some(spec) = lang::detect(path) else {
        return Vec::new();
    };
    if spec.is_test_path(Path::new(shown)) {
        return Vec::new();
    }

    masking::findings(shown, spec.id, &parsed.maskings(), policy)
}

fn covered(paths: &[PathBuf], reviewed: Vec<Reviewed>, changes: Option<&Changes>) -> Vec<Reviewed> {
    let Some(changes) = changes else {
        return reviewed;
    };

    paths
        .iter()
        .zip(reviewed)
        .filter(|(path, _)| changes.covers(path))
        .map(|(_, file)| file)
        .collect()
}

fn duplication_limit(policy: &Policy) -> Option<u32> {
    policy
        .config(jabuti_core::model::Rule::DuplicateBlock)
        .filter(|config| config.severity != jabuti_core::model::Severity::Off)
        .map(|config| config.limit)
}

fn in_diff(finding: &Finding, changes: Option<&Changes>) -> bool {
    changes.is_none_or(|changes| changes.touches(Path::new(&finding.path), finding.span))
}

struct Review<'a> {
    policy: &'a Policy,
    project: &'a Path,
    changes: Option<&'a Changes>,
    churn: Option<&'a Churn>,
    minimum_nodes: Option<u32>,
}

fn review(path: &Path, context: &Review<'_>) -> Reviewed {
    let shown = display(path, context.project);
    let Ok(source) = std::fs::read_to_string(path) else {
        return rejected(shown, "the file could not be read");
    };

    let Some(spec) = lang::detect(path) else {
        return rejected(shown, "no language claims this extension");
    };
    let parsed = match syntax::parse(&source, spec) {
        Ok(parsed) => parsed,
        Err(reason) => return rejected(shown, &reason.to_string()),
    };

    let lines = LineIndex::new(&source, &parsed.comment_ranges());
    let decisions = DecisionIndex::new(&parsed.decisions());
    let cognitive = CognitiveIndex::new(&parsed.increments());
    let units = parsed.units();
    let counted = count_units(&units);

    let file = FileUnderReview {
        path: shown,
        language: spec.id,
        units,
        lines: &lines,
        decisions: &decisions,
        cognitive: &cognitive,
        churn: context.churn.map_or(0, |churn| churn.commits(path)),
    };

    let summary = FileSummary {
        path: file.path.clone(),
        span: file.units.span,
        churn: file.churn,
        complexity: cognitive.total(&file.units),
    };

    let mut findings = context.policy.evaluate(&file);
    findings.extend(masked_errors(&file.path, path, &parsed, context.policy));
    findings.sort_by_key(|finding| finding.span.start_line);
    if let Some(changes) = context.changes {
        findings.retain(|finding| changes.touches(path, finding.span));
    }

    Reviewed {
        readings: context.policy.read(&file),
        fragments: context.minimum_nodes.map(|minimum| FileFragments {
            path: file.path.clone(),
            fragments: parsed.fragments(minimum),
        }),
        findings,
        units: counted,
        unreadable: None,
        summary: Some(summary),
    }
}

fn rejected(shown: String, reason: &str) -> Reviewed {
    Reviewed {
        unreadable: Some(Unreadable {
            path: shown,
            reason: reason.to_owned(),
        }),
        ..Reviewed::default()
    }
}

fn count_units(unit: &Unit) -> usize {
    let counted = usize::from(unit.kind != UnitKind::File);

    counted + unit.children.iter().map(count_units).sum::<usize>()
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
