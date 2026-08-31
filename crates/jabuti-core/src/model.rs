use std::collections::BTreeMap;
use std::ops::Range;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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

impl UnitKind {
    pub fn measured_separately(self) -> bool {
        matches!(
            self,
            Self::File | Self::Module | Self::Type | Self::Function
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub name: Option<String>,
    pub kind: UnitKind,
    pub span: Span,
    pub bytes: Range<usize>,
    pub parameters: u32,
    pub children: Vec<Unit>,
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
pub enum Rule {
    Churn,
    DuplicateBlock,
    Hotspot,
    CognitiveComplexity,
    CyclomaticComplexity,
    FileLines,
    FunctionLines,
    Parameters,
}

impl Rule {
    pub const ALL: [Self; 8] = [
        Self::Churn,
        Self::DuplicateBlock,
        Self::Hotspot,
        Self::CognitiveComplexity,
        Self::CyclomaticComplexity,
        Self::FileLines,
        Self::FunctionLines,
        Self::Parameters,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Churn => "churn",
            Self::DuplicateBlock => "duplicate-block",
            Self::Hotspot => "hotspot",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionEffect {
    Branch,
    Discount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    pub position: usize,
    pub effect: DecisionEffect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fragment {
    pub hash: u64,
    pub span: Span,
    pub bytes: Range<usize>,
    pub nodes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reading {
    pub path: String,
    pub line: u32,
    pub subject: Option<String>,
    pub kind: UnitKind,
    pub values: BTreeMap<&'static str, u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Increment {
    pub position: usize,
    pub amount: u32,
}
