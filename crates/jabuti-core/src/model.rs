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
