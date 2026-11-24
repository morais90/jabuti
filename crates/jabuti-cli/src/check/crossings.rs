use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use jabuti_core::catalog::Rule;
use jabuti_core::code::Measured;
use jabuti_core::crossings::hotspot::{self, FileSummary};
use jabuti_core::crossings::uncovered::{self, FileUnderCoverage};
use jabuti_core::lang::{self, LangSpec};
use jabuti_core::model::{Finding, Span};
use jabuti_core::policy::Policy;
use jabuti_core::tools::coverage::Coverage;

use super::plan::Plan;
use crate::inputs::git::since::Changes;
use crate::inputs::tools::{self, Produced, Scan};
use crate::inputs::{coverage, workspace};

pub(crate) fn hotspots(plan: Plan<'_>, measured: &[Measured], policy: &Policy) -> Vec<Finding> {
    if !plan.runs(Rule::Hotspot) {
        return Vec::new();
    }

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

#[derive(Debug, Clone, Copy)]
pub(crate) struct Judged<'a> {
    pub(crate) changes: &'a Changes,
    pub(crate) measured: &'a [Measured],
}

pub(crate) fn uncovered(
    scan: &Scan<'_>,
    judged: Judged<'_>,
    produced: &[Produced],
    notices: &mut Vec<String>,
) -> Vec<Finding> {
    if !scan.settings.policy.enabled(Rule::UncoveredNewCode) {
        return Vec::new();
    }
    let candidates = candidates(scan, judged.changes);
    if candidates.is_empty() {
        return Vec::new();
    }
    let reports: Vec<(&Path, PathBuf)> = match &scan.settings.coverage {
        Some(report) => vec![(scan.project, scan.project.join(report))],
        None => produced
            .iter()
            .map(|made| (made.workspace.as_path(), made.report.clone()))
            .collect(),
    };
    if reports.is_empty() {
        notices.push(unevaluated(scan));
    }

    let mut found = Vec::new();
    for (workspace, report) in reports {
        let inside: Vec<&Candidate<'_>> = candidates
            .iter()
            .filter(|candidate| candidate.path.starts_with(workspace))
            .collect();
        match read_one(&report, &inside, scan, judged) {
            Ok(stretches) => found.extend(stretches),
            Err(reason) => {
                notices.push(format!("{} skipped: {reason}", Rule::UncoveredNewCode.id()));
            }
        }
    }

    found
}

fn unevaluated(scan: &Scan<'_>) -> String {
    let remedy = if tools::produces_coverage(scan.settings) {
        ""
    } else {
        "; enable a coverage tool under [tools] or name a report under [coverage]"
    };

    format!(
        "{} was not evaluated: no coverage report covers the changed files, so nothing says which new lines a test ran{remedy}",
        Rule::UncoveredNewCode.id()
    )
}

fn read_one(
    report: &Path,
    inside: &[&Candidate<'_>],
    scan: &Scan<'_>,
    judged: Judged<'_>,
) -> Result<Vec<Finding>, String> {
    let sources: Vec<(&Path, &str)> = inside
        .iter()
        .map(|candidate| (candidate.path, candidate.shown.as_str()))
        .collect();
    let coverage = coverage::read(report, scan.project, &sources)?;

    Ok(stretches_in(
        inside,
        &coverage,
        judged,
        &scan.settings.policy,
    ))
}

fn stretches_in(
    candidates: &[&Candidate<'_>],
    coverage: &Coverage,
    judged: Judged<'_>,
    policy: &Policy,
) -> Vec<Finding> {
    let compile_times: BTreeMap<&str, &[Span]> = judged
        .measured
        .iter()
        .map(|file| (file.path.as_str(), file.compile_time.as_slice()))
        .collect();
    let mut found = Vec::new();
    for candidate in candidates {
        let Some(file) = coverage.file(candidate.path) else {
            continue;
        };
        let added = |line: u32| {
            judged.changes.touches(
                candidate.path,
                Span {
                    start_line: line,
                    end_line: line,
                },
            )
        };
        let compile_time = compile_times
            .get(candidate.shown.as_str())
            .copied()
            .unwrap_or_default();
        let under_coverage = FileUnderCoverage {
            path: &candidate.shown,
            language: candidate.spec.id,
            compile_time,
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
