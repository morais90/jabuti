use std::cmp::Ordering;
use std::ops::Range;

use super::units::{self, Unit};
use super::{cognitive, lang};
use crate::model::Span;
use crate::syntax::Parsed;

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

pub fn comment_ranges(parsed: &Parsed<'_>) -> Vec<Range<usize>> {
    let table = lang::table(parsed.language());
    let mut ranges = Vec::new();

    parsed.for_each_match(&table.queries().comments, |matched, _| {
        for capture in matched.captures {
            ranges.push(capture.node.byte_range());
        }
    });

    ranges.sort_by_key(|range| range.start);
    ranges
}

pub fn decisions(parsed: &Parsed<'_>) -> Vec<Decision> {
    let table = lang::table(parsed.language());
    let mut decisions = Vec::new();

    parsed.for_each_match(&table.queries().decisions, |matched, query| {
        for capture in matched.captures {
            decisions.push(Decision {
                position: capture.node.start_byte(),
                effect: effect_of_label(query.capture_names()[capture.index as usize]),
            });
        }
    });

    decisions.sort_by_key(|decision| decision.position);
    decisions
}

fn effect_of_label(label: &str) -> DecisionEffect {
    match label {
        "decision" => DecisionEffect::Branch,
        "decision.discount" => DecisionEffect::Discount,
        unknown => panic!("query captures @{unknown}, which carries no decision effect"),
    }
}

pub fn increments(parsed: &Parsed<'_>) -> Vec<Increment> {
    cognitive::increments(parsed.root(), &lang::table(parsed.language()).cognitive)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Loc {
    pub total: u32,
    pub code: u32,
    pub comment: u32,
    pub blank: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineKind {
    Blank,
    Comment,
    Code,
}

#[derive(Debug)]
pub struct LineIndex {
    kinds: Vec<LineKind>,
}

impl LineIndex {
    pub fn new(source: &str, comments: &[Range<usize>]) -> Self {
        let mut ordered = comments.to_vec();
        ordered.sort_by_key(|comment| comment.start);

        let mut kinds = Vec::new();
        let mut line_start = 0;

        for line in source.split_inclusive('\n') {
            kinds.push(classify(line, line_start, &ordered));
            line_start += line.len();
        }

        Self { kinds }
    }

    pub fn loc(&self, span: Span) -> Loc {
        let first = span.start_line.saturating_sub(1) as usize;
        let last = (span.end_line as usize).min(self.kinds.len());
        let counted = self.kinds.get(first..last).unwrap_or_default();

        let mut loc = Loc {
            total: u32::try_from(counted.len()).unwrap_or(u32::MAX),
            code: 0,
            comment: 0,
            blank: 0,
        };

        for kind in counted {
            match kind {
                LineKind::Blank => loc.blank += 1,
                LineKind::Comment => loc.comment += 1,
                LineKind::Code => loc.code += 1,
            }
        }

        loc
    }
}

fn classify(line: &str, line_start: usize, comments: &[Range<usize>]) -> LineKind {
    let mut occupied = false;

    for (offset, character) in line.char_indices() {
        if character.is_whitespace() {
            continue;
        }

        occupied = true;
        if !is_commented(line_start + offset, comments) {
            return LineKind::Code;
        }
    }

    if occupied {
        LineKind::Comment
    } else {
        LineKind::Blank
    }
}

#[derive(Debug)]
pub struct DecisionIndex {
    decisions: Vec<Decision>,
}

impl DecisionIndex {
    pub fn new(decisions: &[Decision]) -> Self {
        let mut ordered = decisions.to_vec();
        ordered.sort_by_key(|decision| decision.position);

        Self { decisions: ordered }
    }

    pub fn cyclomatic(&self, unit: &Unit) -> u32 {
        let own = self.effect_within(&unit.bytes) - self.effect_of_nested_units(unit);

        u32::try_from(own.saturating_add(1)).unwrap_or(1)
    }

    fn effect_within(&self, bytes: &Range<usize>) -> i64 {
        let first = self
            .decisions
            .partition_point(|decision| decision.position < bytes.start);
        let last = self
            .decisions
            .partition_point(|decision| decision.position < bytes.end);

        self.decisions[first..last]
            .iter()
            .map(|decision| match decision.effect {
                DecisionEffect::Branch => 1,
                DecisionEffect::Discount => -1,
            })
            .sum()
    }

    fn effect_of_nested_units(&self, unit: &Unit) -> i64 {
        unit.children
            .iter()
            .map(|child| {
                if units::measured_separately(child.kind) {
                    self.effect_within(&child.bytes)
                } else {
                    self.effect_of_nested_units(child)
                }
            })
            .sum()
    }
}

#[derive(Debug)]
pub struct CognitiveIndex {
    increments: Vec<Increment>,
}

impl CognitiveIndex {
    pub fn new(increments: &[Increment]) -> Self {
        let mut ordered = increments.to_vec();
        ordered.sort_by_key(|increment| increment.position);

        Self {
            increments: ordered,
        }
    }

    pub fn total(&self, unit: &Unit) -> u32 {
        self.within(&unit.bytes)
    }

    pub fn cognitive(&self, unit: &Unit) -> u32 {
        self.within(&unit.bytes)
            .saturating_sub(self.of_nested_units(unit))
    }

    fn within(&self, bytes: &Range<usize>) -> u32 {
        let first = self
            .increments
            .partition_point(|increment| increment.position < bytes.start);
        let last = self
            .increments
            .partition_point(|increment| increment.position < bytes.end);

        self.increments[first..last]
            .iter()
            .map(|increment| increment.amount)
            .sum()
    }

    fn of_nested_units(&self, unit: &Unit) -> u32 {
        unit.children
            .iter()
            .map(|child| {
                if units::measured_separately(child.kind) {
                    self.within(&child.bytes)
                } else {
                    self.of_nested_units(child)
                }
            })
            .sum()
    }
}

fn is_commented(position: usize, comments: &[Range<usize>]) -> bool {
    comments
        .binary_search_by(|comment| {
            if comment.end <= position {
                Ordering::Less
            } else if comment.start > position {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
        .is_ok()
}
