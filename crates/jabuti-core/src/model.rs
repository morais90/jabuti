use std::cmp::Ordering;
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

    pub fn spec(self) -> RuleSpec {
        match self {
            Self::Assertion => concept_bound("assertion", &[Concept::Assertion]),
            Self::Churn => RuleSpec {
                inputs: &[Input::History],
                ..universal("churn", Severity::Off, 0)
            },
            Self::DuplicateBlock => RuleSpec {
                repository_wide: true,
                ..universal("duplicate-block", Severity::Warning, 120)
            },
            Self::ErrorMasking => concept_bound(
                "error-masking",
                &[
                    Concept::ErrorDiscard,
                    Concept::ErrorPanic,
                    Concept::ErrorSwallow,
                ],
            ),
            Self::Hotspot => RuleSpec {
                repository_wide: true,
                scoping: Scoping::Repository,
                inputs: &[Input::History],
                ..universal("hotspot", Severity::Warning, 90)
            },
            Self::LayerViolation => RuleSpec {
                repository_wide: true,
                inputs: &[Input::Graph, Input::Layers],
                ..universal("layer-violation", Severity::Warning, 0)
            },
            Self::NewDependency => compared("new-dependency"),
            Self::SpeculativeApi => compared("speculative-api"),
            Self::Suppression => concept_bound("suppression", &[Concept::Suppression]),
            Self::UncoveredNewCode => RuleSpec {
                scoping: Scoping::Change,
                ..universal("uncovered-new-code", Severity::Off, 0)
            },
            Self::CognitiveComplexity => RuleSpec {
                language_limits: &[(LanguageId::TypeScript, 18)],
                ..universal("cognitive-complexity", Severity::Warning, 7)
            },
            Self::CyclomaticComplexity => RuleSpec {
                language_limits: &[(LanguageId::TypeScript, 13)],
                ..universal("cyclomatic-complexity", Severity::Off, 10)
            },
            Self::FileLines => universal("file-lines", Severity::Off, 1000),
            Self::FunctionLines => RuleSpec {
                language_limits: &[(LanguageId::Kotlin, 47), (LanguageId::TypeScript, 71)],
                ..universal("function-lines", Severity::Warning, 60)
            },
            Self::Parameters => universal("parameters", Severity::Warning, 4),
        }
    }

    pub fn id(self) -> &'static str {
        self.spec().id
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|rule| rule.id() == id)
    }

    pub fn portability(self) -> Portability {
        self.spec().portability
    }

    pub fn repository_wide(self) -> bool {
        self.spec().repository_wide
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scoping {
    Any,
    Change,
    Repository,
}

impl Scoping {
    pub fn allows(self, scoped: bool) -> bool {
        match self {
            Self::Any => true,
            Self::Change => scoped,
            Self::Repository => !scoped,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    BaseRevision,
    Graph,
    History,
    Layers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleSpec {
    pub id: &'static str,
    pub portability: Portability,
    pub repository_wide: bool,
    pub scoping: Scoping,
    pub inputs: &'static [Input],
    pub severity: Severity,
    pub limit: u32,
    pub language_limits: &'static [(LanguageId, u32)],
    pub concepts: &'static [Concept],
}

const fn universal(id: &'static str, severity: Severity, limit: u32) -> RuleSpec {
    RuleSpec {
        id,
        portability: Portability::Universal,
        repository_wide: false,
        scoping: Scoping::Any,
        inputs: &[],
        severity,
        limit,
        language_limits: &[],
        concepts: &[],
    }
}

const fn compared(id: &'static str) -> RuleSpec {
    RuleSpec {
        scoping: Scoping::Change,
        inputs: &[Input::BaseRevision, Input::Graph],
        ..universal(id, Severity::Warning, 0)
    }
}

const fn concept_bound(id: &'static str, concepts: &'static [Concept]) -> RuleSpec {
    RuleSpec {
        portability: Portability::ConceptBound,
        concepts,
        ..universal(id, Severity::Warning, 0)
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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

impl Ord for Finding {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path
            .cmp(&other.path)
            .then(self.span.cmp(&other.span))
            .then(self.rule.cmp(&other.rule))
            .then(self.subject.cmp(&other.subject))
            .then(self.detail.cmp(&other.detail))
            .then(self.severity.cmp(&other.severity))
    }
}

impl PartialOrd for Finding {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
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
