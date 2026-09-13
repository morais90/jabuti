use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::lang::LanguageId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Span {
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UnitKind {
    File,
    Module,
    Type,
    Function,
    Closure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Off,
    Warning,
    Error,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Portability {
    Universal,
    ConceptBound,
    LanguageSpecific,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Concept {
    ErrorDiscard,
    ErrorPanic,
    ErrorSwallow,
    Suppression,
    Assertion,
}

impl Concept {
    pub const ALL: [Self; 5] = [
        Self::ErrorDiscard,
        Self::ErrorPanic,
        Self::ErrorSwallow,
        Self::Suppression,
        Self::Assertion,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::ErrorDiscard => "error-discard",
            Self::ErrorPanic => "error-panic",
            Self::ErrorSwallow => "error-swallow",
            Self::Suppression => "suppression",
            Self::Assertion => "assertion",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|concept| concept.id() == id)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Rule {
    Assertion,
    Churn,
    DuplicateBlock,
    ErrorMasking,
    Hotspot,
    LayerViolation,
    NewDependency,
    SpeculativeApi,
    Suppression,
    UncoveredNewCode,
    CognitiveComplexity,
    CyclomaticComplexity,
    FileLines,
    FunctionLines,
    Parameters,
}

impl Rule {
    pub const ALL: [Self; 15] = [
        Self::Assertion,
        Self::Churn,
        Self::DuplicateBlock,
        Self::ErrorMasking,
        Self::Hotspot,
        Self::LayerViolation,
        Self::NewDependency,
        Self::SpeculativeApi,
        Self::Suppression,
        Self::UncoveredNewCode,
        Self::CognitiveComplexity,
        Self::CyclomaticComplexity,
        Self::FileLines,
        Self::FunctionLines,
        Self::Parameters,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Assertion => "assertion",
            Self::Churn => "churn",
            Self::DuplicateBlock => "duplicate-block",
            Self::ErrorMasking => "error-masking",
            Self::Hotspot => "hotspot",
            Self::LayerViolation => "layer-violation",
            Self::NewDependency => "new-dependency",
            Self::SpeculativeApi => "speculative-api",
            Self::Suppression => "suppression",
            Self::UncoveredNewCode => "uncovered-new-code",
            Self::CognitiveComplexity => "cognitive-complexity",
            Self::CyclomaticComplexity => "cyclomatic-complexity",
            Self::FileLines => "file-lines",
            Self::FunctionLines => "function-lines",
            Self::Parameters => "parameters",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|rule| rule.id() == id)
    }

    pub fn portability(self) -> Portability {
        match self {
            Self::ErrorMasking | Self::Suppression | Self::Assertion => Portability::ConceptBound,
            Self::Churn
            | Self::DuplicateBlock
            | Self::Hotspot
            | Self::LayerViolation
            | Self::NewDependency
            | Self::SpeculativeApi
            | Self::UncoveredNewCode
            | Self::CognitiveComplexity
            | Self::CyclomaticComplexity
            | Self::FileLines
            | Self::FunctionLines
            | Self::Parameters => Portability::Universal,
        }
    }

    pub fn repository_wide(self) -> bool {
        matches!(
            self,
            Self::DuplicateBlock | Self::Hotspot | Self::LayerViolation
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(into = "String")]
pub enum RuleId {
    Native(Rule),
    External { tool: String, lint: String },
}

impl RuleId {
    pub fn id(&self) -> String {
        match self {
            Self::Native(rule) => rule.id().to_owned(),
            Self::External { tool, lint } => format!("{tool}/{lint}"),
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        match id.split_once('/') {
            Some((tool, lint)) if !tool.is_empty() && !lint.is_empty() => Some(Self::External {
                tool: tool.to_owned(),
                lint: lint.to_owned(),
            }),
            Some(_) => None,
            None => Rule::from_id(id).map(Self::Native),
        }
    }
}

impl From<RuleId> for String {
    fn from(rule: RuleId) -> Self {
        rule.id()
    }
}

impl From<Rule> for RuleId {
    fn from(rule: Rule) -> Self {
        Self::Native(rule)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Detail {
    Threshold { measured: u32, limit: u32 },
    Message { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub rule: RuleId,
    pub severity: Severity,
    pub path: String,
    pub span: Span,
    pub subject: Option<String>,
    pub detail: Detail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reading {
    pub path: String,
    pub line: u32,
    pub subject: Option<String>,
    pub kind: UnitKind,
    pub values: BTreeMap<&'static str, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Unreadable {
    pub path: String,
    pub reason: String,
}
