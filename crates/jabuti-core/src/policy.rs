use std::collections::BTreeMap;

use crate::lang::LanguageId;
use crate::model::{Finding, Rule, RuleId, Severity};

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
        (RuleId::Native(Rule::DuplicateBlock), reporting(120)),
        (RuleId::Native(Rule::ErrorMasking), reporting(0)),
        (RuleId::Native(Rule::CognitiveComplexity), reporting(7)),
        (RuleId::Native(Rule::CyclomaticComplexity), silent(10)),
        (RuleId::Native(Rule::FileLines), silent(1000)),
        (RuleId::Native(Rule::FunctionLines), reporting(60)),
        (RuleId::Native(Rule::Hotspot), reporting(90)),
        (RuleId::Native(Rule::LayerViolation), reporting(0)),
        (RuleId::Native(Rule::NewDependency), reporting(0)),
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
}
