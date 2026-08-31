use std::collections::BTreeMap;

use crate::lang::LanguageId;
use crate::metrics::{CognitiveIndex, DecisionIndex, LineIndex};
use crate::model::{Detail, Finding, Reading, Rule, RuleId, Severity, Unit, UnitKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleConfig {
    pub limit: u32,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    rules: BTreeMap<RuleId, RuleConfig>,
    by_language: BTreeMap<(LanguageId, RuleId), RuleConfig>,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            rules: shared_defaults(),
            by_language: language_defaults(),
        }
    }
}

fn reporting(limit: u32) -> RuleConfig {
    RuleConfig {
        limit,
        severity: Severity::Warning,
    }
}

fn silent(limit: u32) -> RuleConfig {
    RuleConfig {
        limit,
        severity: Severity::Off,
    }
}

fn shared_defaults() -> BTreeMap<RuleId, RuleConfig> {
    BTreeMap::from([
        (RuleId::Native(Rule::Churn), silent(0)),
        (RuleId::Native(Rule::CognitiveComplexity), reporting(7)),
        (RuleId::Native(Rule::CyclomaticComplexity), silent(10)),
        (RuleId::Native(Rule::FileLines), silent(1000)),
        (RuleId::Native(Rule::FunctionLines), reporting(60)),
        (RuleId::Native(Rule::Hotspot), reporting(90)),
        (RuleId::Native(Rule::Parameters), reporting(4)),
    ])
}

fn language_defaults() -> BTreeMap<(LanguageId, RuleId), RuleConfig> {
    BTreeMap::from([(
        (LanguageId::Kotlin, RuleId::Native(Rule::FunctionLines)),
        reporting(47),
    )])
}

impl Policy {
    pub fn set(&mut self, rule: impl Into<RuleId>, config: RuleConfig) {
        self.rules.insert(rule.into(), config);
    }

    pub fn config(&self, rule: impl Into<RuleId>) -> Option<RuleConfig> {
        self.rules.get(&rule.into()).copied()
    }

    pub fn set_for(&mut self, language: LanguageId, rule: impl Into<RuleId>, config: RuleConfig) {
        self.by_language.insert((language, rule.into()), config);
    }

    pub fn config_for(&self, language: LanguageId, rule: impl Into<RuleId>) -> Option<RuleConfig> {
        let rule = rule.into();

        self.by_language
            .get(&(language, rule.clone()))
            .or_else(|| self.rules.get(&rule))
            .copied()
    }

    pub fn admit(&self, finding: Finding) -> Option<Finding> {
        match self.rules.get(&finding.rule) {
            Some(config) if config.severity == Severity::Off => None,
            Some(config) => Some(Finding {
                severity: config.severity,
                ..finding
            }),
            None => Some(finding),
        }
    }

    pub fn evaluate(&self, file: &FileUnderReview<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();

        self.check(Rule::FileLines, file, &file.units, &mut findings);
        self.check(Rule::Churn, file, &file.units, &mut findings);
        self.walk(file, &file.units, &mut findings);

        findings.sort_by(|left, right| {
            left.span
                .start_line
                .cmp(&right.span.start_line)
                .then(left.rule.cmp(&right.rule))
        });
        findings
    }

    pub fn read(&self, file: &FileUnderReview<'_>) -> Vec<Reading> {
        let mut readings = vec![reading(file, &file.units, &[Rule::FileLines, Rule::Churn])];
        gather_readings(file, &file.units, &mut readings);

        readings
    }

    fn walk(&self, file: &FileUnderReview<'_>, unit: &Unit, findings: &mut Vec<Finding>) {
        if unit.kind == UnitKind::Function {
            self.check(Rule::FunctionLines, file, unit, findings);
            self.check(Rule::CyclomaticComplexity, file, unit, findings);
            self.check(Rule::CognitiveComplexity, file, unit, findings);
            self.check(Rule::Parameters, file, unit, findings);
        }

        for child in &unit.children {
            self.walk(file, child, findings);
        }
    }

    fn check(
        &self,
        rule: Rule,
        file: &FileUnderReview<'_>,
        unit: &Unit,
        findings: &mut Vec<Finding>,
    ) {
        let Some(config) = self.config_for(file.language, rule) else {
            return;
        };
        if config.severity == Severity::Off {
            return;
        }

        let measured = file.measure(rule, unit);
        if measured <= config.limit {
            return;
        }

        findings.push(Finding {
            rule: RuleId::Native(rule),
            severity: config.severity,
            path: file.path.clone(),
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

impl FileUnderReview<'_> {
    pub(crate) fn measure(&self, rule: Rule, unit: &Unit) -> u32 {
        match rule {
            Rule::Churn => self.churn,
            Rule::Hotspot => 0,
            Rule::CognitiveComplexity => self.cognitive.cognitive(unit),
            Rule::Parameters => unit.parameters,
            Rule::CyclomaticComplexity => self.decisions.cyclomatic(unit),
            Rule::FileLines | Rule::FunctionLines => self.lines.loc(unit.span).total,
        }
    }
}
