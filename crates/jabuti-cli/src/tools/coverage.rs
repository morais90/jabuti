use std::fs;
use std::path::Path;
use std::time::SystemTime;

use jabuti_core::lang::{self, LangSpec};
use jabuti_core::model::{Finding, Rule, Severity, Span};
use jabuti_core::tools::coverage::{self, Coverage, FileUnderCoverage, Format};

use super::Scan;
use crate::git::since::Changes;
use crate::project;

struct Candidate<'a> {
    path: &'a Path,
    shown: String,
    spec: &'static LangSpec,
}

pub(crate) fn findings(scan: &Scan<'_>, changes: &Changes, report: Option<&Path>) -> Vec<Finding> {
    if !scan.settings.enabled(Rule::UncoveredNewCode) {
        return Vec::new();
    }
    let Some(report) = report else {
        eprintln!(
            "jabuti: {} needs a coverage report; set [coverage] report or enable a tool that produces one",
            Rule::UncoveredNewCode.id()
        );
        return Vec::new();
    };

    let candidates = candidates(scan, changes);
    let coverage = match read(report, scan.project, &candidates) {
        Ok(coverage) => coverage,
        Err(reason) => {
            eprintln!("jabuti: {} skipped: {reason}", Rule::UncoveredNewCode.id());
            return Vec::new();
        }
    };

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
        found.extend(coverage::findings(
            &under_coverage,
            file,
            added,
            &scan.settings.policy,
        ));
    }

    found
}

fn candidates<'a>(scan: &Scan<'a>, changes: &Changes) -> Vec<Candidate<'a>> {
    let mut candidates = Vec::new();

    for path in scan.paths.iter().filter(|path| changes.covers(path)) {
        let Some(spec) = lang::detect(path) else {
            continue;
        };
        let shown = project::display(path, scan.project);
        if spec.is_test_path(Path::new(&shown)) {
            continue;
        }
        let reporting = scan
            .settings
            .policy
            .config_for(spec.id, Rule::UncoveredNewCode)
            .is_some_and(|config| config.severity != Severity::Off);
        if !reporting {
            continue;
        }

        candidates.push(Candidate { path, shown, spec });
    }

    candidates
}

fn read(report: &Path, project: &Path, candidates: &[Candidate<'_>]) -> Result<Coverage, String> {
    let shown = project::display(report, project);
    let Some(format) = Format::of(report) else {
        return Err(format!("{shown} has an extension jabuti cannot read"));
    };
    let written = modified(report).map_err(|error| format!("{shown} {error}"))?;

    for candidate in candidates {
        let changed =
            modified(candidate.path).map_err(|error| format!("{} {error}", candidate.shown))?;
        if written < changed {
            return Err(format!("{shown} is older than {}", candidate.shown));
        }
    }

    let text = fs::read_to_string(report).map_err(|error| format!("{shown} {error}"))?;

    Coverage::parse(format, &text).map_err(|error| format!("{shown} {error}"))
}

fn modified(path: &Path) -> std::io::Result<SystemTime> {
    fs::metadata(path)?.modified()
}
