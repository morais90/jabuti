use std::ops::Range;

use tree_sitter::{Node, Parser, Query, QueryCursor, QueryMatch, StreamingIterator, Tree};

use crate::lang::{LangSpec, Queries};
use crate::model::{Decision, DecisionEffect, Span, Unit, UnitKind};

#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    #[error("parser rejected the grammar: {0}")]
    Grammar(#[from] tree_sitter::LanguageError),
    #[error("source could not be parsed")]
    Malformed,
}

#[derive(Debug)]
pub struct Parsed<'source> {
    tree: Tree,
    source: &'source str,
    queries: &'static Queries,
}

pub fn parse<'source>(
    source: &'source str,
    spec: &'static LangSpec,
) -> Result<Parsed<'source>, SyntaxError> {
    let queries = spec.queries();
    let mut parser = Parser::new();
    parser.set_language(&queries.language)?;

    let tree = parser.parse(source, None).ok_or(SyntaxError::Malformed)?;
    if tree.root_node().has_error() {
        return Err(SyntaxError::Malformed);
    }

    Ok(Parsed {
        tree,
        source,
        queries,
    })
}

impl Parsed<'_> {
    pub fn units(&self) -> Unit {
        let mut captured = Vec::new();

        self.for_each_match(&self.queries.units, |matched, query| {
            if let Some(unit) = captured_unit(matched, query, self.source) {
                captured.push(unit);
            }
        });

        nest(captured, self.file_unit())
    }

    pub fn comment_ranges(&self) -> Vec<Range<usize>> {
        let mut ranges = Vec::new();

        self.for_each_match(&self.queries.comments, |matched, _| {
            for capture in matched.captures {
                ranges.push(capture.node.byte_range());
            }
        });

        ranges.sort_by_key(|range| range.start);
        ranges
    }

    pub fn decisions(&self) -> Vec<Decision> {
        let mut decisions = Vec::new();

        self.for_each_match(&self.queries.decisions, |matched, query| {
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

    fn for_each_match(&self, query: &Query, mut visit: impl FnMut(&QueryMatch<'_, '_>, &Query)) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());

        while let Some(matched) = matches.next() {
            visit(matched, query);
        }
    }

    fn file_unit(&self) -> Unit {
        let lines = self.source.split_inclusive('\n').count().max(1);

        Unit {
            name: None,
            kind: UnitKind::File,
            span: Span {
                start_line: 1,
                end_line: u32::try_from(lines).unwrap_or(u32::MAX),
            },
            bytes: 0..self.source.len(),
            children: Vec::new(),
        }
    }
}

fn captured_unit(matched: &QueryMatch<'_, '_>, query: &Query, source: &str) -> Option<Unit> {
    let mut kind = None;
    let mut node = None;
    let mut name = None;

    for capture in matched.captures {
        let label = query.capture_names()[capture.index as usize];
        if let Some(labelled_kind) = kind_of_label(label) {
            kind = Some(labelled_kind);
            node = Some(capture.node);
        } else if label == "name" {
            name = capture
                .node
                .utf8_text(source.as_bytes())
                .ok()
                .map(str::to_owned);
        }
    }

    let node = node?;
    Some(Unit {
        name,
        kind: kind?,
        span: span_of(node),
        bytes: node.byte_range(),
        children: Vec::new(),
    })
}

fn kind_of_label(label: &str) -> Option<UnitKind> {
    let suffix = label.strip_prefix("unit.")?;

    match suffix {
        "module" => Some(UnitKind::Module),
        "type" => Some(UnitKind::Type),
        "function" => Some(UnitKind::Function),
        "closure" => Some(UnitKind::Closure),
        unknown => panic!("query captures @unit.{unknown}, which maps to no unit kind"),
    }
}

fn effect_of_label(label: &str) -> DecisionEffect {
    match label {
        "decision" => DecisionEffect::Branch,
        "decision.discount" => DecisionEffect::Discount,
        unknown => panic!("query captures @{unknown}, which carries no decision effect"),
    }
}

fn span_of(node: Node<'_>) -> Span {
    Span {
        start_line: line_number(node.start_position().row),
        end_line: line_number(node.end_position().row),
    }
}

fn line_number(row: usize) -> u32 {
    u32::try_from(row.saturating_add(1)).unwrap_or(u32::MAX)
}

fn nest(mut captured: Vec<Unit>, mut file: Unit) -> Unit {
    captured.sort_by(|left, right| {
        left.bytes
            .start
            .cmp(&right.bytes.start)
            .then(right.bytes.end.cmp(&left.bytes.end))
    });

    let mut open: Vec<Unit> = Vec::new();

    for unit in captured {
        while let Some(closed) = close_enclosing(&mut open, &unit.bytes) {
            attach(&mut open, &mut file, closed);
        }
        open.push(unit);
    }

    while let Some(remaining) = open.pop() {
        attach(&mut open, &mut file, remaining);
    }

    file
}

fn close_enclosing(open: &mut Vec<Unit>, bytes: &Range<usize>) -> Option<Unit> {
    let innermost = open.last()?;
    if innermost.bytes.start <= bytes.start && bytes.end <= innermost.bytes.end {
        return None;
    }
    open.pop()
}

fn attach(open: &mut [Unit], file: &mut Unit, unit: Unit) {
    match open.last_mut() {
        Some(parent) => parent.children.push(unit),
        None => file.children.push(unit),
    }
}
