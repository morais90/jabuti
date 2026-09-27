use std::path::{Path, PathBuf};

use jabuti_core::catalog::Rule;
use jabuti_core::crossings::hotspot::{self, FileSummary};
use jabuti_core::crossings::uncovered::{self, FileUnderCoverage};
use jabuti_core::lang::{self, LangSpec};
use jabuti_core::model::{Finding, Span};
use jabuti_core::policy::Policy;
use jabuti_core::tools::coverage::Coverage;

use crate::code::Measured;
use crate::inputs::git::since::Changes;
use crate::inputs::tools::Scan;
use crate::inputs::{coverage, workspace};

pub(crate) fn hotspots(measured: &[Measured], policy: &Policy) -> Vec<Finding> {
    hotspot::hotspots(&summaries(measured), policy)
}

fn summaries(measured: &[Measured]) -> Vec<FileSummary> {
    measured
        .iter()
        .map(|file| FileSummary {
            path: file.path.clone(),
            span: file.span,
            churn: file.churn,
            complexity: file.complexity,
        })
        .collect()
}

struct Candidate<'a> {
    path: &'a Path,
    shown: String,
    spec: &'static LangSpec,
}

pub(crate) fn uncovered(
    scan: &Scan<'_>,
    changes: &Changes,
    produced: Option<PathBuf>,
    notices: &mut Vec<String>,
) -> Vec<Finding> {
    if !scan.settings.enabled(Rule::UncoveredNewCode) {
        return Vec::new();
    }
    let configured = scan
        .settings
        .coverage
        .as_ref()
        .map(|report| scan.project.join(report));
    let Some(report) = configured.or(produced) else {
        notices.push(format!(
            "{} needs a coverage report; set [coverage] report or enable a tool that produces one",
            Rule::UncoveredNewCode.id()
        ));
        return Vec::new();
    };

    let candidates = candidates(scan, changes);
    let sources: Vec<(&Path, &str)> = candidates
        .iter()
        .map(|candidate| (candidate.path, candidate.shown.as_str()))
        .collect();
    let coverage = match coverage::read(&report, scan.project, &sources) {
        Ok(coverage) => coverage,
        Err(reason) => {
            notices.push(format!("{} skipped: {reason}", Rule::UncoveredNewCode.id()));
            return Vec::new();
        }
    };

    stretches_in(&candidates, &coverage, changes, &scan.settings.policy)
}

fn stretches_in(
    candidates: &[Candidate<'_>],
    coverage: &Coverage,
    changes: &Changes,
    policy: &Policy,
) -> Vec<Finding> {
    let mut found = Vec::new();
    for candidate in candidates {
        let Some(file) = coverage.file(candidate.path) else {
            continue;
        };
        let added = |line: u32| {
            changes.touches(
                candidate.path,
                Span {
                    start_line: line,
                    end_line: line,
                },
            )
        };
        let under_coverage = FileUnderCoverage {
            path: &candidate.shown,
            language: candidate.spec.id,
        };
        found.extend(uncovered::findings(&under_coverage, file, added, policy));
    }

    found
}

fn candidates<'a>(scan: &Scan<'a>, changes: &Changes) -> Vec<Candidate<'a>> {
    let mut candidates = Vec::new();

    for path in scan.paths.iter().filter(|path| changes.covers(path)) {
        let Some(spec) = lang::detect(path) else {
            continue;
        };
        let shown = workspace::display(path, scan.project);
        if spec.is_test_path(Path::new(&shown)) {
            continue;
        }
        let reporting = scan
            .settings
            .policy
            .active_for(spec.id, Rule::UncoveredNewCode)
            .is_some();
        if !reporting {
            continue;
        }

        candidates.push(Candidate { path, shown, spec });
    }

    candidates
}
