use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use jabuti_core::code::duplication::{self, FileFragments};
use jabuti_core::code::masking;
use jabuti_core::code::metrics::{self, CognitiveIndex, DecisionIndex, LineIndex};
use jabuti_core::code::review::{self, FileUnderReview};
use jabuti_core::code::units::{self, Unit};
use jabuti_core::model::{Finding, Reading, Rule, Severity, Span, UnitKind, Unreadable};
use jabuti_core::policy::Policy;
use jabuti_core::report::Scanned;
use jabuti_core::{lang, syntax};
use rayon::prelude::*;

use crate::git::since::Changes;
use crate::project;

#[derive(Debug, Default)]
pub(crate) struct Outcome {
    pub(crate) findings: Vec<Finding>,
    pub(crate) readings: Vec<Reading>,
    pub(crate) scanned: Scanned,
    pub(crate) unreadable: Vec<Unreadable>,
    pub(crate) measured: Vec<Measured>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Measured {
    pub(crate) path: String,
    pub(crate) span: Span,
    pub(crate) churn: u32,
    pub(crate) complexity: u32,
}

#[derive(Debug, Default)]
struct Reviewed {
    findings: Vec<Finding>,
    readings: Vec<Reading>,
    fragments: Option<FileFragments>,
    units: usize,
    unreadable: Option<Unreadable>,
    measured: Option<Measured>,
}

pub(crate) fn scan(
    paths: &[PathBuf],
    project: &Path,
    policy: &Policy,
    changes: Option<&Changes>,
    churn: &BTreeMap<PathBuf, u32>,
) -> Outcome {
    let minimum_nodes = duplication_limit(policy);
    let mut paths = paths.to_vec();
    if let (Some(changes), None) = (changes, minimum_nodes) {
        paths.retain(|path| changes.covers(path));
    }

    let reviewed: Vec<Reviewed> = paths
        .par_iter()
        .map(|path| {
            review(
                path,
                &Review {
                    policy,
                    project,
                    changes,
                    churn,
                    minimum_nodes,
                },
            )
        })
        .collect();

    let measured: Vec<Measured> = reviewed
        .iter()
        .filter_map(|file| file.measured.clone())
        .collect();

    let repeated: Vec<FileFragments> = reviewed
        .iter()
        .filter_map(|file| file.fragments.clone())
        .collect();

    let mut outcome = gather(covered(&paths, reviewed, changes));
    outcome.findings.extend(
        duplication::duplicates(&repeated, policy)
            .into_iter()
            .filter(|finding| in_diff(finding, changes)),
    );
    outcome.measured = measured;

    outcome
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

fn masked_errors(
    shown: &str,
    path: &Path,
    parsed: &syntax::Parsed<'_>,
    policy: &Policy,
) -> Vec<Finding> {
    let Some(spec) = lang::detect(path) else {
        return Vec::new();
    };
    if jabuti_core::code::lang::is_test_path(spec.id, Path::new(shown)) {
        return Vec::new();
    }

    masking::findings(shown, spec.id, &masking::maskings(parsed), policy)
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
        .config(Rule::DuplicateBlock)
        .filter(|config| config.severity != Severity::Off)
        .map(|config| config.limit)
}

fn in_diff(finding: &Finding, changes: Option<&Changes>) -> bool {
    changes.is_none_or(|changes| changes.touches(Path::new(&finding.path), finding.span))
}

struct Review<'a> {
    policy: &'a Policy,
    project: &'a Path,
    changes: Option<&'a Changes>,
    churn: &'a BTreeMap<PathBuf, u32>,
    minimum_nodes: Option<u32>,
}

fn review(path: &Path, context: &Review<'_>) -> Reviewed {
    let shown = project::display(path, context.project);
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

    let lines = LineIndex::new(&source, &metrics::comment_ranges(&parsed));
    let decisions = DecisionIndex::new(&metrics::decisions(&parsed));
    let cognitive = CognitiveIndex::new(&metrics::increments(&parsed));
    let units = units::units(&parsed);
    let counted = count_units(&units);

    let file = FileUnderReview {
        path: shown,
        language: spec.id,
        units,
        lines: &lines,
        decisions: &decisions,
        cognitive: &cognitive,
        churn: context.churn.get(path).copied().unwrap_or(0),
    };

    let measured = Measured {
        path: file.path.clone(),
        span: file.units.span,
        churn: file.churn,
        complexity: cognitive.total(&file.units),
    };

    let mut findings = review::evaluate(context.policy, &file);
    findings.extend(masked_errors(&file.path, path, &parsed, context.policy));
    findings.sort_by_key(|finding| finding.span.start_line);
    if let Some(changes) = context.changes {
        findings.retain(|finding| changes.touches(path, finding.span));
    }

    Reviewed {
        readings: review::read(&file),
        fragments: context.minimum_nodes.map(|minimum| FileFragments {
            path: file.path.clone(),
            fragments: duplication::fragments(&parsed, minimum),
        }),
        findings,
        units: counted,
        unreadable: None,
        measured: Some(measured),
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
