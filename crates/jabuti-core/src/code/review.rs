use super::metrics::{CognitiveIndex, DecisionIndex, LineIndex};
use super::units::Unit;
use crate::lang::LanguageId;
use crate::model::{Detail, Finding, Reading, Rule, RuleId, Severity, UnitKind};
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

    judged.check(Rule::FileLines, &file.units, &mut findings);
    judged.check(Rule::Churn, &file.units, &mut findings);
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
    let mut readings = vec![reading(file, &file.units, &[Rule::FileLines, Rule::Churn])];
    gather_readings(file, &file.units, &mut readings);

    readings
}

struct Judged<'a> {
    policy: &'a Policy,
    file: &'a FileUnderReview<'a>,
}

impl Judged<'_> {
    fn walk(&self, unit: &Unit, findings: &mut Vec<Finding>) {
        if unit.kind == UnitKind::Function {
            for rule in PER_FUNCTION {
                self.check(rule, unit, findings);
            }
        }

        for child in &unit.children {
            self.walk(child, findings);
        }
    }

    fn check(&self, rule: Rule, unit: &Unit, findings: &mut Vec<Finding>) {
        let Some(config) = self.policy.config_for(self.file.language, rule) else {
            return;
        };
        if config.severity == Severity::Off {
            return;
        }

        let measured = self.file.measure(rule, unit);
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

const PER_FUNCTION: [Rule; 4] = [
    Rule::FunctionLines,
    Rule::CyclomaticComplexity,
    Rule::CognitiveComplexity,
    Rule::Parameters,
];

fn gather_readings(file: &FileUnderReview<'_>, unit: &Unit, readings: &mut Vec<Reading>) {
    if unit.kind == UnitKind::Function {
        readings.push(reading(file, unit, &PER_FUNCTION));
    }

    for child in &unit.children {
        gather_readings(file, child, readings);
    }
}

fn reading(file: &FileUnderReview<'_>, unit: &Unit, rules: &[Rule]) -> Reading {
    Reading {
        path: file.path.clone(),
        line: unit.span.start_line,
        subject: unit.name.clone(),
        kind: unit.kind,
        values: rules
            .iter()
            .map(|rule| (rule.id(), file.measure(*rule, unit)))
            .collect(),
    }
}

const NOT_MEASURED_PER_UNIT: u32 = 0;

impl FileUnderReview<'_> {
    fn measure(&self, rule: Rule, unit: &Unit) -> u32 {
        match rule {
            Rule::Churn => self.churn,
            Rule::DuplicateBlock
            | Rule::ErrorMasking
            | Rule::Hotspot
            | Rule::LayerViolation
            | Rule::NewDependency => NOT_MEASURED_PER_UNIT,
            Rule::CognitiveComplexity => self.cognitive.cognitive(unit),
            Rule::Parameters => unit.parameters,
            Rule::CyclomaticComplexity => self.decisions.cyclomatic(unit),
            Rule::FileLines | Rule::FunctionLines => self.lines.loc(unit.span).total,
        }
    }
}
