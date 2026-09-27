use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::Serialize;

use crate::catalog::{RuleId, Severity, UnitKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Span {
    pub start_line: u32,
    pub end_line: u32,
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
