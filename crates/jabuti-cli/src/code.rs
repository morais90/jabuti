use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use jabuti_core::code::duplication::{self, FileFragments};
use jabuti_core::code::metrics::{self, CognitiveIndex, DecisionIndex, LineIndex};
use jabuti_core::code::review::{self, FileUnderReview};
use jabuti_core::code::units::{self, Unit};
use jabuti_core::code::{assertion, concepts, masking, suppression};
use jabuti_core::model::{
    ConceptBindings, Finding, Reading, Rule, Severity, Span, UnitKind, Unreadable,
};
use jabuti_core::policy::Policy;
use jabuti_core::report::Scanned;
use jabuti_core::syntax::Parsed;

use crate::corpus::Text;
use crate::git::since::Changes;

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

#[derive(Debug)]
pub(crate) struct Reviewed {
    path: PathBuf,
    findings: Vec<Finding>,
    readings: Vec<Reading>,
    fragments: Option<FileFragments>,
    units: usize,
    measured: Measured,
}

#[derive(Debug)]
pub(crate) struct Scan<'a> {
    pub(crate) policy: &'a Policy,
    pub(crate) bindings: &'a ConceptBindings,
    pub(crate) changes: Option<&'a Changes>,
    pub(crate) churn: &'a BTreeMap<PathBuf, u32>,
}

impl Scan<'_> {
    pub(crate) fn scope(&self, paths: &[PathBuf]) -> BTreeSet<PathBuf> {
        let mut scope: BTreeSet<PathBuf> = paths.iter().cloned().collect();
        if let (Some(changes), None) = (self.changes, duplication_limit(self.policy)) {
            scope.retain(|path| changes.covers(path));
        }

        scope
    }
}

pub(crate) fn review(
    text: &Text,
    parsed: &Parsed<'_>,
    aliases: &BTreeMap<String, String>,
    request: &Scan<'_>,
) -> Reviewed {
    let lines = LineIndex::new(&text.source, &metrics::comment_ranges(parsed));
    let decisions = DecisionIndex::new(&metrics::decisions(parsed));
    let cognitive = CognitiveIndex::new(&metrics::increments(parsed));
    let units = units::units(parsed);
    let counted = count_units(&units);

    let file = FileUnderReview {
        path: text.shown.clone(),
        language: text.spec.id,
        units,
        lines: &lines,
        decisions: &decisions,
        cognitive: &cognitive,
        churn: request.churn.get(&text.path).copied().unwrap_or(0),
    };

    let measured = Measured {
        path: file.path.clone(),
        span: file.units.span,
        churn: file.churn,
        complexity: cognitive.total(&file.units),
    };

    let mut findings = review::evaluate(request.policy, &file);
    findings.extend(concept_findings(
        text,
        parsed,
        aliases,
        request,
        &file.units,
    ));
    let findings = scoped(findings, &text.path, request.changes);

    Reviewed {
        path: text.path.clone(),
        readings: review::read(&file),
        fragments: duplication_limit(request.policy).map(|minimum| FileFragments {
            path: file.path.clone(),
            fragments: duplication::fragments(parsed, minimum),
        }),
        findings,
        units: counted,
        measured,
    }
}

pub(crate) fn scan(reviewed: Vec<Reviewed>, request: &Scan<'_>) -> Outcome {
    let measured: Vec<Measured> = reviewed.iter().map(|file| file.measured.clone()).collect();

    let repeated: Vec<FileFragments> = reviewed
        .iter()
        .filter_map(|file| file.fragments.clone())
        .collect();

    let mut outcome = gather(covered(reviewed, request.changes));
    outcome.findings.extend(
        duplication::duplicates(&repeated, request.policy)
            .into_iter()
            .filter(|finding| in_diff(finding, request.changes)),
    );
    outcome.measured = measured;

    outcome
}

fn gather(reviewed: Vec<Reviewed>) -> Outcome {
    let mut outcome = Outcome::default();

    for file in reviewed {
        outcome.scanned.files += 1;
        outcome.scanned.units += file.units;
        outcome.findings.extend(file.findings);
        outcome.readings.extend(file.readings);
    }

    outcome
}

fn covered(reviewed: Vec<Reviewed>, changes: Option<&Changes>) -> Vec<Reviewed> {
    let Some(changes) = changes else {
        return reviewed;
    };

    reviewed
        .into_iter()
        .filter(|file| changes.covers(&file.path))
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

fn scoped(mut findings: Vec<Finding>, path: &Path, changes: Option<&Changes>) -> Vec<Finding> {
    findings.sort_by_key(|finding| finding.span.start_line);
    if let Some(changes) = changes {
        findings.retain(|finding| changes.touches(path, finding.span));
    }

    findings
}

fn concept_findings(
    text: &Text,
    parsed: &Parsed<'_>,
    aliases: &BTreeMap<String, String>,
    request: &Scan<'_>,
    units: &Unit,
) -> Vec<Finding> {
    let occurrences = concepts::occurrences(parsed, request.bindings, aliases);

    let mut findings =
        suppression::findings(&text.shown, text.spec.id, &occurrences, request.policy);
    findings.extend(assertion::findings(
        &assertion::Input {
            path: &text.shown,
            parsed,
            unit: units,
            occurrences: &occurrences,
        },
        request.policy,
    ));

    if !text.spec.is_test_path(Path::new(&text.shown)) {
        findings.extend(masking::findings(
            &text.shown,
            text.spec.id,
            &occurrences,
            request.policy,
        ));
    }

    findings
}

fn count_units(unit: &Unit) -> usize {
    let counted = usize::from(unit.kind != UnitKind::File);

    counted + unit.children.iter().map(count_units).sum::<usize>()
}
