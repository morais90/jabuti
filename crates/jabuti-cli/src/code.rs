use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use jabuti_core::catalog::{Rule, UnitKind};
use jabuti_core::code::duplication::{self, FileFragments};
use jabuti_core::code::metrics::{self, CognitiveIndex, DecisionIndex, LineIndex};
use jabuti_core::code::review::{self, FileUnderReview};
use jabuti_core::code::units::{self, Unit};
use jabuti_core::code::{assertion, concepts, masking, suppression};
use jabuti_core::model::{Finding, Reading, Span};
use jabuti_core::policy::{ConceptBindings, Policy};
use jabuti_core::report::Outcome;
use jabuti_core::syntax::Parsed;

use crate::corpus::Text;

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
    pub(crate) touched: Option<&'a BTreeSet<PathBuf>>,
    pub(crate) churn: &'a BTreeMap<PathBuf, u32>,
}

impl Scan<'_> {
    pub(crate) fn scope(&self, paths: &[PathBuf]) -> BTreeSet<PathBuf> {
        let mut scope: BTreeSet<PathBuf> = paths.iter().cloned().collect();
        if let (Some(touched), None) = (self.touched, duplication_limit(self.policy)) {
            scope.retain(|path| touched.contains(path));
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

pub(crate) fn scan(reviewed: Vec<Reviewed>, request: &Scan<'_>) -> (Outcome, Vec<Measured>) {
    let measured: Vec<Measured> = reviewed.iter().map(|file| file.measured.clone()).collect();

    let repeated: Vec<FileFragments> = reviewed
        .iter()
        .filter_map(|file| file.fragments.clone())
        .collect();

    let mut outcome = gather(covered(reviewed, request.touched));
    outcome
        .findings
        .extend(duplication::duplicates(&repeated, request.policy));

    (outcome, measured)
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

fn covered(reviewed: Vec<Reviewed>, touched: Option<&BTreeSet<PathBuf>>) -> Vec<Reviewed> {
    let Some(touched) = touched else {
        return reviewed;
    };

    reviewed
        .into_iter()
        .filter(|file| touched.contains(&file.path))
        .collect()
}

fn duplication_limit(policy: &Policy) -> Option<u32> {
    policy
        .active(Rule::DuplicateBlock)
        .map(|config| config.limit)
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
