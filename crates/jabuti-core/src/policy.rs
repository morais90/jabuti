use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::catalog::{Concept, Rule, RuleId, RuleSpec, Severity};
use crate::lang::{self, LanguageId};
use crate::model::Finding;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleConfig {
    pub limit: u32,
    pub severity: Severity,
}

impl RuleConfig {
    fn reports(self) -> bool {
        self.severity != Severity::Off
    }
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

fn shared_defaults() -> BTreeMap<RuleId, RuleConfig> {
    Rule::ALL
        .into_iter()
        .map(|rule| {
            let spec = rule.spec();
            (RuleId::Native(rule), default_config(spec, spec.limit))
        })
        .collect()
}

fn language_defaults() -> BTreeMap<(LanguageId, RuleId), RuleConfig> {
    let mut defaults = BTreeMap::new();

    for rule in Rule::ALL {
        let spec = rule.spec();
        for &(language, limit) in spec.language_limits {
            defaults.insert(
                (language, RuleId::Native(rule)),
                default_config(spec, limit),
            );
        }
    }

    defaults
}

fn default_config(spec: RuleSpec, limit: u32) -> RuleConfig {
    RuleConfig {
        limit,
        severity: spec.severity,
    }
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

    pub fn active(&self, rule: impl Into<RuleId>) -> Option<RuleConfig> {
        self.config(rule).filter(|config| config.reports())
    }

    pub fn active_for(&self, language: LanguageId, rule: impl Into<RuleId>) -> Option<RuleConfig> {
        self.config_for(language, rule)
            .filter(|config| config.reports())
    }

    pub fn enabled(&self, rule: Rule) -> bool {
        self.anywhere(rule, |severity| severity != Severity::Off)
    }

    pub fn gates(&self, rule: Rule) -> bool {
        self.anywhere(rule, |severity| severity == Severity::Error)
    }

    fn anywhere(&self, rule: Rule, holds: fn(Severity) -> bool) -> bool {
        lang::ALL.iter().any(|spec| {
            self.config_for(spec.id, rule)
                .is_some_and(|config| holds(config.severity))
        })
    }

    pub fn admit(&self, finding: Finding) -> Option<Finding> {
        let configured = match lang::detect(Path::new(&finding.path)) {
            Some(spec) => self.config_for(spec.id, finding.rule.clone()),
            None => self.config(finding.rule.clone()),
        };

        match configured {
            Some(config) if !config.reports() => None,
            Some(config) => Some(Finding {
                severity: config.severity,
                ..finding
            }),
            None => Some(finding),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConceptBindings {
    paths: BTreeMap<(LanguageId, Concept), BTreeSet<String>>,
}

impl ConceptBindings {
    pub fn set(&mut self, language: LanguageId, concept: Concept, paths: Vec<String>) {
        self.paths
            .entry((language, concept))
            .or_default()
            .extend(paths);
    }

    pub fn paths(&self, language: LanguageId, concept: Concept) -> Option<&BTreeSet<String>> {
        self.paths.get(&(language, concept))
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn is_empty_for(&self, language: LanguageId) -> bool {
        !self.paths.keys().any(|(bound, _)| *bound == language)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Layers {
    pub of: BTreeMap<PathBuf, String>,
    pub allowed: BTreeMap<String, BTreeSet<String>>,
}
