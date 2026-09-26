use std::collections::BTreeSet;

use super::metrics::{CognitiveIndex, DecisionIndex, LineIndex};
use super::units::Unit;
use crate::lang::LanguageId;
use crate::model::{Detail, Finding, Measure, Reading, Rule, RuleId, UnitKind};
use crate::policy::Policy;

#[derive(Debug)]
pub struct FileUnderReview<'a> {
    pub path: String,
    pub language: LanguageId,
    pub units: Unit,
    pub lines: &'a LineIndex,
    pub decisions: &'a DecisionIndex,
    pub cognitive: &'a CognitiveIndex,
    pub churn: u32,
}

pub fn evaluate(policy: &Policy, file: &FileUnderReview<'_>) -> Vec<Finding> {
    let judged = Judged { policy, file };
    let mut findings = Vec::new();

    judged.walk(&file.units, &mut findings);

    findings.sort_by(|left, right| {
        left.span
            .start_line
            .cmp(&right.span.start_line)
            .then(left.rule.cmp(&right.rule))
    });
    findings
}

pub fn read(file: &FileUnderReview<'_>) -> Vec<Reading> {
    let mut readings = Vec::new();
    gather_readings(file, &file.units, &mut readings);

    readings
}

struct Judged<'a> {
    policy: &'a Policy,
    file: &'a FileUnderReview<'a>,
}

impl Judged<'_> {
    fn walk(&self, unit: &Unit, findings: &mut Vec<Finding>) {
        for (rule, measure) in thresholds_on(unit.kind) {
            self.check(rule, measure, unit, findings);
        }

        for child in &unit.children {
            self.walk(child, findings);
        }
    }

    fn check(&self, rule: Rule, measure: Measure, unit: &Unit, findings: &mut Vec<Finding>) {
        let Some(config) = self.policy.active_for(self.file.language, rule) else {
            return;
        };

        let measured = self.file.measure(measure, unit);
        if measured <= config.limit {
            return;
        }

        findings.push(Finding {
            rule: RuleId::Native(rule),
            severity: config.severity,
            path: self.file.path.clone(),
            span: unit.span,
            subject: unit.name.clone(),
            detail: Detail::Threshold {
                measured,
                limit: config.limit,
            },
        });
    }
}

fn thresholds_on(kind: UnitKind) -> impl Iterator<Item = (Rule, Measure)> {
    Rule::ALL.into_iter().filter_map(move |rule| {
        rule.spec()
            .threshold
            .filter(|threshold| threshold.unit == kind)
            .map(|threshold| (rule, threshold.measure))
    })
}

fn gather_readings(file: &FileUnderReview<'_>, unit: &Unit, readings: &mut Vec<Reading>) {
    let measures: BTreeSet<Measure> = thresholds_on(unit.kind)
        .map(|(_, measure)| measure)
        .collect();
    if !measures.is_empty() {
        readings.push(reading(file, unit, &measures));
    }

    for child in &unit.children {
        gather_readings(file, child, readings);
    }
}

fn reading(file: &FileUnderReview<'_>, unit: &Unit, measures: &BTreeSet<Measure>) -> Reading {
    Reading {
        path: file.path.clone(),
        line: unit.span.start_line,
        subject: unit.name.clone(),
        kind: unit.kind,
        values: measures
            .iter()
            .map(|measure| (measure.id(), file.measure(*measure, unit)))
            .collect(),
    }
}

impl FileUnderReview<'_> {
    fn measure(&self, measure: Measure, unit: &Unit) -> u32 {
        match measure {
            Measure::Churn => self.churn,
            Measure::CognitiveComplexity => self.cognitive.cognitive(unit),
            Measure::CyclomaticComplexity => self.decisions.cyclomatic(unit),
            Measure::Lines => self.lines.loc(unit.span).total,
            Measure::Parameters => unit.parameters,
        }
    }
}
