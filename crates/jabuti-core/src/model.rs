use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub children: Vec<Unit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    CognitiveComplexity,
    CyclomaticComplexity,
    FileLines,
    FunctionLines,
}

impl Rule {
    pub const ALL: [Self; 4] = [
        Self::CognitiveComplexity,
        Self::CyclomaticComplexity,
        Self::FileLines,
        Self::FunctionLines,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::CognitiveComplexity => "cognitive-complexity",
            Self::CyclomaticComplexity => "cyclomatic-complexity",
            Self::FileLines => "file-lines",
            Self::FunctionLines => "function-lines",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|rule| rule.id() == id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule: Rule,
    pub severity: Severity,
    pub path: String,
    pub span: Span,
    pub subject: Option<String>,
    pub measured: u32,
    pub limit: u32,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Increment {
    pub position: usize,
    pub amount: u32,
}
