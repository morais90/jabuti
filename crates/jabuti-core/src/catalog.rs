use serde::Serialize;

use crate::lang::LanguageId;

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
            Self::Assertion => ASSERTION,
            Self::Churn => CHURN,
            Self::DuplicateBlock => DUPLICATE_BLOCK,
            Self::ErrorMasking => ERROR_MASKING,
            Self::Hotspot => HOTSPOT,
            Self::LayerViolation => LAYER_VIOLATION,
            Self::NewDependency => NEW_DEPENDENCY,
            Self::SpeculativeApi => SPECULATIVE_API,
            Self::Suppression => SUPPRESSION,
            Self::UncoveredNewCode => UNCOVERED_NEW_CODE,
            Self::CognitiveComplexity => COGNITIVE_COMPLEXITY,
            Self::CyclomaticComplexity => CYCLOMATIC_COMPLEXITY,
            Self::FileLines => FILE_LINES,
            Self::FunctionLines => FUNCTION_LINES,
            Self::Parameters => PARAMETERS,
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

const ASSERTION: RuleSpec = concept_bound("assertion", &[Concept::Assertion]);

const CHURN: RuleSpec = RuleSpec {
    inputs: &[Input::History],
    threshold: Some(on(Measure::Churn, UnitKind::File)),
    ..universal("churn", Severity::Off, 0)
};

const DUPLICATE_BLOCK: RuleSpec = RuleSpec {
    repository_wide: true,
    ..universal("duplicate-block", Severity::Warning, 120)
};

const ERROR_MASKING: RuleSpec = concept_bound(
    "error-masking",
    &[
        Concept::ErrorDiscard,
        Concept::ErrorPanic,
        Concept::ErrorSwallow,
    ],
);

const HOTSPOT: RuleSpec = RuleSpec {
    repository_wide: true,
    scoping: Scoping::Repository,
    inputs: &[Input::History],
    ..universal("hotspot", Severity::Warning, 90)
};

const LAYER_VIOLATION: RuleSpec = RuleSpec {
    repository_wide: true,
    inputs: &[Input::Graph, Input::Layers],
    ..universal("layer-violation", Severity::Warning, 0)
};

const NEW_DEPENDENCY: RuleSpec = compared("new-dependency");

const SPECULATIVE_API: RuleSpec = compared("speculative-api");

const SUPPRESSION: RuleSpec = concept_bound("suppression", &[Concept::Suppression]);

const UNCOVERED_NEW_CODE: RuleSpec = RuleSpec {
    scoping: Scoping::Change,
    ..universal("uncovered-new-code", Severity::Off, 0)
};

const COGNITIVE_COMPLEXITY: RuleSpec = RuleSpec {
    language_limits: &[(LanguageId::TypeScript, 18)],
    threshold: Some(on(Measure::CognitiveComplexity, UnitKind::Function)),
    ..universal("cognitive-complexity", Severity::Warning, 7)
};

const CYCLOMATIC_COMPLEXITY: RuleSpec = RuleSpec {
    language_limits: &[(LanguageId::TypeScript, 13)],
    threshold: Some(on(Measure::CyclomaticComplexity, UnitKind::Function)),
    ..universal("cyclomatic-complexity", Severity::Off, 10)
};

const FILE_LINES: RuleSpec = RuleSpec {
    threshold: Some(on(Measure::Lines, UnitKind::File)),
    ..universal("file-lines", Severity::Off, 1000)
};

const FUNCTION_LINES: RuleSpec = RuleSpec {
    language_limits: &[(LanguageId::Kotlin, 47), (LanguageId::TypeScript, 71)],
    threshold: Some(on(Measure::Lines, UnitKind::Function)),
    ..universal("function-lines", Severity::Warning, 60)
};

const PARAMETERS: RuleSpec = RuleSpec {
    threshold: Some(on(Measure::Parameters, UnitKind::Function)),
    ..universal("parameters", Severity::Warning, 4)
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Measure {
    Churn,
    CognitiveComplexity,
    CyclomaticComplexity,
    Lines,
    Parameters,
}

impl Measure {
    pub const ALL: [Self; 5] = [
        Self::Churn,
        Self::CognitiveComplexity,
        Self::CyclomaticComplexity,
        Self::Lines,
        Self::Parameters,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Churn => "churn",
            Self::CognitiveComplexity => "cognitive-complexity",
            Self::CyclomaticComplexity => "cyclomatic-complexity",
            Self::Lines => "lines",
            Self::Parameters => "parameters",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Threshold {
    pub measure: Measure,
    pub unit: UnitKind,
}

const fn on(measure: Measure, unit: UnitKind) -> Threshold {
    Threshold { measure, unit }
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
    pub threshold: Option<Threshold>,
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
        threshold: None,
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
