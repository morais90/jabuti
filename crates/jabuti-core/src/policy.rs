use std::collections::BTreeMap;

use crate::metrics::{CognitiveIndex, DecisionIndex, LineIndex};
use crate::model::{Finding, Rule, Severity, Unit, UnitKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleConfig {
    pub limit: u32,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    rules: BTreeMap<Rule, RuleConfig>,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            rules: BTreeMap::from([
                (
                    Rule::CognitiveComplexity,
                    RuleConfig {
                        limit: 7,
                        severity: Severity::Warning,
                    },
                ),
                (
                    Rule::Parameters,
                    RuleConfig {
                        limit: 4,
                        severity: Severity::Warning,
                    },
                ),
                (
                    Rule::CyclomaticComplexity,
                    RuleConfig {
                        limit: 10,
                        severity: Severity::Off,
                    },
                ),
                (
                    Rule::FunctionLines,
                    RuleConfig {
                        limit: 60,
                        severity: Severity::Warning,
                    },
                ),
                (
                    Rule::FileLines,
                    RuleConfig {
                        limit: 1000,
                        severity: Severity::Off,
                    },
                ),
            ]),
        }
    }
}

impl Policy {
    pub fn set(&mut self, rule: Rule, config: RuleConfig) {
        self.rules.insert(rule, config);
    }

    pub fn config(&self, rule: Rule) -> Option<RuleConfig> {
        self.rules.get(&rule).copied()
    }

    pub fn evaluate(&self, file: &FileUnderReview<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();

        self.check(Rule::FileLines, file, &file.units, &mut findings);
        self.walk(file, &file.units, &mut findings);

        findings.sort_by(|left, right| {
            left.span
                .start_line
                .cmp(&right.span.start_line)
                .then(left.rule.cmp(&right.rule))
        });
        findings
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
        let Some(config) = self.rules.get(&rule) else {
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
            rule,
            severity: config.severity,
            path: file.path.clone(),
            span: unit.span,
            subject: unit.name.clone(),
            measured,
            limit: config.limit,
        });
    }
}

#[derive(Debug)]
pub struct FileUnderReview<'a> {
    pub path: String,
    pub units: Unit,
    pub lines: &'a LineIndex,
    pub decisions: &'a DecisionIndex,
    pub cognitive: &'a CognitiveIndex,
}

impl FileUnderReview<'_> {
    fn measure(&self, rule: Rule, unit: &Unit) -> u32 {
        match rule {
            Rule::CognitiveComplexity => self.cognitive.cognitive(unit),
            Rule::Parameters => unit.parameters,
            Rule::CyclomaticComplexity => self.decisions.cyclomatic(unit),
            Rule::FileLines | Rule::FunctionLines => self.lines.loc(unit.span).total,
        }
    }
}
